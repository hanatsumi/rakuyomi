local BD = require("ui/bidi")
local ButtonDialog = require("ui/widget/buttondialog")
local Menu = require("ui/widget/menu")
local InfoMessage = require("ui/widget/infomessage")
local InputDialog = require("ui/widget/inputdialog")
local UIManager = require("ui/uimanager")
local Trapper = require("ui/trapper")
local Screen = require("device").screen
local logger = require("logger")
local LoadingDialog = require("LoadingDialog")
local util = require("util")

local Backend = require("Backend")
local DownloadChapter = require("jobs/DownloadChapter")
local DownloadUnreadChapters = require("jobs/DownloadUnreadChapters")
local DownloadUnreadChaptersJobDialog = require("DownloadUnreadChaptersJobDialog")
local Icons = require("Icons")
local ErrorDialog = require("ErrorDialog")
local MangaReader = require("MangaReader")

local findNextChapter = require("chapters/findNextChapter")

--- @class ChapterListing : { [any]: any }
--- @field manga Manga
--- @field chapters Chapter[]
--- @field chapter_sorting_mode ChapterSortingMode
local ChapterListing = Menu:extend {
  name = "chapter_listing",
  is_enable_shortcut = false,
  is_popout = false,
  title = "Chapter listing",
  align_baselines = true,

  -- the manga we're listing
  manga = nil,
  -- list of chapters
  chapters = {},
  chapter_sorting_mode = nil,
  -- callback to be called when pressing the back button
  on_return_callback = nil,
}

function ChapterListing:init()
  self.title_bar_left_icon = "appbar.menu"
  self.onLeftButtonTap = function()
    self:openMenu()
  end

  self.width = Screen:getWidth()
  self.height = Screen:getHeight()

  -- FIXME `Menu` calls `updateItems()` during init, but we haven't fetched any items yet, as
  -- we do it in `updateChapterList`. Not sure if there's any downside to it, but here's a notice.
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
  self:updateChapterList()
end

--- Fetches the cached chapter list from the backend and updates the menu items.
function ChapterListing:updateChapterList()
  local response = Backend.listCachedChapters(self.manga.source.id, self.manga.id)

  if response.type == 'ERROR' then
    ErrorDialog:show(response.message)

    return
  end

  local chapter_results = response.body
  self.chapters = chapter_results

  self:updateItems()
end

--- Updates the menu item contents with the chapter information
--- @private
function ChapterListing:updateItems()
  if #self.chapters > 0 then
    self.item_table = self:generateItemTableFromChapters(self.chapters)
    self.multilines_show_more_text = false
    self.items_per_page = nil
  else
    self.item_table = self:generateEmptyViewItemTable()
    self.multilines_show_more_text = true
    self.items_per_page = 1
  end

  Menu.updateItems(self)
end

--- @private
function ChapterListing:generateEmptyViewItemTable()
  return {
    {
      text = "No chapters found. Try swiping down to refresh the chapter list.",
      dim = true,
      select_enabled = false,
    }
  }
end

--- Compares whether chapter `a` is before `b`. Expects the `index` of the chapter in the
--- chapter array to be present inside the chapter object.
---
--- @param a Chapter|{ index: number }
--- @param b Chapter|{ index: number }
--- @return boolean `true` if chapter `a` should be displayed before `b`, otherwise `false`.
local function isBeforeChapter(a, b)
  if a.volume_num ~= nil and b.volume_num ~= nil and a.volume_num ~= b.volume_num then
    return a.volume_num < b.volume_num
  end

  if a.chapter_num ~= nil and b.chapter_num ~= nil and a.chapter_num ~= b.chapter_num then
    return a.chapter_num < b.chapter_num
  end

  -- This is _very_ flaky, but we assume that source order is _always_ from newer chapters -> older chapters.
  -- Unfortunately we need to make some kind of assumptions here to handle edgecases (e.g. chapters without a chapter number)
  return a.index > b.index
end

--- @private
function ChapterListing:generateItemTableFromChapters(chapters)
  --- @type table
  --- @diagnostic disable-next-line: assign-type-mismatch
  local sorted_chapters_with_index = util.tableDeepCopy(chapters)
  for index, chapter in ipairs(sorted_chapters_with_index) do
    chapter.index = index
  end

  if self.chapter_sorting_mode == 'chapter_ascending' then
    table.sort(sorted_chapters_with_index, isBeforeChapter)
  else
    table.sort(sorted_chapters_with_index, function(a, b) return not isBeforeChapter(a, b) end)
  end

  local item_table = {}

  for _, chapter in ipairs(sorted_chapters_with_index) do
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
      mandatory = mandatory .. Icons.FA_BOOK
    end

    if chapter.downloaded then
      mandatory = mandatory .. Icons.FA_DOWNLOAD
    end

    table.insert(item_table, {
      chapter = chapter,
      text = text,
      mandatory = mandatory,
    })
  end

  return item_table
