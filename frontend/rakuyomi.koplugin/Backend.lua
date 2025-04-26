local logger = require("logger")
local ffiutil = require("ffi/util")
local rapidjson = require("rapidjson")
local util = require("util")

local Paths = require("Paths")
local Platform = require("Platform")

local SERVER_STARTUP_TIMEOUT_SECONDS = tonumber(os.getenv('RAKUYOMI_SERVER_STARTUP_TIMEOUT') or 5)

--- @class Backend
--- @field private server Server
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
--- @field path string The path of the request
--- @field method string? The request method to be used
--- @field body unknown? The request body to be sent. Must be encodable as JSON.
--- @field query_params table<string, string|number>? The query parameters to be sent on request.
--- @field timeout number? The timeout used for this request. If unset, the default value for the platform will be used (usually 60 seconds).

--- @class SuccessfulResponse<T>: { type: 'SUCCESS', body: T }
--- @class ErrorResponse: { type: 'ERROR', message: string }

--- Performs a HTTP request, using JSON to encode the request body and to decode the response body.
--- @private
--- @param request RequestParameters The parameters used for this request.
--- @generic T: any
--- @nodiscard
--- @return SuccessfulResponse<T>|ErrorResponse # The parsed JSON response or nil, if there was an error.
function Backend.requestJson(request)
  assert(Backend.server ~= nil, "backend wasn't initialized!")
  local url = require("socket.url")

  -- FIXME naming
  local query_params = request.query_params or {}
  local built_query_params = ""
  for name, value in pairs(query_params) do
    if built_query_params ~= "" then
      built_query_params = built_query_params .. "&"
    end
    built_query_params = built_query_params .. name .. "=" .. url.escape(value)
  end

  local path_and_query = request.path
  if built_query_params ~= "" then
    path_and_query = path_and_query .. "?" .. built_query_params
  end

  local headers = {}
  local serialized_body = nil
  if request.body ~= nil then
    serialized_body = rapidjson.encode(request.body)
    headers["Content-Type"] = "application/json"
    headers["Content-Length"] = tostring(serialized_body:len())
  end

  logger.info('Requesting to', path_and_query)

  local response = Backend.server:request(
    {
      path = path_and_query,
      method = request.method or "GET",
      headers = headers,
      body = serialized_body,
    }
  )

  if response.type == 'ERROR' then
    return response
  end

  -- Under normal conditions, we should always have a request body, even when the status code
  -- is not 2xx
  local parsed_body, err = rapidjson.decode(response.body)
  if err then
    error("Expected to be able to decode the response body as JSON: " ..
      response.body .. "(status code: " .. response.status .. ")")
  end

  if not (response.status and response.status >= 200 and response.status <= 299) then
    logger.err("Request failed with status code", response.status, "and body", parsed_body)
    local error_message = parsed_body.message
    assert(error_message ~= nil, "Request failed without error message")

    return { type = 'ERROR', message = error_message }
  end

  return { type = 'SUCCESS', body = replaceRapidJsonNullWithNilRecursively(parsed_body) }
end

---@return boolean
local function waitUntilHttpServerIsReady()
  local start_time = os.time()

  while os.time() - start_time < SERVER_STARTUP_TIMEOUT_SECONDS do
    local ok, response = pcall(function()
      return Backend.requestJson({
        path = '/health-check',
        timeout = 1,
      })
    end)

    if ok and response.type == 'SUCCESS' then
      return true
    end

    ffiutil.sleep(1)
  end

  return false
end

---@return boolean success Whether the backend was initialized successfully.
---@return string|nil logs On error, the last logs written by the server.
function Backend.initialize()
  assert(Backend.server == nil, "backend was already initialized!")

  Backend.server = Platform:startServer()

  if not waitUntilHttpServerIsReady() then
    local logBuffer = Backend.server:getLogBuffer()

    return false, table.concat(logBuffer, "\n")
  end

  return true, nil
end

--- @class SourceInformation
--- @field id string The ID of the source.
--- @field name string The name of the source.
--- @field version number The version of the source.

--- @class Manga
--- @field id string The ID of the manga.
--- @field source SourceInformation The source information for this manga.
--- @field title string The title of this manga.
--- @field unread_chapters_count number|nil The number of unread chapters for this manga, or `nil` if we do not know how many chapters this manga has.

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

--- Lists mangas added to the user's library.
--- @return SuccessfulResponse<Manga[]>|ErrorResponse
function Backend.getMangasInLibrary()
  return Backend.requestJson({
    path = "/library",
  })
end

--- Adds a manga to the user's library.
--- @return SuccessfulResponse<nil>|ErrorResponse
function Backend.addMangaToLibrary(source_id, manga_id)
  return Backend.requestJson({
    path = "/mangas/" .. source_id .. "/" .. util.urlEncode(manga_id) .. "/add-to-library",
    method = "POST"
  })
