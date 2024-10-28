local DataStorage = require("datastorage")
local logger = require("logger")
local C = require("ffi").C
local ffi = require("ffi")
local ffiutil = require("ffi/util")
local rapidjson = require("rapidjson")
local util = require("util")

local SERVER_STARTUP_TIMEOUT_SECONDS = tonumber(os.getenv('RAKUYOMI_SERVER_STARTUP_TIMEOUT') or 5)
local SERVER_COMMAND_WORKING_DIRECTORY = os.getenv('RAKUYOMI_SERVER_WORKING_DIRECTORY')
local SERVER_COMMAND_OVERRIDE = os.getenv('RAKUYOMI_SERVER_COMMAND_OVERRIDE')

local Backend = {}

local function replaceRapidJsonNullWithNilRecursively(maybeTable)
  if type(maybeTable) ~= "table" then
    return maybeTable
  end

  local t = maybeTable

  for key, value in pairs(t) do
    if value == rapidjson.null then
      t[key] = nil
    else
      t[key] = replaceRapidJsonNullWithNilRecursively(value)
    end
  end

  return t
end

--- @class RequestParameters
--- @field url string The URL of the request
--- @field method string? The request method to be used
--- @field body unknown? The request body to be sent. Must be encodable as JSON.
--- @field query_params table<string, string|number>? The query parameters to be sent on request.
--- @field timeout number? The timeout used for this request. If unset, the default `luasocket` timeout will be used.

--- @class SuccessfulResponse<T>: { type: 'SUCCESS', body: T }
--- @class ErrorResponse: { type: 'ERROR', message: string }

--- Performs a HTTP request, using JSON to encode the request body and to decode the response body.
--- @param request RequestParameters The parameters used for this request.
--- @generic T: any
--- @nodiscard
--- @return SuccessfulResponse<T>|ErrorResponse # The parsed JSON response or nil, if there was an error.
local function requestJson(request)
  local url = require("socket.url")
  local ltn12 = require("ltn12")
  local http = require("socket.http")
  local socketutil = require("socketutil")
  local parsed_url = url.parse(request.url)

  -- FIXME naming
  local query_params = request.query_params or {}
  local built_query_params = ""
  for name, value in pairs(query_params) do
    if built_query_params ~= "" then
      built_query_params = built_query_params .. "&"
    end
    built_query_params = built_query_params .. name .. "=" .. url.escape(value)
  end

  parsed_url.query = built_query_params ~= "" and built_query_params or nil
  local built_url = url.build(parsed_url)

  local headers = {}
  local serialized_body = nil
  if request.body ~= nil then
    serialized_body = rapidjson.encode(request.body)
    headers["Content-Type"] = "application/json"
    headers["Content-Length"] = serialized_body:len()
  end

  -- Specify a timeout for the given request
  local timeout = request.timeout or nil
  if timeout ~= nil then
    socketutil:set_timeout(timeout, timeout)
  end

  logger.info("Requesting to ", parsed_url, built_query_params)

  local sink = {}
  local _, status_code, response_headers = http.request({
    url = built_url,
    method = request.method or "GET",
    headers = headers,
    source = serialized_body ~= nil and ltn12.source.string(serialized_body) or nil,
    sink = ltn12.sink.table(sink)
  })

  socketutil:reset_timeout()

  local response_body = table.concat(sink)
  -- Under normal conditions, we should always have a request body, even when the status code
  -- is not 2xx
  local parsed_body, err = rapidjson.decode(response_body)
  if err then
    error("Expected to be able to decode the response body as JSON: " ..
      response_body .. "(status code: " .. status_code .. ")")
  end

  if not (status_code and status_code >= 200 and status_code <= 299) then
    logger.err("Request failed with status code", status_code, "and body", parsed_body)
    local error_message = parsed_body.message
    assert(error_message ~= nil, "Request failed without error message")

    return { type = 'ERROR', message = error_message }
  end

  return { type = 'SUCCESS', body = replaceRapidJsonNullWithNilRecursively(parsed_body) }
end

local function getSourceDir()
  local callerSource = debug.getinfo(2, "S").source
  if callerSource:find("^@") then
    return callerSource:gsub("^@(.*)/[^/]*", "%1")
  end
end

local function waitUntilHttpServerIsReady()
  local start_time = os.time()

  while os.time() - start_time < SERVER_STARTUP_TIMEOUT_SECONDS do
    local ok, response = pcall(function()
      return requestJson({
        url = 'http://localhost:30727/health-check',
        timeout = 1,
      })
    end)

    if ok and response.type == 'SUCCESS' then
      return
    end

    ffiutil.sleep(1)
  end

  error('server readiness check timed out')
end

