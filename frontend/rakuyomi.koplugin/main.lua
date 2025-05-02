local DocumentRegistry = require("document/documentregistry")
local InputContainer = require("ui/widget/container/inputcontainer")
local logger = require("logger")
local _ = require("gettext")
local OfflineAlertDialog = require("OfflineAlertDialog")

local Backend = require("Backend")
local CbzDocument = require("extensions/CbzDocument")
local ErrorDialog = require("ErrorDialog")
local LibraryView = require("LibraryView")
local MangaReader = require("MangaReader")
local Testing = require("testing")

logger.info("Loading Rakuyomi plugin...")
local backendInitialized, logs = Backend.initialize()

local Rakuyomi = InputContainer:extend({
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

  CbzDocument:register(DocumentRegistry)

  Testing:init()
  Testing:emitEvent('initialized')
end

function Rakuyomi:addToMainMenu(menu_items)
  menu_items.rakuyomi = {
    text = _("Rakuyomi"),
    sorting_hint = "search",
    callback = function()
      if not backendInitialized then
        self:showErrorDialog()

        return
      end

      self:openLibraryView()
    end
  }
end

function Rakuyomi:showErrorDialog()
  ErrorDialog:show(
    "Oops! Rakuyomi encountered an issue while starting up!\n" ..
    "Here are some messages that might help identify the problem:\n\n" ..
    logs
  )
end

function Rakuyomi:openLibraryView()
  LibraryView:fetchAndShow()
  OfflineAlertDialog:showIfOffline()
end

return Rakuyomi
