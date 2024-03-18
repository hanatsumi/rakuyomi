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

  -- list of chapters
  results = nil,
  -- callback to be called when pressing the back button
  on_return_callback = nil,
}

function ChapterListing:init()
  self.results = self.results or {}
  self.item_table = self:generateItemTableFromResults(self.results)
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

function ChapterListing:generateItemTableFromResults(results)
  local item_table = {}
  -- FIXME result -> chapter? also result -> manga in the manga screen
  for _, result in ipairs(results) do
    local text = ""
    if result.volume_num ~= nil then
      -- FIXME we assume there's a chapter number if there's a volume number
      -- might not be true but who knows
      text = text .. "Volume " .. result.volume_num .. ", "
    end

    if result.chapter_num ~= nil then
      text = text .. "Chapter " .. result.chapter_num .. " - "
    end

    text = text .. result.title

    if result.scanlator ~= nil then
      text = text .. " (" .. result.scanlator .. ")"
    end

    table.insert(item_table, {
      chapter = result,
      text = text,
    })
  end

  return item_table
end

function ChapterListing:onReturn()
  local path = table.remove(self.paths)

  self:onClose()
  path.callback()
end

function ChapterListing:show(results, onReturnCallback)
  UIManager:show(ChapterListing:new {
    results = results,
    on_return_callback = onReturnCallback,
    covers_fullscreen = true, -- hint for UIManager:_repaint()
  })
end

function ChapterListing:onMenuSelect(item)
  local chapter = item.chapter

  local downloadingMessage = InfoMessage:new{
      text = "Downloading chapter…",
  }

  UIManager:show(downloadingMessage)

  -- FIXME when the backend functions become actually async we can get rid of this probably
  UIManager:nextTick(function()
    local time = require("ui/time")
    local startTime = time.now()
    Backend.downloadChapter(chapter.source_id, chapter.manga_id, chapter.id, function(outputPath, err)
      UIManager:close(downloadingMessage)

      if err ~= nil then
        ErrorDialog:show(err)

        return
      end

      logger.info("Downloaded chapter in ", time.to_ms(time.since(startTime)), "ms")
      local onReturnCallback = function()
        UIManager:show(self)
      end

      self:onClose()

      MangaReader:show(outputPath, onReturnCallback)
    end)
  end)
end

return ChapterListing
