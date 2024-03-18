local ButtonDialog = require("ui/widget/buttondialog")
local Menu = require("ui/widget/menu")
local InfoMessage = require("ui/widget/infomessage")
local UIManager = require("ui/uimanager")
local Screen = require("device").screen
local logger = require("logger")

local Backend = require("Backend")
local ErrorDialog = require("ErrorDialog")
local MangaReader = require("MangaReader")

-- FIXME maybe rename to screen i think ill do it
local ChapterListing = Menu:extend {
  name = "chapter_listing",
  is_enable_shortcut = false,
  is_popout = false,
  title = "Chapter listing",
  align_baselines = true,

  -- the manga we're listing
  manga = nil,
  -- list of chapters
  chapters = nil,
  -- callback to be called when pressing the back button
  on_return_callback = nil,
}

local FA_BOOK_ICON = "\u{F02D}"
local FA_DOWNLOAD_ICON = "\u{F019}"

function ChapterListing:init()
  self.chapters = self.chapters or {}
  self.title_bar_left_icon = "appbar.menu"
  self.onLeftButtonTap = function()
    self:openMenu()
  end

  self.width = Screen:getWidth()
  self.height = Screen:getHeight()
  Menu.init(self)

  -- we need to fill this with *something* in order to Koreader actually recognize
  -- that the back button is active, so yeah
  -- we also need to set this _after_ the `Menu.init` call, because it changes
  -- this value to {}
  self.paths = {
    { callback = self.on_return_callback },
  }
  -- idk might make some gc shenanigans actually work
  self.on_return_callback = nil
  -- we need to do this after updating 
  self:updateItems()
end

-- Updates the menu item contents with the chapter information
function ChapterListing:updateItems()
  self.item_table = self:generateItemTableFromChapters(self.chapters)

  Menu.updateItems(self)
end

function ChapterListing:generateItemTableFromChapters(chapters)
  local item_table = {}

  for _, chapter in ipairs(chapters) do
    local text = ""
    if chapter.volume_num ~= nil then
      -- FIXME we assume there's a chapter number if there's a volume number
      -- might not be true but who knows
      text = text .. "Volume " .. chapter.volume_num .. ", "
    end

    if chapter.chapter_num ~= nil then
      text = text .. "Chapter " .. chapter.chapter_num .. " - "
    end

    text = text .. chapter.title

    if chapter.scanlator ~= nil then
      text = text .. " (" .. chapter.scanlator .. ")"
    end

    -- The text that shows to the right of the menu item
    local mandatory = ""
    if chapter.read then
      mandatory = mandatory .. FA_BOOK_ICON
    end

    if chapter.downloaded then
      mandatory = mandatory .. FA_DOWNLOAD_ICON
    end

    table.insert(item_table, {
      chapter = chapter,
      text = text,
      mandatory = mandatory,
    })
  end

  return item_table
end

function ChapterListing:onReturn()
  local path = table.remove(self.paths)

  self:onClose()
  path.callback()
end

function ChapterListing:show(manga, chapters, onReturnCallback)
  UIManager:show(ChapterListing:new {
    manga = manga,
    chapters = chapters,
    on_return_callback = onReturnCallback,
    covers_fullscreen = true, -- hint for UIManager:_repaint()
  })
end

function ChapterListing:onMenuSelect(item)
  local chapter = item.chapter

  self:openChapterOnReader(chapter)
end

