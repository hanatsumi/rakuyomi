local Blitbuffer = require("ffi/blitbuffer")
local CheckButton = require("ui/widget/checkbutton")
local CheckMark = require("ui/widget/checkmark")
local Device = require("device")
local FocusManager = require("ui/widget/focusmanager")
local HorizontalGroup = require("ui/widget/horizontalgroup")
local HorizontalSpan = require("ui/widget/horizontalspan")
local InputDialog = require("ui/widget/inputdialog")
local GestureRange = require("ui/gesturerange")
local Geom = require("ui/geometry")
local Font = require("ui/font")
local FrameContainer = require("ui/widget/container/framecontainer")
local InputContainer = require("ui/widget/container/inputcontainer")
local LineWidget = require("ui/widget/linewidget")
local OverlapGroup = require("ui/widget/overlapgroup")
local RadioButtonWidget = require("ui/widget/radiobuttonwidget")
local Screen = require("device").screen
local Size = require("ui/size")
local TextWidget = require("ui/widget/textwidget")
local TextBoxWidget = require("ui/widget/textboxwidget")
local TitleBar = require("ui/widget/titlebar")
local UIManager = require("ui/uimanager")
local VerticalGroup = require("ui/widget/verticalgroup")
local logger = require("logger")

local Backend = require("Backend")
local ErrorDialog = require("ErrorDialog")
local Icons = require("Icons")

local FOOTER_FONT_SIZE = 14
local SETTING_ITEM_FONT_SIZE = 18

--- @class SettingItemValue: { [any]: any }
--- @field setting_definition SettingDefinition
--- @field on_return_callback fun(): nil
local SettingItemValue = InputContainer:extend {
  show_parent = nil,
  max_width = nil,
  setting_definition = nil,
  -- If not set, the default value from the setting definition will be shown instead.
  value = nil,
  on_value_changed_callback = nil,
}

--- @private
function SettingItemValue:init()
  -- REFACT We should refactor this `SettingDefinition` type in a way that actual settings
  -- are different from a group. See the backend's side defintiion of `SettingDefinition` for
  -- more details.
  assert(self.setting_definition.type ~= "group")
  self.show_parent = self.show_parent or self

  self.ges_events = {
    Tap = {
      GestureRange:new {
        ges = "tap",
        range = function()
          return self.dimen
        end
      }
    },
  }

  self[1] = self:createValueWidget()
end

--- @private
function SettingItemValue:getCurrentValue()
  local value = self.value
  if value == nil then
    value = self.setting_definition.default
  end

  return value
end

--- @private
function SettingItemValue:createValueWidget()
  -- REFACT maybe split this into multiple widgets, one for each setting definition type
  -- TODO add support for the `subtitle` field of the setting definition
  if self.setting_definition.type == "select" then
    local title_for_value = {}
    for index, value in ipairs(self.setting_definition.values) do
      title_for_value[value] = self.setting_definition.titles[index]
    end

    return TextWidget:new {
      text = title_for_value[self:getCurrentValue()] .. " " .. Icons.UNICODE_ARROW_RIGHT,
      editable = true,
      face = Font:getFace("cfont", SETTING_ITEM_FONT_SIZE),
      max_width = self.max_width,
    }
  elseif self.setting_definition.type == "switch" then
    return CheckMark:new {
      checked = self:getCurrentValue(),
      face = Font:getFace("smallinfofont", SETTING_ITEM_FONT_SIZE),
    }
  elseif self.setting_definition.type == "text" then
    return TextWidget:new {
      text = self:getCurrentValue(),
      editable = true,
      face = Font:getFace("cfont", SETTING_ITEM_FONT_SIZE),
      max_width = self.max_width,
    }
  else
    error("unexpected setting definition type: " .. self.setting_definition.type)
  end
end

--- @private
function SettingItemValue:onTap()
  if self.setting_definition.type == "select" then
    local radio_buttons = {}
    for index, value in ipairs(self.setting_definition.values) do
      local title = self.setting_definition.titles[index]

      table.insert(radio_buttons, {
        {
          text = title,
          provider = value,
          checked = self:getCurrentValue() == value,
        },
      })
    end

    local dialog
    dialog = RadioButtonWidget:new {
      title_text = self.setting_definition.title,
      radio_buttons = radio_buttons,
      callback = function(radio)
        UIManager:close(dialog)

        self:updateCurrentValue(radio.provider)
      end
    }

    UIManager:show(dialog)
  elseif self.setting_definition.type == "switch" then
    self:updateCurrentValue(not self:getCurrentValue())
  elseif self.setting_definition.type == "text" then
    local dialog
    dialog = InputDialog:new {
      title = self.setting_definition.title or self.setting_definition.placeholder,
      input = self:getCurrentValue(),
      input_hint = self.setting_definition.placeholder,
      buttons = {
        {
          {
            text = "Cancel",
            id = "close",
            callback = function()
              UIManager:close(dialog)
            end,
          },
          {
            text = "Save",
            is_enter_default = true,
            callback = function()
              UIManager:close(dialog)

              self:updateCurrentValue(dialog:getInputText())
            end,
          },
        }
      }
    }

    UIManager:show(dialog)
    dialog:onShowKeyboard()
  end
end

