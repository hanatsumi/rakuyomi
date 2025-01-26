local serpent = require("ffi/serpent")
local rapidjson = require("rapidjson")

local UIManager = require("ui/uimanager")
local logger = require("logger")

local NullTesting = {
  init = function() end,
  getHomeDirectory = function()
    return nil
  end,
  dumpVisibleUI = function() end,
  emitEvent = function(name, params) end
}

local Testing = {}

local function describeCurrentUI()
  local visible_windows = {}
  for i = #UIManager._window_stack, 0, -1 do
    local window = UIManager._window_stack[i]

    visible_windows[#visible_windows + 1] = window

    if window.widget.covers_fullscreen then
      break
    end
  end

  print("Got " .. #visible_windows .. " visible windows")

  local ignored_keys = {
    key_events = true,
    ges_events = true,
    _xshaping = true,
    face = true,
    koptinterface = true,
    deinflector = true,
    -- This technically helps the AI but is technically not UI
    -- and it takes like a shitload of context space
    item_table = true,
    -- This contains some cdata, which includes hashes. Those break
    -- some caching.
    ftsize = true,
  }

  local keyignore = {}
  local metatable = {}
  metatable.__index = function(table, key)
    if ignored_keys[key] then
      return true
    end

    if string.sub(key, 1, 1) == "_" then
      return true
    end

    return nil
  end

  setmetatable(keyignore, metatable)

  return serpent.block(visible_windows, {
    maxlevel = 15,
    indent = "  ",
    nocode = true,
    comment = false,
    keyignore = keyignore,
  })
end

function Testing:init()
  self:hookOntoKeyPresses()

  logger.info("Testing hooks initialized!")
end

function Testing:getHomeDirectory()
  return os.getenv('RAKUYOMI_TEST_HOME_DIRECTORY')
end

function Testing:dumpVisibleUI()
  logger.info("Dumping visible UI")

  local ui_contents = describeCurrentUI()

  self:emitEvent('ui_contents', {
    contents = ui_contents
  })
end

--- @param name string
--- @param params table|nil
function Testing:emitEvent(name, params)
  local json_message = {
    type = name,
    params = params,
  }

  print(rapidjson.encode(json_message))
end

---@private
function Testing:hookOntoKeyPresses()
  local oldSendEvent = UIManager.sendEvent
  UIManager.sendEvent = function(newSelf, event)
    if event.handler == "onKeyPress" then
      if self:onKeyPress(event.args[1]) then
        return
      end
    end

    oldSendEvent(newSelf, event)
  end
end

---@private
function Testing:onKeyPress(key)
  if key.Shift and key.F8 then
    self:dumpVisibleUI()

    return true
  elseif key.Shift and key.F9 then
    local LibraryView = require("LibraryView")

    LibraryView:fetchAndShow()

    return true
  end
end

return os.getenv('RAKUYOMI_IS_TESTING') == '1' and Testing or NullTesting