function ChapterListing:openChapterOnReader(chapter)
  local index = self:findChapterIndex(chapter)
  assert(index ~= nil)

  local nextChapter = nil
  if index > 1 then
    -- Chapters are shown in source order, which means that newer chapters come _first_
    nextChapter = self.chapters[index - 1]
  end

  local downloadingMessage = InfoMessage:new{
      text = "Downloading chapter…",
  }

  UIManager:show(downloadingMessage)

  UIManager:tickAfterNext(function()
    local time = require("ui/time")
    local startTime = time.now()
    local outputPath, err = Backend.downloadChapter(chapter.source_id, chapter.manga_id, chapter.id)
    chapter.downloaded = true

    UIManager:close(downloadingMessage)

    if err ~= nil then
      ErrorDialog:show(err)

      return
    end

    logger.info("Downloaded chapter in ", time.to_ms(time.since(startTime)), "ms")
    local onReturnCallback = function()
      self:updateItems()

      UIManager:show(self)
    end

    local onEndOfBookCallback = function()
      Backend.markChapterAsRead(chapter.source_id, chapter.manga_id, chapter.id)
      -- `chapter` here is one of the elements of the `self.chapters` array, so mutating it
      -- here will also change the one inside of the array, and therefore the display will
      -- get updated when we call `updateItems` below
      chapter.read = true

      if nextChapter ~= nil then
        logger.info("opening next chapter", nextChapter)
        self:openChapterOnReader(nextChapter)
      else
        MangaReader:closeReaderUi(function()
          self:updateItems()

          UIManager:show(self)
        end)
      end
    end

    self:onClose()

    MangaReader:show(outputPath, onReturnCallback, onEndOfBookCallback)
  end)
end

function ChapterListing:openMenu()
  local dialog

  local buttons = {
    {
      {
        text = FA_DOWNLOAD_ICON .. " Download all chapters",
        callback = function()
          UIManager:close(dialog)

          self:onDownloadAllChapters()
        end
      }
    }
  }

  dialog = ButtonDialog:new {
    buttons = buttons,
  }

  UIManager:show(dialog)
end

function ChapterListing:onDownloadAllChapters()
  local downloadingMessage = InfoMessage:new{
      text = "Downloading all chapters, this will take a while…",
  }

  UIManager:show(downloadingMessage)

  -- FIXME when the backend functions become actually async we can get rid of this probably
  UIManager:nextTick(function()
    local time = require("ui/time")
    local startTime = time.now()
    local _, err = Backend.downloadAllChapters(self.manga.source_id, self.manga.id)

    if err ~= nil then
      ErrorDialog:show(err)

      return
    end

    local onDownloadFinished = function()
      -- FIXME I don't think mutating the chapter list here is the way to go, but it's quicker
      -- than making another call to list the chapters from the backend...
      -- some possible alternatives:
      -- - return the chapter list from the backend on the `downloadAllChapters` call
      -- - biting the bullet and making the API call
      for _, chapter in ipairs(self.chapters) do
        chapter.downloaded = true
      end

      logger.info("Downloaded all chapters in ", time.to_ms(time.since(startTime)), "ms")

      self:updateItems()
    end

    local updateProgress = nil
    updateProgress = function()
      local downloadProgress, err = Backend.getDownloadAllChaptersProgress(self.manga.source_id, self.manga.id)

      if err ~= nil then
        ErrorDialog:show(err)

        return
      end

      UIManager:close(downloadingMessage)

      local messageText = nil
      if downloadProgress.type == "INITIALIZING" then
        messageText = "Downloading all chapters, this will take a while…"
      elseif downloadProgress.type == "PROGRESSING" then
        messageText = "Downloading all chapters, this will take a while… (" .. downloadProgress.downloaded .. "/" .. downloadProgress.total .. ")"
      elseif downloadProgress.type == "FINISHED" then
        onDownloadFinished()

        return
      elseif downloadProgress.type == "ERRORED" then
        ErrorDialog:show(downloadProgress.message)

        return
      end

      downloadingMessage = InfoMessage:new{
        text = messageText,
      }
      UIManager:show(downloadingMessage)

      UIManager:scheduleIn(1, updateProgress)
    end

    UIManager:scheduleIn(1, updateProgress)
  end)
end

--- Finds the index of the given chapter on the chapter listing.
---@param needle table The chapter being looked for.
---@return number|nil The index of the chapter on the listing, or nil, if it could not be found.
function ChapterListing:findChapterIndex(needle)
  local function isSameChapter(a, b)
    return a.source_id == b.source_id and a.manga_id == b.manga_id and a.id == b.id
  end

  for i, chapter in ipairs(self.chapters) do
    if isSameChapter(chapter, needle) then
      return i
    end
  end

  return nil
end

return ChapterListing
