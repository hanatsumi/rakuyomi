local serpent = require("ffi/serpent")
local rapidjson = require("rapidjson")
local UIManager = require("ui/uimanager")
local logger = require("logger")

local Testing = {}

local function describeCurrentUI()
  local visible_windows = {}
  for i = #UIManager._window_stack, 1, -1 do
    local window = UIManager._window_stack[i]

    visible_windows[#visible_windows + 1] = window

    if window.widget.covers_fullscreen then
      break
    end
  end

  return serpent.block(visible_windows, {
    maxlevel = 10,
    indent = "  ",
    nocode = true,
    // 
    keyignore = {
      "ges_events",
    }
  })
end

function Testing:dumpVisibleUI()
  logger.info("Dumping visible UI")

  local ui_contents = describeCurrentUI()

  local json_message = {
    type = "ui_contents",
    contents = ui_contents,
  }

  print(ui_contents)

  -- print(rapidjson.encode(json_message))
end

return Testing
