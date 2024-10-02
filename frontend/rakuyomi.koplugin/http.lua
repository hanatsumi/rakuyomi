local DataStorage = require("datastorage")
local ffiutil = require("ffi/util")
local logger = require("logger")
local rapidjson = require("rapidjson")

local Paths = require("Paths")

local COMMAND_WORKING_DIRECTORY = os.getenv('RAKUYOMI_UDS_HTTP_REQUEST_WORKING_DIRECTORY')
local COMMAND_OVERRIDE = os.getenv('RAKUYOMI_UDS_HTTP_REQUEST_COMMAND_OVERRIDE')

local http = {}

--- @class RequestUnixSocketOptions
--- @field method string? The request method. Defaults to "GET".
--- @field body string? The request body to be sent.
--- @field headers table<string, string>? The headers to be sent with the request.
--- @field timeout_seconds number? How many seconds until the request times out. Defaults to 60 seconds.

--- Performs a HTTP request using a Unix domain socket as the transport.
---
--- @param socket_path string Path to the Unix domain socket to be requested.
--- @param path string The path to be sent on the HTTP request, with any query parameters and/or fragment information.
--- @param options RequestUnixSocketOptions
--- @return { type: 'ERROR', message: string }|{ type: 'RESPONSE', status: number, body: string }
function http.requestUnixSocket(socket_path, path, options)
  local udsHttpRequestCommand = COMMAND_OVERRIDE or Paths.getPluginDirectory() .. "/uds_http_request"

  local requestData = {
    socket_path = socket_path,
    path = path,
    method = options.method or "GET",
    headers = options.headers or {},
    body = options.body or "",
    timeout_seconds = options.timeout_seconds or 60,
  }

  local requestJson = rapidjson.encode(requestData)

  -- i swear to god i hate lua it has literally nothing on its stdlib so we have
  -- to do those horrible hacks
  local requestFilePath = os.tmpname()
  local requestFile, err = io.open(requestFilePath, 'w')
  if requestFile == nil then
    return { type = 'ERROR', message = err }
  end

  requestFile:write(requestJson)
  requestFile:close()

  local command = 'cat ' .. requestFilePath .. ' | ' .. udsHttpRequestCommand
  if COMMAND_WORKING_DIRECTORY ~= nil then
    command = 'cd ' .. COMMAND_WORKING_DIRECTORY .. ' && ' .. command
  end

  local output, err = io.popen(command, 'r')
  if output == nil then
    os.remove(requestFilePath)

    return { type = 'ERROR', message = err }
  end

  local responseJson = output:read('*a')
  output:close()

  os.remove(requestFilePath)

  local response, err = rapidjson.decode(responseJson)
  if err ~= nil then
    return { type = 'ERROR', message = err }
  end

  return response
end

return http