--- @private
function SettingItemValue:updateCurrentValue(new_value)
  self.value = new_value
  self[1] = self:createValueWidget()
  -- our dimensions are cached? i mean what the actual fuck
  self.dimen = nil
  UIManager:setDirty(self.show_parent, "ui")

  self.on_value_changed_callback(self.setting_definition.key, new_value)
end

local SettingItem = InputContainer:extend {
  show_parent = nil,
  width = nil,
  setting_definition = nil,
  stored_value = nil,
  on_value_changed_callback = nil,
}

function SettingItem:init()
  self.show_parent = self.show_parent or self
  self.width = self.width or Screen:getWidth()
  self.label_widget = TextBoxWidget:new {
    -- REFACT `text` setting definitions usually have the `placeholder` field as a replacement for
    -- `title`, however this is a implementation detail of Aidoku's extensions and it shouldn't
    -- leak here
    text = self.setting_definition.title or self.setting_definition.placeholder,
    face = Font:getFace("cfont", SETTING_ITEM_FONT_SIZE),
    width = self.width / 2,
  }

  -- FIXME what is this name?????????
  self.value_widget = SettingItemValue:new {
    show_parent = self.show_parent,
    setting_definition = self.setting_definition,
    max_width = self.width / 2,
    value = self.stored_value,
    on_value_changed_callback = function(key, new_value)
      self:onValueChanged(key, new_value)
    end,
  }

  self[1] = HorizontalGroup:new {
    self.label_widget,
    self:createHorizontalSpacerWidget(),
    self.value_widget,
  }
end

--- @private
function SettingItem:createHorizontalSpacerWidget()
  return HorizontalSpan:new {
    width = self.width - self.label_widget:getSize().w - self.value_widget:getSize().w,
  }
end

--- @private
function SettingItem:onValueChanged(key, new_value)
  -- The value widget's size might have changed, which means we need to recalculate the size
  -- of the spacer widget.
  self[1][2] = self:createHorizontalSpacerWidget()
  -- The HorizontalGroup layout is also cached, so we clear it too
  self[1]:resetLayout()

  UIManager:setDirty(self.show_parent, "ui")

  self.on_value_changed_callback(key, new_value)
end

local SourceSettings = FocusManager:extend {
  source_id = nil,
  setting_definitions = nil,
  stored_settings = nil,
  -- callback to be called when pressing the back button
  on_return_callback = nil,
}

--- @private
function SourceSettings:init()
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

  local should_add_separator = false

  for _, setting_definition in ipairs(self.setting_definitions) do
    -- TODO This assumes 1-level groups at maximum, which is probably true for all extensions but
    -- multiple nested groups are technically possible...
    local children = {}
    if setting_definition.type == "group" then
      if setting_definition.title ~= nil then
        vertical_group[#vertical_group + 1] = TextWidget:new {
          text = setting_definition.title,
          face = Font:getFace("cfont"),
          bold = true,
        }
      end

      children = setting_definition.items
    else
      children = { setting_definition }
    end

    if should_add_separator then
      local separator = LineWidget:new {
        background = Blitbuffer.COLOR_LIGHT_GRAY,
        dimen = Geom:new {
          w = self.item_width,
          h = Size.line.thick,
        },
        style = "solid",
      }

      vertical_group[#vertical_group + 1] = separator
    end

    for _, child in ipairs(children) do
      local setting_item = SettingItem:new {
        show_parent = self,
        width = self.item_width,
        setting_definition = child,
        stored_value = self.stored_settings[child.key],
        on_value_changed_callback = function(key, new_value)
          self:updateStoredSetting(key, new_value)
        end
      }

      vertical_group[#vertical_group + 1] = setting_item
    end

    if setting_definition.type == "group" and setting_definition.footer ~= nil then
      local footer = TextBoxWidget:new {
        text = setting_definition.footer,
        face = Font:getFace("cfont", FOOTER_FONT_SIZE),
        color = Blitbuffer.COLOR_LIGHT_GRAY,
        width = self.item_width,
      }

      vertical_group[#vertical_group + 1] = footer
    end

    -- If we're inside a group, add a separator before the next setting widget
    should_add_separator = setting_definition.type == "group"
  end

  self.title_bar = TitleBar:new {
    -- TODO add source name here
    title = "Source settings",
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
function SourceSettings:onClose()
  UIManager:close(self)
end

--- @private
function SourceSettings:onReturn()
  self:onClose()

  self.on_return_callback()
end

--- @private
function SourceSettings:updateStoredSetting(key, new_value)
  self.stored_settings[key] = new_value

  local response = Backend.setSourceStoredSettings(self.source_id, self.stored_settings)
  if response.type == 'ERROR' then
    ErrorDialog:show(response.message)
  end
end

--- @private
function SourceSettings:fetchAndShow(source_id, on_return_callback)
  local setting_definitions_response = Backend.getSourceSettingDefinitions(source_id)
  if setting_definitions_response.type == 'ERROR' then
    ErrorDialog:show(setting_definitions_response.message)
    return
  end

  local stored_settings_response = Backend.getSourceStoredSettings(source_id)
  if stored_settings_response.type == 'ERROR' then
    ErrorDialog:show(stored_settings_response.message)
    return
  end

  local setting_definitions = setting_definitions_response.body
  local stored_settings = stored_settings_response.body

  UIManager:show(SourceSettings:new {
    source_id = source_id,
    setting_definitions = setting_definitions,
    stored_settings = stored_settings,
    on_return_callback = on_return_callback,
  })
end

return SourceSettings
