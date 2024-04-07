-- FIXME make class names have _some_ kind of logic
local Menu = require("ui/widget/menu")
local InfoMessage = require("ui/widget/infomessage")
local InputDialog = require("ui/widget/inputdialog")
local UIManager = require("ui/uimanager")
local Screen = require("device").screen
local Trapper = require("ui/trapper")
local logger = require("logger")
local _ = require("gettext")
local LoadingDialog = require("LoadingDialog")
local Icons = require("Icons")
local ButtonDialog = require("ui/widget/buttondialog")
local InstalledSourcesListing = require("InstalledSourcesListing")

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
  self.title_bar_left_icon = "appbar.menu"
  self.onLeftButtonTap = function()
    self:openMenu()
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
  Trapper:wrap(function()
    local manga = item.manga

    local refresh_chapters_response = LoadingDialog:showAndRun(
      "Refreshing chapters...",
      function()
        return Backend.refreshChapters(manga.source_id, manga.id)
      end
    )

    if refresh_chapters_response.type == 'ERROR' then
      -- Specifically from the LibraryView, we should be able to handle
      -- failures from the refresh chapters response. As we can read chapters
      -- that were cached into the database/downloaded into the storage,
      -- just log here and move on (we could also somehow inform the user,
      -- but I don't think this is needed).

      logger.info("Failed to refresh chapters for manga", manga)
    end

    local response = Backend.listCachedChapters(manga.source_id, manga.id)
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
  end)
end

function LibraryView:openMenu()
  local dialog

  local buttons = {
    {
      {
        text = Icons.FA_MAGNIFYING_GLASS .. " Search for mangas",
        callback = function()
          UIManager:close(dialog)

          self:openSearchMangasDialog()
        end
      },
    },
    {
      {
        text = Icons.FA_PLUG .. " Manage sources",
        callback = function()
          UIManager:close(dialog)

          self:openInstalledSourcesListing()
        end
      },
    }
  }

  dialog = ButtonDialog:new {
    buttons = buttons,
  }

  UIManager:show(dialog)
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
  Trapper:wrap(function()
    local response = LoadingDialog:showAndRun(
      "Searching for \"" .. search_text .. "\"",
      function() return Backend.searchMangas(search_text) end
    )

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
  end)
end

function LibraryView:openInstalledSourcesListing()
  local response = Backend.listInstalledSources()
  if response.type == 'ERROR' then
    ErrorDialog:show(response.message)

    return
  end

  local installed_sources = response.body

  local onReturnCallback = function()
    self:show()
  end

  self:onClose()

  InstalledSourcesListing:show(installed_sources, onReturnCallback)
end

return LibraryView