end

--- Removes a manga from the user's library.
--- @return SuccessfulResponse<nil>|ErrorResponse
function Backend.removeMangaFromLibrary(source_id, manga_id)
  return Backend.requestJson({
    path = "/mangas/" .. source_id .. "/" .. util.urlEncode(manga_id) .. "/remove-from-library",
    method = "POST"
  })
end

--- Searches manga from the manga sources.
--- @return SuccessfulResponse<Manga[]>|ErrorResponse
function Backend.searchMangas(search_text)
  return Backend.requestJson({
    path = "/mangas",
    query_params = {
      q = search_text
    }
  })
end

--- Lists chapters from a given manga that are already cached into the database.
--- @return SuccessfulResponse<Chapter[]>|ErrorResponse
function Backend.listCachedChapters(source_id, manga_id)
  return Backend.requestJson({
    path = "/mangas/" .. source_id .. "/" .. util.urlEncode(manga_id) .. "/chapters",
  })
end

--- Refreshes the chapters of a given manga on the database.
--- @return SuccessfulResponse<{}>|ErrorResponse
function Backend.refreshChapters(source_id, manga_id)
  return Backend.requestJson({
    path = "/mangas/" .. source_id .. "/" .. util.urlEncode(manga_id) .. "/refresh-chapters",
    method = "POST",
  })
end

--- Begins downloading all chapters from a given manga to the storage.
--- @return SuccessfulResponse<nil>|ErrorResponse
function Backend.downloadAllChapters(source_id, manga_id)
  return Backend.requestJson({
    path = "/mangas/" .. source_id .. "/" .. util.urlEncode(manga_id) .. "/chapters/download-all",
    method = "POST",
  })
end

--- @alias DownloadAllChaptersProgress { type: 'INITIALIZING' }|{ type: 'PROGRESSING', downloaded: number, total: number }|{ type: 'FINISHED' }|{ type: 'CANCELLED' }

--- Checks the status of a "download all chapters" operation.
--- @return SuccessfulResponse<DownloadAllChaptersProgress>|ErrorResponse
function Backend.getDownloadAllChaptersProgress(source_id, manga_id)
  return Backend.requestJson({
    path = "/mangas/" ..
        source_id .. "/" .. util.urlEncode(manga_id) .. "/chapters/download-all-progress",
  })
end

--- Requests cancellation of a "download all chapters" operation. This can only be called
--- when the operation status is `PROGRESSING`.
--- @return SuccessfulResponse<nil>|ErrorResponse
function Backend.cancelDownloadAllChapters(source_id, manga_id)
  return Backend.requestJson({
    path = "/mangas/" ..
        source_id .. "/" .. util.urlEncode(manga_id) .. "/chapters/cancel-download-all",
    method = "POST",
  })
end

--- Downloads the given chapter to the storage.
--- @return SuccessfulResponse<string>|ErrorResponse
function Backend.downloadChapter(source_id, manga_id, chapter_id, chapter_num)
  local query_params = {}

  if chapter_num ~= nil then
    query_params.chapter_num = chapter_num
  end

  return Backend.requestJson({
    path = "/mangas/" ..
        source_id .. "/" .. util.urlEncode(manga_id) .. "/chapters/" .. util.urlEncode(chapter_id) .. "/download",
    query_params = query_params,
    method = "POST",
  })
end

--- Marks the chapter as read.
--- @return SuccessfulResponse<nil>|ErrorResponse
function Backend.markChapterAsRead(source_id, manga_id, chapter_id)
  return Backend.requestJson({
    path = "/mangas/" ..
        source_id .. "/" .. util.urlEncode(manga_id) .. "/chapters/" .. util.urlEncode(chapter_id) .. "/mark-as-read",
    method = "POST",
  })
end

--- Lists information about the installed sources.
--- @return SuccessfulResponse<SourceInformation[]>|ErrorResponse
function Backend.listInstalledSources()
  return Backend.requestJson({
    path = "/installed-sources",
  })
end

--- Lists information about sources available via our source lists.
--- @return SuccessfulResponse<SourceInformation[]>|ErrorResponse
function Backend.listAvailableSources()
  return Backend.requestJson({
    path = "/available-sources",
  })
end

--- Installs a source.
--- @return SuccessfulResponse<SourceInformation[]>|ErrorResponse
function Backend.installSource(source_id)
  return Backend.requestJson({
    path = "/available-sources/" .. source_id .. "/install",
    method = "POST",
  })
end

--- Uninstalls a source.
--- @return SuccessfulResponse<nil>|ErrorResponse
function Backend.uninstallSource(source_id)
  return Backend.requestJson({
    path = "/installed-sources/" .. source_id,
    method = "DELETE",
  })
