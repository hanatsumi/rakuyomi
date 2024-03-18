local Menu = require("ui/widget/menu")
local UIManager = require("ui/uimanager")
local Screen = require("device").screen
local Backend = require("Backend")
local ChapterListing = require("ChapterListing")

-- FIXME maybe rename to screen i think ill do it
local MangaSearchResults = Menu:extend {
  name = "manga_search_results",
  is_enable_shortcut = false,
  is_popout = false,
  title = "Search results...",
}

function MangaSearchResults:init()
  self.results = self.results or {}
  self.item_table = self:generateItemTableFromResults(self.results)
  self.width = Screen:getWidth()
  self.height = Screen:getHeight()
  Menu.init(self)
end

function MangaSearchResults:generateItemTableFromResults(results)
  local item_table = {}
  for _, result in ipairs(results) do
    table.insert(item_table, {
      manga = result,
      text = result.title,
    })
  end

  return item_table
end

function MangaSearchResults:show(results)
  UIManager:show(MangaSearchResults:new {
    results = results,
    covers_fullscreen = true, -- hint for UIManager:_repaint()
  })
end

function MangaSearchResults:onMenuSelect(item)
  local manga = item.manga

  Backend.listChapters(manga.source_id, manga.id, function(chapter_results)
    local onReturnCallback = function()
      UIManager:show(self)
    end

    UIManager:close(self)

    ChapterListing:show(chapter_results, onReturnCallback)
  end)
end

return MangaSearchResults
