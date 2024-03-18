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
    self.ui.menu:registerToMainMenu(MangaReader)
    self.ui:registerPostInitCallback(function()
      self:hookWithPriorityOntoReaderUiEndOfBook()
    end)
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

function Rakuyomi:onEndOfBook()
  if MangaReader:isShowing() then
    MangaReader:onEndOfBook()

    return true
  end
end

-- FIXME maybe move all the `ReaderUI` related logic into `MangaReader`
-- We need to reorder the `ReaderUI` children such that we are the first children,
-- in order to receive events before all other widgets
function Rakuyomi:hookWithPriorityOntoReaderUiEndOfBook()
  assert(self.ui.name == "ReaderUI", "expected to be inside ReaderUI")

  local endOfBookEventListener = WidgetContainer:new({})
  endOfBookEventListener.onEndOfBook = function()
    -- FIXME this makes `Rakuyomi:onEndOfBook()` get called twice if it does not
    -- return true in the first invocation...
    return self:onEndOfBook()
  end

  table.insert(self.ui, 2, endOfBookEventListener)
end

function Rakuyomi:openLibraryView()
  LibraryView:show()
end

return Rakuyomi
