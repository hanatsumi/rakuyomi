local WidgetContainer = require("ui/widget/container/widgetcontainer")
local InfoMessage = require("ui/widget/infomessage")
local InputDialog = require("ui/widget/inputdialog")
local UIManager = require("ui/uimanager")
local logger = require("logger")
local _ = require("gettext")
local backend = require("backend")
local DataStorage = require("datastorage")
local MangaSearchResults = require("MangaSearchResults")

logger.info("Loading Rakuyomi plugin...")

local Rakuyomi = WidgetContainer:extend({
  name = "rakuyomi"
})

function Rakuyomi:init()
  local sourcesPath = DataStorage:getDataDir() .. "/rakuyomi/sources"
  backend.initialize(sourcesPath)
  self.ui.menu:registerToMainMenu(self)
end

function Rakuyomi:addToMainMenu(menu_items)
  menu_items.search_mangas_with_rakuyomi = {
    text = _("Search mangas with Rakuyomi..."),
    sorting_hint = "search",
    callback = function()
      self:openSearchMangasDialog()
    end
  }
end

function Rakuyomi:openSearchMangasDialog()
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

function Rakuyomi:searchMangas(search_text)
  local results = backend.search_mangas(search_text)

  UIManager:show(MangaSearchResults:new {
    results = results,
    covers_fullscreen = true, -- hint for UIManager:_repaint()
  })
end

return Rakuyomi