end

--- @class GroupSettingDefinition: { type: 'group', title: string|nil, items: SettingDefinition[], footer: string|nil }
--- @class SwitchSettingDefinition: { type: 'switch', title: string, key: string, default: boolean }
--- @class SelectSettingDefinition: { type: 'select', title: string, key: string, values: string[], titles: string[]|nil, default: string  }
--- @class TextSettingDefinition: { type: 'text', placeholder: string, key: string, default: string|nil }
--- @class LinkSettingDefinition: { type: 'link', title: string, url: string }

--- @alias SettingDefinition GroupSettingDefinition|SwitchSettingDefinition|SelectSettingDefinition|TextSettingDefinition|LinkSettingDefinition

--- Lists the setting definitions for a given source.
--- @return SuccessfulResponse<SettingDefinition[]>|ErrorResponse
function Backend.getSourceSettingDefinitions(source_id)
  return Backend.requestJson({
    path = "/installed-sources/" .. source_id .. "/setting-definitions",
  })
end

--- Finds the stored settings for a given source.
--- @return SuccessfulResponse<table<string, string|boolean>>|ErrorResponse
function Backend.getSourceStoredSettings(source_id)
  return Backend.requestJson({
    path = "/installed-sources/" .. source_id .. "/stored-settings",
  })
end

function Backend.setSourceStoredSettings(source_id, stored_settings)
  return Backend.requestJson({
    path = "/installed-sources/" .. source_id .. "/stored-settings",
    method = 'POST',
    body = stored_settings,
  })
end

--- @alias ChapterSortingMode 'chapter_ascending'|'chapter_descending'
--- @class Settings: { chapter_sorting_mode: ChapterSortingMode }

--- Reads the application settings.
--- @return SuccessfulResponse<Settings>|ErrorResponse
function Backend.getSettings()
  return Backend.requestJson({
    path = "/settings"
  })
end

--- Updates the application settings.
--- @return SuccessfulResponse<Settings>|ErrorResponse
function Backend.setSettings(settings)
  return Backend.requestJson({
    path = "/settings",
    method = 'PUT',
    body = settings
  })
end

--- Creates a new download chapter job. Returns the job's UUID.
--- @return SuccessfulResponse<string>|ErrorResponse
function Backend.createDownloadChapterJob(source_id, manga_id, chapter_id, chapter_num)
  return Backend.requestJson({
    path = "/jobs/download-chapter",
    method = 'POST',
    body = {
      source_id = source_id,
      manga_id = manga_id,
      chapter_id = chapter_id,
      chapter_num = chapter_num,
    }
  })
end

--- Creates a new download unread chapters job. Returns the job's UUID.
--- @return SuccessfulResponse<string>|ErrorResponse
function Backend.createDownloadUnreadChaptersJob(source_id, manga_id, amount)
  return Backend.requestJson({
    path = "/jobs/download-unread-chapters",
    method = 'POST',
    body = {
      source_id = source_id,
      manga_id = manga_id,
      amount = amount
    }
  })
end

--- @class PendingJob<T>: { type: 'PENDING', data: T }
--- @class CompletedJob<T>: { type: 'COMPLETED', data: T }
--- @class ErroredJob: { type: 'ERROR', data: ErrorResponse }

--- @alias DownloadChapterJobDetails PendingJob<nil>|CompletedJob<string>|ErroredJob

--- Gets details about a job.
--- @return SuccessfulResponse<DownloadChapterJobDetails>|ErrorResponse
function Backend.getJobDetails(id)
  return Backend.requestJson({
    path = "/jobs/" .. id,
    method = 'GET'
  })
end

--- Requests for a job to be cancelled.
--- @return SuccessfulResponse<DownloadChapterJobDetails>|ErrorResponse
function Backend.requestJobCancellation(id)
  return Backend.requestJson({
    path = "/jobs/" .. id,
    method = 'DELETE'
  })
end

--- @class UpdateInfo
--- @field public available boolean Whether an update is available
--- @field public current_version string The current version of rakuyomi
--- @field public latest_version string The latest available version
--- @field public release_url string URL to the release page
--- @field public auto_installable boolean Whether the update can be automatically installed

--- Checks if there is an update available for rakuyomi
--- @return SuccessfulResponse<UpdateInfo>|ErrorResponse
function Backend.checkForUpdates()
  return Backend.requestJson({
    path = "/update/check",
    method = "GET"
  })
end

--- Updates the plugin to the given version.
--- @param version string
function Backend.installUpdate(version)
  return Backend.requestJson({
    path = "/update/install",
    method = "POST",
    body = {
      version = version,
    },
    timeout = 120,
  })
end

function Backend.cleanup()
  if Backend.server ~= nil then
    Backend.server:stop()
  end
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
