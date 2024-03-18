-- FIXME make class names have _some_ kind of logic
local Menu = require("ui/widget/menu")
local InfoMessage = require("ui/widget/infomessage")
local InputDialog = require("ui/widget/inputdialog")
local UIManager = require("ui/uimanager")
local Screen = require("device").screen
local logger = require("logger")
local _ = require("gettext")

local Backend = require("Backend")
local ErrorDialog = require("ErrorDialog")
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
  local response = Backend.getMangasInLibrary()
  if response.type == 'ERROR' then
    ErrorDialog:show(response.message)

    return
  end

  local mangas = response.body

  UIManager:show(LibraryView:new {
    mangas = mangas,
    covers_fullscreen = true, -- hint for UIManager:_repaint()
  })
end

function LibraryView:onMenuSelect(item)
  local manga = item.manga

  local response = Backend.listChapters(manga.source_id, manga.id)
  if response.type == 'ERROR' then
    ErrorDialog:show(response.message)

    return
  end

  local chapter_results = response.body

  local onReturnCallback = function()
    self:show()
  end

  self:onClose(self)

  ChapterListing:show(manga, chapter_results, onReturnCallback)
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
  local response = Backend.searchMangas(search_text)
  if response.type == 'ERROR' then
    ErrorDialog:show(response.message)

    return
  end

  local results = response.body

  local onReturnCallback = function()
    self:show()
  end

  self:onClose()

  MangaSearchResults:show(results, onReturnCallback)
end

return LibraryView
