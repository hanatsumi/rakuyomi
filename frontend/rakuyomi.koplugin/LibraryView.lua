-- FIXME make class names have _some_ kind of logic
local Menu = require("ui/widget/menu")
local InfoMessage = require("ui/widget/infomessage")
local InputDialog = require("ui/widget/inputdialog")
local UIManager = require("ui/uimanager")
local Screen = require("device").screen
local logger = require("logger")
local _ = require("gettext")

local Backend = require("Backend")
local ChapterListing = require("ChapterListing")
local MangaSearchResults = require("MangaSearchResults")

local LibraryView = Menu:extend {
  name = "library_view",
  is_enable_shortcut = false,
  is_popout = false,
  title = "Library",

  -- list of mangas in your library
  mangas = nil,
}

function LibraryView:init()
  self.mangas = self.mangas or {}
  self.title_bar_left_icon = "appbar.search"
  self.onLeftButtonTap = function()
    self:openSearchMangasDialog()
  end
  self.item_table = self:generateItemTableFromMangas(self.mangas)
  self.width = Screen:getWidth()
  self.height = Screen:getHeight()
  Menu.init(self)
end

function LibraryView:generateItemTableFromMangas(mangas)
  local item_table = {}
  for _, manga in ipairs(mangas) do
    table.insert(item_table, {
      manga = manga,
      text = manga.title,
    })
  end

  return item_table
end

function LibraryView:show()
  Backend.getMangasInLibrary(function(mangas)
    UIManager:show(LibraryView:new {
      mangas = mangas,
      covers_fullscreen = true, -- hint for UIManager:_repaint()
    })
  end)
end

function LibraryView:onMenuSelect(item)
  local manga = item.manga

  Backend.listChapters(manga.source_id, manga.id, function(chapter_results)
    local onReturnCallback = function()
      UIManager:show(self)
    end

    UIManager:close(self)

    ChapterListing:show(chapter_results, onReturnCallback)
  end)
end

function LibraryView:openSearchMangasDialog()
  local dialog
  dialog = InputDialog:new {
    title = _("Manga search..."),
    input_hint = _("Houseki no Kuni"),
    description = _("Type the manga name to search for"),
    buttons = {
      {
        {
          text = _("Cancel"),
          id = "close",
          callback = function()
            UIManager:close(dialog)
          end,
        },
        {
          text = _("Search"),
          is_enter_default = true,
          callback = function()
            UIManager:close(dialog)

            self:searchMangas(dialog:getInputText())
          end,
        },
      }
    }
  }

  UIManager:show(dialog)
  dialog:onShowKeyboard()
end

function LibraryView:searchMangas(search_text)
  Backend.searchMangas(search_text, function(results)
    UIManager:show(MangaSearchResults:new {
      results = results,
      covers_fullscreen = true, -- hint for UIManager:_repaint()
    })
  end)
end


return LibraryView
