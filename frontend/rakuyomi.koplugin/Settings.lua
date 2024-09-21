local FocusManager = require("ui/widget/focusmanager")
local Geom = require("ui/geometry")
local InputContainer = require("ui/widget/container/inputcontainer")
local Screen = require("device").screen
local Size = require("ui/size")
local VerticalGroup = require("ui/widget/verticalgroup")

-- REFACT This is duplicated from `SourceSettings` (pretty much all of it actually)

local Settings = FocusManager:extend {
  on_return_callback = nil,
}

--- @private
function Settings:init()
  self.dimen = Geom:new {
    x = 0,
    y = 0,
    w = self.width or Screen:getWidth(),
    h = self.height or Screen:getHeight(),
  }

  if self.dimen.w == Screen:getWidth() and self.dimen.h == Screen:getHeight() then
    self.covers_fullscreen = true -- hint for UIManager:_repaint()
  end

  local border_size = Size.border.window
  local padding = Size.padding.large

  self.inner_dimen = Geom:new {
    w = self.dimen.w - 2 * border_size,
    h = self.dimen.h - 2 * border_size,
  }

  self.item_width = self.inner_dimen.w - 2 * padding

  local vertical_group = VerticalGroup:new {
    align = "left",
  }
end
