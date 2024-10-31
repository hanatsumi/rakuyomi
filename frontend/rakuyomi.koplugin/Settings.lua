local Blitbuffer = require("ffi/blitbuffer")
local FocusManager = require("ui/widget/focusmanager")
local FrameContainer = require("ui/widget/container/framecontainer")
local Geom = require("ui/geometry")
local HorizontalGroup = require("ui/widget/horizontalgroup")
local HorizontalSpan = require("ui/widget/horizontalspan")
local OverlapGroup = require("ui/widget/overlapgroup")
local Screen = require("device").screen
local Size = require("ui/size")
local TitleBar = require("ui/widget/titlebar")
local UIManager = require("ui/uimanager")
local VerticalGroup = require("ui/widget/verticalgroup")
local logger = require("logger")

local Backend = require("Backend")
local ErrorDialog = require("ErrorDialog")
local SettingItem = require('widgets/SettingItem')

-- REFACT This is duplicated from `SourceSettings` (pretty much all of it actually)
local Settings = FocusManager:extend {
  settings = {},
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

  --- @type [string, ValueDefinition][]
  local setting_value_definitions = {
    {
      'chapter_sorting_mode',
      {
        type = 'enum',
        title = 'Chapter sorting mode',
        options = {
          { label = 'By chapter ascending',  value = 'chapter_ascending' },
          { label = 'By chapter descending', value = 'chapter_descending' },
        }
      }
    },
    {
      'storage_size_limit_mb',
      {
        type = 'integer',
        title = 'Storage size limit',
        min_value = 1,
        max_value = 10240,
        unit = 'MB'
      }
    }
  }

  local vertical_group = VerticalGroup:new {
    align = "left",
  }

  for _, tuple in ipairs(setting_value_definitions) do
    local key = tuple[1]
    local definition = tuple[2]

    table.insert(vertical_group, SettingItem:new {
      show_parent = self,
      width = self.item_width,
      label = definition.title,
      value_definition = definition,
      value = self.settings[key],
      on_value_changed_callback = function(new_value)
        self:updateSetting(key, new_value)
      end
    })
  end

  self.title_bar = TitleBar:new {
    title = "Settings",
    fullscreen = true,
    width = self.dimen.w,
    with_bottom_line = true,
    bottom_line_color = Blitbuffer.COLOR_DARK_GRAY,
    bottom_line_h_padding = padding,
    left_icon = "chevron.left",
    left_icon_tap_callback = function()
      self:onReturn()
    end,
    close_callback = function()
      self:onClose()
    end,
  }

  local content = OverlapGroup:new {
    allow_mirroring = false,
    dimen = self.inner_dimen:copy(),
    VerticalGroup:new {
      align = "left",
      self.title_bar,
      HorizontalGroup:new {
        HorizontalSpan:new { width = padding },
        vertical_group
      }
    }
  }

  self[1] = FrameContainer:new {
    show_parent = self,
    width = self.dimen.w,
    height = self.dimen.h,
    padding = 0,
    margin = 0,
    bordersize = border_size,
    focusable = true,
    background = Blitbuffer.COLOR_WHITE,
    content
  }

  UIManager:setDirty(self, "ui")
end

--- @private
function Settings:onClose()
  UIManager:close(self)
end

--- @private
function Settings:onReturn()
  self:onClose()

  self.on_return_callback()
end

--- @private
function Settings:updateSetting(key, value)
  self.settings[key] = value

  local response = Backend.setSettings(self.settings)
  if response.type == 'ERROR' then
    ErrorDialog:show(response.message)
  end
end

function Settings:fetchAndShow(on_return_callback)
  local response = Backend.getSettings()
  if response.type == 'ERROR' then
    ErrorDialog:show(response.message)
  end

  UIManager:show(Settings:new {
    settings = response.body,
    on_return_callback = on_return_callback
  })
end

return Settings
