local Device = require("device")
local InputContainer = require("ui/widget/container/inputcontainer")
local logger = require("logger")
local _ = require("gettext")

local Backend = require("Backend")
local LibraryView = require("LibraryView")
local MangaReader = require("MangaReader")
local Testing = require("testing")

logger.info("Loading Rakuyomi plugin...")
Backend.initialize()

local Rakuyomi = InputContainer:extend({
  name = "rakuyomi"
})

-- We can get initialized from two contexts:
-- - when the `FileManager` is initialized, we're called
-- - when the `ReaderUI` is initialized, we're also called
-- so we should register to the menu accordingly
function Rakuyomi:init()
  self:registerKeyEvents()

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

function Rakuyomi:registerKeyEvents()
  if Device:hasKeyboard() and os.getenv('RAKUYOMI_IS_TESTING') == '1' then
    logger.info("Registering key events for testing")

    self.key_events = {
      DumpVisibleUI = {
        { "Shift", "F8" }
      }
    }
  end
end

function Rakuyomi:onDumpVisibleUI()
  Testing:dumpVisibleUI()

  return true
end

return Rakuyomi
