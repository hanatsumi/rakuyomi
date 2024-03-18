local Menu = require("ui/widget/menu")
local InfoMessage = require("ui/widget/infomessage")
local UIManager = require("ui/uimanager")
local Screen = require("device").screen
local Backend = require("Backend")
local DataStorage = require("datastorage")
local logger = require("logger")

-- FIXME maybe rename to screen i think ill do it
local ChapterListing = Menu:extend {
  is_enable_shortcut = false,
  title = "Chapter listing",
}

function ChapterListing:init()
  self.results = self.results or {}
  self.item_table = self:generateItemTableFromResults(self.results)
  self.width = Screen:getWidth()
  self.height = Screen:getHeight()
  Menu.init(self)
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

function ChapterListing:show(results)
  UIManager:show(ChapterListing:new {
    results = results,
    covers_fullscreen = true, -- hint for UIManager:_repaint()
  })
end

function ChapterListing:onMenuSelect(item)
  local chapter = item.chapter
  local outputFilename = chapter.source_id .. "-" .. chapter.id .. ".cbz"
  local outputPath = DataStorage:getDataDir() .. "/rakuyomi/downloads/" .. outputFilename

  local downloadingMessage = InfoMessage:new{
      text = "Downloading chapter…",
  }

  UIManager:show(downloadingMessage)

  -- FIXME when the backend functions become actually async we can get rid of this probably
  UIManager:nextTick(function()
    Backend.downloadChapter(chapter.source_id, chapter.manga_id, chapter.id, outputPath, function()
      -- took this from opds reader
      local Event = require("ui/event")
      UIManager:broadcastEvent(Event:new("SetupShowReader"))

      self:onClose()
      UIManager:close(downloadingMessage)

      local ReaderUI = require("apps/reader/readerui")
      ReaderUI:showReader(outputPath)
    end)
  end)
end

return ChapterListing
