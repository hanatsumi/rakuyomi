local WidgetContainer = require("ui/widget/container/widgetcontainer")
local logger = require("logger")
local _ = require("gettext")

local Backend = require("Backend")
local LibraryView = require("LibraryView")
local MangaReader = require("MangaReader")

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
    MangaReader:initializeFromReaderUI(self.ui)
  else
    self.ui.menu:registerToMainMenu(self)
  end
end

function Rakuyomi:addToMainMenu(menu_items)
  menu_items.rakuyomi = {
    text = _("Rakuyomi"),
    sorting_hint = "search",
    callback = function()
      self:openLibraryView()
    end
  }
end

function Rakuyomi:openLibraryView()
  LibraryView:fetchAndShow()
end

return Rakuyomi
