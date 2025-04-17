local ffi = require('ffi')
local C = ffi.C
local UIManager = require('ui/uimanager')
local util = {}

---@param operation string
---@param return_code number
function util.must(operation, return_code)
  if return_code < 0 then
    error("failed to " .. operation .. ": " .. ffi.string(C.strerror(ffi.errno())))
  end

  return return_code
end

local F_SETFL = 4
local O_NONBLOCK = 0x4

---@class SubprocessOutputCapturer
---@field stdout_pipe ffi.cdata*
---@field stderr_pipe ffi.cdata*
local SubprocessOutputCapturer = {}

---@return SubprocessOutputCapturer
function SubprocessOutputCapturer:new()
  local obj = {
    stdout_pipe = ffi.new("int[2]"),
    stderr_pipe = ffi.new("int[2]"),
  }
  setmetatable(obj, { __index = self })

  util.must("create stdout pipe", C.pipe(obj.stdout_pipe))
  util.must("create stderr pipe", C.pipe(obj.stderr_pipe))

  -- Set reading end to non-blocking
  util.must("set stdout non-blocking", C.fcntl(obj.stdout_pipe[0], F_SETFL, O_NONBLOCK))
  util.must("set stderr non-blocking", C.fcntl(obj.stderr_pipe[0], F_SETFL, O_NONBLOCK))

  return obj
end

function SubprocessOutputCapturer:setupChildProcess()
  -- Redirect stdout to write end of pipe
  util.must("dup2 stdout", C.dup2(self.stdout_pipe[1], 1))
  util.must("dup2 stderr", C.dup2(self.stderr_pipe[1], 2))

  -- Close reading ends in child
  C.close(self.stdout_pipe[0])
  C.close(self.stderr_pipe[0])
end

function SubprocessOutputCapturer:setupParentProcess()
  -- Close writing ends in parent
  C.close(self.stdout_pipe[1])
  C.close(self.stderr_pipe[1])
end

---@param onStdout fun(contents: string):nil
---@param onStderr fun(contents: string):nil
function SubprocessOutputCapturer:periodicallyPipeOutput(onStdout, onStderr)
  UIManager:scheduleIn(0.5, function()
    self:pipeOutput(onStdout, onStderr)

    self:periodicallyPipeOutput(onStdout, onStderr)
  end)
end

---@param onStdout fun(contents: string):nil
---@param onStderr fun(contents: string):nil
function SubprocessOutputCapturer:pipeOutput(onStdout, onStderr)
  local buffer = ffi.new("char[?]", 1024)
  local function readPipe(fd, callback)
    local bytes_read = C.read(fd, buffer, 1024)
    if bytes_read > 0 then
      callback(ffi.string(buffer, bytes_read))
    end
  end

  -- Read from both pipes
  readPipe(self.stdout_pipe[0], onStdout)
  readPipe(self.stderr_pipe[0], onStderr)
end

util.SubprocessOutputCapturer = SubprocessOutputCapturer

return util
