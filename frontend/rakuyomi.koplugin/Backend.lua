local DataStorage = require("datastorage")
local logger = require("logger")
local C = require("ffi").C
local ffiutil = require("ffi/util")
local rapidjson = require("rapidjson")

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

-- FIXME document
local function requestJson(request)
  local url = require("socket.url")
  local ltn12 = require("ltn12")
  local http = require("socket.http")
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

  logger.info("Requesting to ", parsed_url, built_query_params)

  local sink = {}
  local _, status_code, response_headers = http.request({
    url = built_url,
    method = request.method or "GET",
    headers = headers,
    source = serialized_body ~= nil and ltn12.source.string(serialized_body) or nil,
    sink = ltn12.sink.table(sink)
  })

  assert(
    status_code and status_code >= 200 and status_code <= 299,
    "Request failed with status code " .. status_code
  )

  local response_body = table.concat(sink)
  local parsed_body, err = rapidjson.decode(response_body)
  assert(not err)

  return replaceRapidJsonNullWithNilRecursively(parsed_body)
end

function Backend.initialize()
  -- spawn subprocess and store the pid
  local pid = C.fork()
  if pid == 0 then
    local sourcesPath = DataStorage:getDataDir() .. "/rakuyomi/sources"

    local serverPath = DataStorage:getDataDir() .. "/plugins/rakuyomi.koplugin/server"
    local args = table.pack(serverPath, sourcesPath)

    os.exit(C.execl(serverPath, unpack(args, 1, args.n+1))) -- Last arg must be a NULL pointer
  end

  logger.info("Spawned HTTP server with PID " .. pid)
  Backend.server_pid = pid
end

function Backend.searchMangas(search_text, callback)
  callback(requestJson({
    url = "http://localhost:30727/mangas",
    query_params = {
      q = search_text
    }
  }))
end

function Backend.listChapters(source_id, manga_id, callback)
  callback(requestJson({
    url = "http://localhost:30727/mangas/" .. source_id .. "/" .. manga_id .. "/chapters",
  }))
end

function Backend.downloadChapter(source_id, manga_id, chapter_id, output_path, callback)
  callback(requestJson({
    url = "http://localhost:30727/mangas/" .. source_id .. "/" .. manga_id .. "/chapters/" .. chapter_id .. "/download",
    method = "POST",
    body = {
      output_path = output_path,
    },
  }))
end

function Backend.cleanup()
  logger.info("Terminating subprocess with PID " .. Backend.server_pid)
  -- send SIGTERM to the backend
  C.kill(Backend.server_pid, 15)
  local done = ffiutil.isSubProcessDone(Backend.server_pid, true)
  logger.info("Subprocess is done:", done)
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
