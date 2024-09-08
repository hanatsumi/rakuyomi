local ConfirmBox = require("ui/widget/confirmbox")
local Menu = require("ui/widget/menu")
local UIManager = require("ui/uimanager")
local Screen = require("device").screen
local Trapper = require("ui/trapper")

local Backend = require("Backend")
local ErrorDialog = require("ErrorDialog")
local LoadingDialog = require("LoadingDialog")
local ChapterListing = require("ChapterListing")

--- @class MangaSearchResults: { [any]: any }
--- @field results SourceMangaSearchResults[]
--- @field on_return_callback fun(): nil
local MangaSearchResults = Menu:extend {
  name = "manga_search_results",
  is_enable_shortcut = false,
  is_popout = false,
  title = "Search results...",

  -- list of mangas
  results = nil,
  -- callback to be called when pressing the back button
  on_return_callback = nil,
}

function MangaSearchResults:init()
  self.results = self.results or {}
  self.width = Screen:getWidth()
  self.height = Screen:getHeight()
  Menu.init(self)

  -- see `ChapterListing` for an explanation on this
  -- FIXME we could refactor this into a single class
  self.paths = {
    { callback = self.on_return_callback },
  }
  self.on_return_callback = nil

  self:updateItems()
end

--- Updates the menu item contents with the manga information
--- @private
function MangaSearchResults:updateItems()
  self.item_table = self:generateItemTableFromSearchResults(self.results)

  Menu.updateItems(self)
end

--- Generates the item table for displaying the search results.
--- @private
--- @param results SourceMangaSearchResults[]
--- @return table
function MangaSearchResults:generateItemTableFromSearchResults(results)
  local item_table = {}
  for _, result in ipairs(results) do
    local source_information = result.source_information

    for _, manga in ipairs(result.mangas) do
      table.insert(item_table, {
        manga = manga,
        text = manga.title .. " (" .. source_information.name .. ")",
      })
    end
  end

  return item_table
end

--- @private
function MangaSearchResults:onReturn()
  local path = table.remove(self.paths)

  self:onClose()
  path.callback()
end

--- Searches for mangas and shows the results.
--- @param search_text string The text to be searched for.
--- @param onReturnCallback any
function MangaSearchResults:searchAndShow(search_text, onReturnCallback)
  local response = LoadingDialog:showAndRun(
    "Searching for \"" .. search_text .. "\"",
    function() return Backend.searchMangas(search_text) end
  )

  if response.type == 'ERROR' then
    ErrorDialog:show(response.message)

    return
  end

  local results = response.body

  UIManager:show(MangaSearchResults:new {
    results = results,
    on_return_callback = onReturnCallback,
    covers_fullscreen = true, -- hint for UIManager:_repaint()
  })
end

--- @private
function MangaSearchResults:onMenuSelect(item)
  Trapper:wrap(function()
    --- @type Manga
    local manga = item.manga

    local onReturnCallback = function()
      UIManager:show(self)
    end

    ChapterListing:fetchAndShow(manga, onReturnCallback)

    UIManager:close(self)
  end)
end

--- @private
function MangaSearchResults:onMenuHold(item)
  --- @type Manga
  local manga = item.manga
  UIManager:show(ConfirmBox:new {
    text = "Do you want to add \"" .. manga.title .. "\" to your library?",
    ok_text = "Add",
    ok_callback = function()
      local _, err = Backend.addMangaToLibrary(manga.source.id, manga.id)

      if err ~= nil then
        ErrorDialog:show(err)

        return
      end
    end
  })
end

return MangaSearchResults
