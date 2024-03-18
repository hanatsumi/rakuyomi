local ConfirmBox = require("ui/widget/confirmbox")
local Menu = require("ui/widget/menu")
local UIManager = require("ui/uimanager")
local Screen = require("device").screen

local Backend = require("Backend")
local ErrorDialog = require("ErrorDialog")
local ChapterListing = require("ChapterListing")

-- FIXME maybe rename to screen i think ill do it
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

-- Updates the menu item contents with the manga information
function MangaSearchResults:updateItems()
  self.item_table = self:generateItemTableFromMangas(self.results)

  Menu.updateItems(self)
end

function MangaSearchResults:generateItemTableFromMangas(mangas)
  local item_table = {}
  for _, manga in ipairs(mangas) do
    table.insert(item_table, {
      manga = manga,
      text = manga.title,
    })
  end

  return item_table
end

function MangaSearchResults:onReturn()
  local path = table.remove(self.paths)

  self:onClose()
  path.callback()
end

function MangaSearchResults:show(results, onReturnCallback)
  UIManager:show(MangaSearchResults:new {
    results = results,
    on_return_callback = onReturnCallback,
    covers_fullscreen = true, -- hint for UIManager:_repaint()
  })
end

function MangaSearchResults:onMenuSelect(item)
  local manga = item.manga

  Backend.listChapters(manga.source_id, manga.id, function(chapter_results, err)
    if err ~= nil then
      ErrorDialog:show(err)

      return
    end

    local onReturnCallback = function()
      UIManager:show(self)
    end

    UIManager:close(self)

    ChapterListing:show(manga, chapter_results, onReturnCallback)
  end)
end

function MangaSearchResults:onMenuHold(item)
  local manga = item.manga
  UIManager:show(ConfirmBox:new {
    text = "Do you want to add \"" .. manga.title .. "\" to your library?",
    ok_text = "Add",
    ok_callback = function()
      Backend.addMangaToLibrary(manga.source_id, manga.id, function(_, err)
        if err ~= nil then
          ErrorDialog:show(err)

          return
        end

        -- FIXME should we do something here?
      end)
    end
  })
end

return MangaSearchResults