end

--- @private
function ChapterListing:onReturn()
  local path = table.remove(self.paths)

  self:onClose()
  path.callback()
end

--- Shows the chapter list for a given manga. Must be called from a function wrapped with `Trapper:wrap()`.
---
--- @param manga Manga
--- @param onReturnCallback fun(): nil
--- @param accept_cached_results? boolean If set, failing to refresh the list of chapters from the source
--- will not show an error. Defaults to false.
function ChapterListing:fetchAndShow(manga, onReturnCallback, accept_cached_results)
  accept_cached_results = accept_cached_results or false

  local refresh_chapters_response = LoadingDialog:showAndRun(
    "Refreshing chapters...",
    function()
      return Backend.refreshChapters(manga.source.id, manga.id)
    end
  )

  if not accept_cached_results and refresh_chapters_response.type == 'ERROR' then
    ErrorDialog:show(refresh_chapters_response.message)

    return
  end

  local response = Backend.getSettings()

  if response.type == 'ERROR' then
    ErrorDialog:show(response.message)

    return
  end

  local settings = response.body

  UIManager:show(ChapterListing:new {
    manga = manga,
    chapter_sorting_mode = settings.chapter_sorting_mode,
    on_return_callback = onReturnCallback,
    covers_fullscreen = true, -- hint for UIManager:_repaint()
  })
end

--- @private
function ChapterListing:onMenuChoice(item)
  local chapter = item.chapter

  self:openChapterOnReader(chapter)
end

--- @private
function ChapterListing:onSwipe(arg, ges_ev)
  local direction = BD.flipDirectionIfMirroredUILayout(ges_ev.direction)
  if direction == "south" then
    self:refreshChapters()

    return
  end

  Menu.onSwipe(self, arg, ges_ev)
end

--- @private
function ChapterListing:refreshChapters()
  Trapper:wrap(function()
    local refresh_chapters_response = LoadingDialog:showAndRun(
      "Refreshing chapters...",
      function()
        return Backend.refreshChapters(self.manga.source.id, self.manga.id)
      end
    )

    if refresh_chapters_response.type == 'ERROR' then
      ErrorDialog:show(refresh_chapters_response.message)

      return
    end

    self:updateChapterList()
  end)
end

--- @private
function ChapterListing:openChapterOnReader(chapter, downloadJob)
  Trapper:wrap(function()
    local nextChapter = findNextChapter(self.chapters, chapter)
    local nextChapterDownloadJob = nil

    local onDownloadJobFinished = function()
      -- FIXME Mutating here _still_ sucks, we gotta think of a better way.
      chapter.downloaded = true

      if nextChapter ~= nil then
        nextChapterDownloadJob = DownloadChapter:new(nextChapter.source_id, nextChapter.manga_id, nextChapter.id)
      end
    end

    local onReturnCallback = function()
      self:updateItems()

      UIManager:show(self)
    end

    local onEndOfBookCallback = function()
      Backend.markChapterAsRead(chapter.source_id, chapter.manga_id, chapter.id)

      self:updateChapterList()

      if nextChapter ~= nil then
        logger.info("opening next chapter", nextChapter)
        self:openChapterOnReader(nextChapter, nextChapterDownloadJob)
      else
        MangaReader:closeReaderUi(function()
          UIManager:show(self)
        end)
      end
    end

    MangaReader:downloadAndShow({
      download_job = downloadJob or DownloadChapter:new(chapter.source_id, chapter.manga_id, chapter.id),
      on_download_job_finished = onDownloadJobFinished,
      on_end_of_book_callback = onEndOfBookCallback,
      on_return_callback = onReturnCallback,
    })

    self:onClose()
  end)
end

--- @private
function ChapterListing:openMenu()
  local dialog

  local buttons = {
    {
      {
        text = Icons.FA_DOWNLOAD .. " Download unread chapters…",
        callback = function()
          UIManager:close(dialog)

          self:onDownloadUnreadChapters()
        end
      }
    }
  }

  dialog = ButtonDialog:new {
    buttons = buttons,
  }

  UIManager:show(dialog)
end

