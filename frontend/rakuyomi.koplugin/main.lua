local WidgetContainer = require("ui/widget/container/widgetcontainer")
local InputDialog = require("ui/widget/inputdialog")
local UIManager = require("ui/uimanager")
local logger = require("logger")
local _ = require("gettext")

local Backend = require("Backend")
local MangaReader = require("MangaReader")
local MangaSearchResults = require("MangaSearchResults")

logger.info("Loading Rakuyomi plugin...")
Backend.initialize()

local Rakuyomi = WidgetContainer:extend({
  name = "rakuyomi"
})

-- We can get initialized from two contexts:
-- - when the `FileManager` is initialized, we're called 
-- - when the `ReaderUI` is initialized, we're also called
-- so we should register to the menu accordingly
function Rakuyomi:init()
  if self.ui.name == "ReaderUI" then
    self.ui.menu:registerToMainMenu(MangaReader)
  else
    self.ui.menu:registerToMainMenu(self)
  end
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
  Backend.searchMangas(search_text, function(results)
    UIManager:show(MangaSearchResults:new {
      results = results,
      covers_fullscreen = true, -- hint for UIManager:_repaint()
    })
  end)
end

return Rakuyomi
