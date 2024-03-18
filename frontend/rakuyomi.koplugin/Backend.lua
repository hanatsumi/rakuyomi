local DataStorage = require("datastorage")
local logger = require("logger")

local backend_library = require("backend")

local Backend = {}

function Backend.initialize()
  local sourcesPath = DataStorage:getDataDir() .. "/rakuyomi/sources"
  backend_library.initialize(sourcesPath)
end

function Backend.searchMangas(search_text)
  return backend_library.search_mangas(search_text)
end

function Backend.listChapters(source_id, manga_id)
  return backend_library.list_chapters(source_id, manga_id)
end

function Backend.downloadChapter(source_id, manga_id, chapter_id, output_path)
  backend_library.download_chapter(source_id, manga_id, chapter_id, output_path)
end

function Backend.cleanup()
  logger.info("cleaning up")
end

-- we can't really rely upon Koreader informing us
-- it has terminated in every case, so use the
-- garbage collector to clean up stuff
if _VERSION == "Lua 5.1" then
  logger.info("setting up __gc proxy")
  local proxy = newproxy(true)
  local proxyMeta = getmetatable(proxy)

  proxyMeta.__gc = function()
    Backend.cleanup()
  end

  rawset(Backend, '__proxy', proxy)
else
  setmetatable(Backend, {
    __gc = function()
      Backend.cleanup()
    end
  })
end

return Backend
