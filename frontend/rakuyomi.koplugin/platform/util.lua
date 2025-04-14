local ffi = require('ffi')
local C = ffi.C

local util = {}

---@param operation string
---@param return_code number
function util.must(operation, return_code)
  if return_code < 0 then
    error("failed to " .. operation .. ": " .. ffi.string(C.strerror(ffi.errno())))
  end

  return return_code
end

return util