function Backend.initialize()
  assert(Backend.server_pid == nil, "backend was already initialized!")

  -- spawn subprocess and store the pid
  local pid = C.fork()
  if pid == 0 then
    local homePath = DataStorage:getDataDir() .. "/rakuyomi"
    local sourceDir = assert(getSourceDir())

    local serverCommand = nil
    if SERVER_COMMAND_OVERRIDE ~= nil then
      serverCommand = util.splitToArray(SERVER_COMMAND_OVERRIDE, ' ')
    else
      serverCommand = { sourceDir .. "/server" }
    end

    if SERVER_COMMAND_WORKING_DIRECTORY ~= nil then
      ffi.cdef([[
        int chdir(const char *) __attribute__((nothrow, leaf));
      ]])
      logger.info('changing directory to', SERVER_COMMAND_WORKING_DIRECTORY)
      C.chdir(SERVER_COMMAND_WORKING_DIRECTORY)
    end

    local serverCommandWithArgs = {}
    util.arrayAppend(serverCommandWithArgs, serverCommand)
    util.arrayAppend(serverCommandWithArgs, { homePath })

    os.exit(C.execl(serverCommandWithArgs[1], unpack(serverCommandWithArgs, 1, #serverCommandWithArgs + 1))) -- Last arg must be a NULL pointer
  end

  logger.info("Spawned HTTP server with PID " .. pid)
  Backend.server_pid = pid

  waitUntilHttpServerIsReady()
end

--- @class SourceInformation
--- @field id string The ID of the source.
--- @field name string The name of the source.
--- @field version number The version of the source.

--- @class Manga
--- @field id string The ID of the manga.
--- @field source SourceInformation The source information for this manga.
--- @field title string The title of this manga.

--- @class Chapter
--- @field id string The ID of this chapter.
--- @field source_id string The ID of the source for this chapter.
--- @field manga_id string The ID of the manga that this chapter belongs to.
--- @field scanlator string? The scanlation group that worked on this chapter.
--- @field chapter_num number? The chapter number.
--- @field volume_num number? The volume that this chapter belongs to, if known.
--- @field read boolean If this chapter was read to its end.
--- @field downloaded boolean If this chapter was already downloaded to the storage.

--- @class SourceMangaSearchResults
--- @field source_information SourceInformation Information about the source that generated those results.
--- @field mangas Manga[] Found mangas.

-- REFACT Move `http://localhost:30727/` to a constant.

--- Lists mangas added to the user's library.
--- @return SuccessfulResponse<Manga[]>|ErrorResponse
function Backend.getMangasInLibrary()
  return requestJson({
    url = "http://localhost:30727/library",
  })
end

--- Adds a manga to the user's library.
--- @return SuccessfulResponse<nil>|ErrorResponse
function Backend.addMangaToLibrary(source_id, manga_id)
  return requestJson({
    url = "http://localhost:30727/mangas/" .. source_id .. "/" .. util.urlEncode(manga_id) .. "/add-to-library",
    method = "POST"
  })
end

--- Removes a manga from the user's library.
--- @return SuccessfulResponse<nil>|ErrorResponse
function Backend.removeMangaFromLibrary(source_id, manga_id)
  return requestJson({
    url = "http://localhost:30727/mangas/" .. source_id .. "/" .. util.urlEncode(manga_id) .. "/remove-from-library",
    method = "POST"
  })
end

--- Searches manga from the manga sources.
--- @return SuccessfulResponse<Manga[]>|ErrorResponse
function Backend.searchMangas(search_text)
  return requestJson({
    url = "http://localhost:30727/mangas",
    query_params = {
      q = search_text
    }
  })
end

--- Lists chapters from a given manga that are already cached into the database.
--- @return SuccessfulResponse<Chapter[]>|ErrorResponse
function Backend.listCachedChapters(source_id, manga_id)
  return requestJson({
    url = "http://localhost:30727/mangas/" .. source_id .. "/" .. util.urlEncode(manga_id) .. "/chapters",
  })
end

--- Refreshes the chapters of a given manga on the database.
--- @return SuccessfulResponse<{}>|ErrorResponse
function Backend.refreshChapters(source_id, manga_id)
  return requestJson({
    url = "http://localhost:30727/mangas/" .. source_id .. "/" .. util.urlEncode(manga_id) .. "/refresh-chapters",
    method = "POST",
  })
end

--- Begins downloading all chapters from a given manga to the storage.
--- @return SuccessfulResponse<nil>|ErrorResponse
function Backend.downloadAllChapters(source_id, manga_id)
  return requestJson({
    url = "http://localhost:30727/mangas/" .. source_id .. "/" .. util.urlEncode(manga_id) .. "/chapters/download-all",
    method = "POST",
  })
end

--- @alias DownloadAllChaptersProgress { type: 'INITIALIZING' }|{ type: 'PROGRESSING', downloaded: number, total: number }|{ type: 'FINISHED' }|{ type: 'CANCELLED' }

--- Checks the status of a "download all chapters" operation.
--- @return SuccessfulResponse<DownloadAllChaptersProgress>|ErrorResponse
function Backend.getDownloadAllChaptersProgress(source_id, manga_id)
  return requestJson({
    url = "http://localhost:30727/mangas/" ..
        source_id .. "/" .. util.urlEncode(manga_id) .. "/chapters/download-all-progress",
  })
end

--- Requests cancellation of a "download all chapters" operation. This can only be called
--- when the operation status is `PROGRESSING`.
--- @return SuccessfulResponse<nil>|ErrorResponse
function Backend.cancelDownloadAllChapters(source_id, manga_id)
  return requestJson({
    url = "http://localhost:30727/mangas/" ..
        source_id .. "/" .. util.urlEncode(manga_id) .. "/chapters/cancel-download-all",
    method = "POST",
  })
end

--- Downloads the given chapter to the storage.
--- @return SuccessfulResponse<string>|ErrorResponse
function Backend.downloadChapter(source_id, manga_id, chapter_id)
  return requestJson({
    url = "http://localhost:30727/mangas/" ..
        source_id .. "/" .. util.urlEncode(manga_id) .. "/chapters/" .. util.urlEncode(chapter_id) .. "/download",
    method = "POST",
  })
end

--- Marks the chapter as read.
--- @return SuccessfulResponse<nil>|ErrorResponse
function Backend.markChapterAsRead(source_id, manga_id, chapter_id)
  return requestJson({
    url = "http://localhost:30727/mangas/" ..
        source_id .. "/" .. util.urlEncode(manga_id) .. "/chapters/" .. util.urlEncode(chapter_id) .. "/mark-as-read",
    method = "POST",
  })
end

--- Lists information about the installed sources.
--- @return SuccessfulResponse<SourceInformation[]>|ErrorResponse
function Backend.listInstalledSources()
  return requestJson({
    url = "http://localhost:30727/installed-sources",
  })
end

--- Lists information about sources available via our source lists.
--- @return SuccessfulResponse<SourceInformation[]>|ErrorResponse
function Backend.listAvailableSources()
  return requestJson({
    url = "http://localhost:30727/available-sources",
  })
end

--- Installs a source.
--- @return SuccessfulResponse<SourceInformation[]>|ErrorResponse
function Backend.installSource(source_id)
  return requestJson({
    url = "http://localhost:30727/available-sources/" .. source_id .. "/install",
    method = "POST",
  })
end

--- @class GroupSettingDefinition: { type: 'group', title: string|nil, items: SettingDefinition[], footer: string|nil }
--- @class SwitchSettingDefinition: { type: 'switch', title: string, key: string, default: boolean }
--- @class SelectSettingDefinition: { type: 'select', title: string, key: string, values: string[], titles: string[], default: string  }
--- @class TextSettingDefinition: { type: 'text', placeholder: string, key: string, default: string|nil }

--- @alias SettingDefinition GroupSettingDefinition|SwitchSettingDefinition|SelectSettingDefinition|TextSettingDefinition

--- Lists the setting definitions for a given source.
--- @return SuccessfulResponse<SettingDefinition[]>|ErrorResponse
function Backend.getSourceSettingDefinitions(source_id)
  return requestJson({
    url = "http://localhost:30727/installed-sources/" .. source_id .. "/setting-definitions",
  })
end

--- Finds the stored settings for a given source.
--- @return SuccessfulResponse<table<string, string|boolean>>|ErrorResponse
function Backend.getSourceStoredSettings(source_id)
  return requestJson({
    url = "http://localhost:30727/installed-sources/" .. source_id .. "/stored-settings",
  })
end

function Backend.setSourceStoredSettings(source_id, stored_settings)
  return requestJson({
    url = "http://localhost:30727/installed-sources/" .. source_id .. "/stored-settings",
    method = 'POST',
    body = stored_settings,
  })
end

--- @alias ChapterSortingMode 'chapter_ascending'|'chapter_descending'
--- @class Settings: { chapter_sorting_mode: ChapterSortingMode }

--- Reads the application settings.
--- @return SuccessfulResponse<Settings>|ErrorResponse
function Backend.getSettings()
  return requestJson({
    url = "http://localhost:30727/settings"
  })
end

--- Updates the application settings.
--- @return SuccessfulResponse<Settings>|ErrorResponse
function Backend.setSettings(settings)
  return requestJson({
    url = "http://localhost:30727/settings",
    method = 'PUT',
    body = settings
  })
end

function Backend.cleanup()
  logger.info("Terminating subprocess with PID " .. Backend.server_pid)
  -- send SIGTERM to the backend
  C.kill(Backend.server_pid, 15)
  local done = ffiutil.isSubProcessDone(Backend.server_pid, true)
  logger.info("Subprocess is done:", done)
end

-- we can't really rely upon Koreader informing us it has terminated because
-- the plugin lifecycle is really obscure, so use the garbage collector to
-- detect we're done and cleanup
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
