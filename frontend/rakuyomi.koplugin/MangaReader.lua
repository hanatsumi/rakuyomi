local ReaderUI = require("apps/reader/readerui")
local UIManager = require("ui/uimanager")
local logger = require("logger")
local _ = require("gettext")

local Backend = require("Backend")
local ErrorDialog = require("ErrorDialog")
local LoadingDialog = require("LoadingDialog")

local MangaReader = {
  on_return_callback = nil,
  on_end_of_book_callback = nil,
  is_showing = false,
}

-- Used to add the "Go back to Rakuyomi" menu item
function MangaReader:addToMainMenu(menu_items)
  menu_items.go_back_to_rakuyomi = {
    text = _("Go back to Rakuyomi..."),
    sorting_hint = "main",
    callback = function()
      self:onReturn()
    end
  }
end

function MangaReader:onReturn()
  self:closeReaderUi(function()
    self.on_return_callback()
  end)
end

function MangaReader:closeReaderUi(done_callback)
  -- Let all event handlers run before closing the ReaderUI, because
  -- some stuff might break if we just remove it ASAP
  UIManager:nextTick(function()
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

    (done_callback or function() end)()
  end)
end

function MangaReader:onEndOfBook()
  logger.info("Got end of book")

  -- ReaderUI.instance:reloadDocument()
  self.on_end_of_book_callback()
end

function MangaReader:onReaderUiCloseWidget()
  self.is_showing = false
end

--- Downloads the given chapter and opens the reader. Must be called from a function wrapped with `Trapper:wrap()`
--- @param chapter Chapter The chapter to be downloaded.
--- @param onReturnCallback fun(): nil
--- @param onEndOfBookCallback fun(): nil
function MangaReader:downloadAndShow(chapter, onReturnCallback, onEndOfBookCallback)
  self.on_return_callback = onReturnCallback
  self.on_end_of_book_callback = onEndOfBookCallback

  local time = require("ui/time")
  local start_time = time.now()
  local response = LoadingDialog:showAndRun(
    "Downloading chapter...",
    function()
      return Backend.downloadChapter(chapter.source_id, chapter.manga_id, chapter.id)
    end
  )

  if response.type == 'ERROR' then
    ErrorDialog:show(response.message)

    return
  end

  -- FIXME mutating here sucks, maybe a callback?
  chapter.downloaded = true

  local manga_path = response.body

  logger.info("Downloaded chapter in ", time.to_ms(time.since(start_time)), "ms")

  if self.is_showing then
    -- if we're showing, just switch the document
    ReaderUI.instance:switchDocument(manga_path)
  else
    -- took this from opds reader
    local Event = require("ui/event")
    UIManager:broadcastEvent(Event:new("SetupShowReader"))

    ReaderUI:showReader(manga_path)
  end

  self.is_showing = true
end

function MangaReader:isShowing()
  return self.is_showing
end

return MangaReader