function ChapterListing:onDownloadUnreadChapters()
  local input_dialog
  input_dialog = InputDialog:new {
    title = "Download unread chapters...",
    input_type = "number",
    input_hint = "Amount of unread chapters (default: all)",
    description = "Specify the amount of unread chapters to download, or leave empty to download all of them.",
    buttons = {
      {
        {
          text = "Cancel",
          id = "close",
          callback = function()
            UIManager:close(input_dialog)
          end,
        },
        {
          text = "Download",
          is_enter_default = true,
          callback = function()
            UIManager:close(input_dialog)

            local amount = nil
            if input_dialog:getInputText() ~= '' then
              amount = tonumber(input_dialog:getInputText())

              if amount == nil then
                ErrorDialog:show('Invalid amount of chapters!')

                return
              end
            end

            local job = DownloadUnreadChapters:new(self.manga.source.id, self.manga.id, amount)
            local dialog = DownloadUnreadChaptersJobDialog:new({
              show_parent = self,
              job = job,
              dismiss_callback = function()
                self:updateChapterList()
              end
            })

            dialog:show()
          end,
        },
      }
    }
  }

  UIManager:show(input_dialog)
end

function ChapterListing:onDownloadAllChapters()
  local downloadingMessage = InfoMessage:new {
    text = "Downloading all chapters, this will take a while…",
  }

  UIManager:show(downloadingMessage)

  -- FIXME when the backend functions become actually async we can get rid of this probably
  UIManager:nextTick(function()
    local time = require("ui/time")
    local startTime = time.now()
    local response = Backend.downloadAllChapters(self.manga.source.id, self.manga.id)

    if response.type == 'ERROR' then
      ErrorDialog:show(response.message)

      return
    end

    local onDownloadFinished = function()
      -- FIXME I don't think mutating the chapter list here is the way to go, but it's quicker
      -- than making another call to list the chapters from the backend...
      -- this also behaves wrong when the download fails but manages to download some chapters.
      -- some possible alternatives:
      -- - return the chapter list from the backend on the `downloadAllChapters` call
      -- - biting the bullet and making the API call
      for _, chapter in ipairs(self.chapters) do
        chapter.downloaded = true
      end

      logger.info("Downloaded all chapters in ", time.to_ms(time.since(startTime)), "ms")

      self:updateItems()
    end

    local updateProgress = function() end

    local cancellationRequested = false
    local onCancellationRequested = function()
      local response = Backend.cancelDownloadAllChapters(self.manga.source.id, self.manga.id)
      -- FIXME is it ok to assume there are no errors here?
      assert(response.type == 'SUCCESS')

      cancellationRequested = true

      updateProgress()
    end

    local onCancelled = function()
      local cancelledMessage = InfoMessage:new {
        text = "Cancelled.",
      }

      UIManager:show(cancelledMessage)
    end

    updateProgress = function()
      -- Remove any scheduled `updateProgress` calls, because we do not want this to be
      -- called again if not scheduled by ourselves. This may happen when `updateProgress` is called
      -- from another place that's not from the scheduler (eg. the `onCancellationRequested` handler),
      -- which could result in an additional `updateProgress` call that was already scheduled previously,
      -- even if we do not schedule it at the end of the method.
      UIManager:unschedule(updateProgress)
      UIManager:close(downloadingMessage)

      local response = Backend.getDownloadAllChaptersProgress(self.manga.source.id, self.manga.id)
      if response.type == 'ERROR' then
        ErrorDialog:show(response.message)

        return
      end

      local downloadProgress = response.body

      local messageText = nil
      local isCancellable = false
      if downloadProgress.type == "INITIALIZING" then
        messageText = "Downloading all chapters, this will take a while…"
      elseif downloadProgress.type == "FINISHED" then
        onDownloadFinished()

        return
      elseif downloadProgress.type == "CANCELLED" then
        onCancelled()

        return
      elseif cancellationRequested then
        messageText = "Waiting for download to be cancelled…"
      elseif downloadProgress.type == "PROGRESSING" then
        messageText = "Downloading all chapters, this will take a while… (" ..
            downloadProgress.downloaded .. "/" .. downloadProgress.total .. ")." ..
            "\n\n" ..
            "Tap outside this message to cancel."

        isCancellable = true
      else
        logger.err("unexpected download progress message", downloadProgress)

        error("unexpected download progress message")
      end

      downloadingMessage = InfoMessage:new {
        text = messageText,
        dismissable = isCancellable,
      }

      -- Override the default `onTapClose`/`onAnyKeyPressed` actions
      if isCancellable then
        local originalOnTapClose = downloadingMessage.onTapClose
        downloadingMessage.onTapClose = function(messageSelf)
          onCancellationRequested()

          originalOnTapClose(messageSelf)
        end

        local originalOnAnyKeyPressed = downloadingMessage.onAnyKeyPressed
        downloadingMessage.onAnyKeyPressed = function(messageSelf)
          onCancellationRequested()

          originalOnAnyKeyPressed(messageSelf)
        end
      end
      UIManager:show(downloadingMessage)

      UIManager:scheduleIn(1, updateProgress)
    end

    UIManager:scheduleIn(1, updateProgress)
  end)
end

return ChapterListing
