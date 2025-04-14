local logger = require('logger')
local Device = require('device')
local ffi = require('ffi')
local C = ffi.C
local ffiutil = require('ffi/util')
local Paths = require('Paths')
local util = require('frontend/util')
local must = require('util').must
local rapidjson = require("rapidjson")

local SERVER_COMMAND_WORKING_DIRECTORY = os.getenv('RAKUYOMI_SERVER_WORKING_DIRECTORY')
local SERVER_COMMAND_OVERRIDE = os.getenv('RAKUYOMI_SERVER_COMMAND_OVERRIDE')
local REQUEST_COMMAND_WORKING_DIRECTORY = os.getenv('RAKUYOMI_UDS_HTTP_REQUEST_WORKING_DIRECTORY')
local REQUEST_COMMAND_OVERRIDE = os.getenv('RAKUYOMI_UDS_HTTP_REQUEST_COMMAND_OVERRIDE')

local SOCKET_PATH = '/tmp/rakuyomi.sock'

---@class UnixServer: Server
---@field pid number
local UnixServer = {}

function UnixServer:new(pid)
  local server = {
    pid = pid
  }
  setmetatable(server, { __index = UnixServer })
  return server
end

function UnixServer:request(request)
  local requestWithDefaults = {
    socket_path = SOCKET_PATH,
    path = request.path,
    method = request.method or "GET",
    headers = request.headers or {},
    body = request.body or "",
    timeout_seconds = request.timeout_seconds or 60,
  }

  local requestJson = rapidjson.encode(requestWithDefaults)

  local udsHttpRequestCommand = REQUEST_COMMAND_OVERRIDE or Paths.getPluginDirectory() .. "/uds_http_request"

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
  if REQUEST_COMMAND_WORKING_DIRECTORY ~= nil then
    command = 'cd ' .. REQUEST_COMMAND_WORKING_DIRECTORY .. ' && ' .. command
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

function UnixServer:stop()
  local SIGTERM = 15

  logger.info("Terminating subprocess with PID " .. self.pid)
  must("kill", C.kill(self.pid, SIGTERM))
  local done = ffiutil.isSubProcessDone(self.pid, true)

  logger.info("Subprocess finished:", done)
end

---@class GenericUnixPlatform: Platform
local GenericUnixPlatform = {}

function GenericUnixPlatform:startServer()
  -- setup loopback on Kobo devices (see #22)
  if Device:isKobo() then
    os.execute("ifconfig lo 127.0.0.1")
  end

  local serverCommand = nil
  if SERVER_COMMAND_OVERRIDE ~= nil then
    serverCommand = util.splitToArray(SERVER_COMMAND_OVERRIDE, ' ')
  else
    serverCommand = { Paths.getPluginDirectory() .. "/server" }
  end

  local serverCommandWithArgs = {}
  util.arrayAppend(serverCommandWithArgs, serverCommand)
  util.arrayAppend(serverCommandWithArgs, { Paths.getHomeDirectory() })

  local pid = must("fork", C.fork())
  if pid == 0 then
    if SERVER_COMMAND_WORKING_DIRECTORY ~= nil then
      ffi.cdef([[
        int chdir(const char *) __attribute__((nothrow, leaf));
      ]])
      logger.info('changing directory to', SERVER_COMMAND_WORKING_DIRECTORY)
      C.chdir(SERVER_COMMAND_WORKING_DIRECTORY)
    end

    local exitCode = must(
      "execl",
      C.execl(serverCommandWithArgs[1], unpack(serverCommandWithArgs, 1, #serverCommandWithArgs + 1))
    )

    logger.info("server exited with code " .. exitCode)
  end

  return UnixServer:new(pid)
end

return GenericUnixPlatform
