local ReaderUI = require("apps/reader/readerui")
local UIManager = require("ui/uimanager")
local _ = require("gettext")

local MangaReader = {
  on_return_callback = nil,
}

-- Used to add the "Go back to Rakuyomi" menu item
function MangaReader:addToMainMenu(menu_items)
  menu_items.go_back_to_rakuyomi = {
    text = _("Go back to Rakuyomi..."),
    sorting_hint = "main",
    callback = function()
      local FileManager = require("apps/filemanager/filemanager")

      -- we **have** to reopen the `FileManager`, because
      -- apparently this is the only way to get out of the `ReaderUI` without shit
      -- completely breaking (koreader really does not like when there's no `ReaderUI`
      -- nor `FileManager`)
      ReaderUI.instance:onClose()
      if FileManager.instance then
        FileManager.instance:reinit()
      else
        FileManager:showFiles()
      end

      self.on_return_callback()
    end
  }
end

function MangaReader:show(manga_path, onReturnCallback)
  self.on_return_callback = onReturnCallback

  -- took this from opds reader
  local Event = require("ui/event")
  UIManager:broadcastEvent(Event:new("SetupShowReader"))

  ReaderUI:showReader(manga_path)
end

return MangaReader
