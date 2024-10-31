local CheckMark = require("ui/widget/checkmark")
local GestureRange = require("ui/gesturerange")
local Font = require("ui/font")
local InputContainer = require("ui/widget/container/inputcontainer")
local InputDialog = require("ui/widget/inputdialog")
local RadioButtonWidget = require("ui/widget/radiobuttonwidget")
local SpinWidget = require("ui/widget/spinwidget")
local TextWidget = require("ui/widget/textwidget")
local UIManager = require("ui/uimanager")

local Icons = require("Icons")

local SETTING_ITEM_FONT_SIZE = 18

--- @class BooleanValueDefinition: { type: 'boolean' }
--- @class EnumValueDefinitionOption: { label: string, value: string }
--- @class EnumValueDefinition: { type: 'enum', title: string, options: EnumValueDefinitionOption[] }
--- @class IntegerValueDefinition: { type: 'integer', title: string, min_value: number, max_value: number, unit?: string }
--- @class StringValueDefinition: { type: 'string', title: string, placeholder: string }

--- @alias ValueDefinition BooleanValueDefinition|EnumValueDefinition|IntegerValueDefinition|StringValueDefinition

--- @class SettingItemValue: { [any]: any }
--- @field value_definition ValueDefinition
local SettingItemValue = InputContainer:extend {
  show_parent = nil,
  max_width = nil,
  value_definition = nil,
  value = nil,
  on_value_changed_callback = nil,
}

--- @private
function SettingItemValue:init()
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
  return self.value
end

--- @private
function SettingItemValue:createValueWidget()
  -- REFACT maybe split this into multiple widgets, one for each value definition type
  if self.value_definition.type == "enum" then
    local label_for_value = {}
    for _, option in ipairs(self.value_definition.options) do
      label_for_value[option.value] = option.label
    end

    return TextWidget:new {
      text = label_for_value[self:getCurrentValue()] .. " " .. Icons.UNICODE_ARROW_RIGHT,
      editable = true,
      face = Font:getFace("cfont", SETTING_ITEM_FONT_SIZE),
      max_width = self.max_width,
    }
  elseif self.value_definition.type == "boolean" then
    return CheckMark:new {
      checked = self:getCurrentValue(),
      face = Font:getFace("smallinfofont", SETTING_ITEM_FONT_SIZE),
    }
  elseif self.value_definition.type == "integer" then
    return TextWidget:new {
      text = self:getCurrentValue() .. (self.value_definition.unit and (' ' .. self.value_definition.unit) or '') .. ' ' .. Icons.UNICODE_ARROW_RIGHT,
      editable = true,
      face = Font:getFace("cfont", SETTING_ITEM_FONT_SIZE),
      max_width = self.max_width,
    }
  elseif self.value_definition.type == "string" then
    return TextWidget:new {
      text = self:getCurrentValue(),
      editable = true,
      face = Font:getFace("cfont", SETTING_ITEM_FONT_SIZE),
      max_width = self.max_width,
    }
  else
    error("unexpected value definition type: " .. self.value_definition.type)
  end
end

--- @private
function SettingItemValue:onTap()
  if self.value_definition.type == "enum" then
    local radio_buttons = {}
    for _, option in ipairs(self.value_definition.options) do
      table.insert(radio_buttons, {
        {
          text = option.label,
          provider = option.value,
          checked = self:getCurrentValue() == option.value,
        },
      })
    end

    local dialog
    dialog = RadioButtonWidget:new {
      title_text = self.value_definition.title,
      radio_buttons = radio_buttons,
      callback = function(radio)
        UIManager:close(dialog)

        self:updateCurrentValue(radio.provider)
      end
    }

    UIManager:show(dialog)
  elseif self.value_definition.type == "boolean" then
    self:updateCurrentValue(not self:getCurrentValue())
  elseif self.value_definition.type == "integer" then
    local dialog = SpinWidget:new {
      title_text = self.value_definition.title,
      value = self:getCurrentValue(),
      value_min = self.value_definition.min_value,
      value_max = self.value_definition.max_value,
      callback = function(spin)
        self:updateCurrentValue(spin.value)
      end,
    }

    UIManager:show(dialog)
  elseif self.value_definition.type == "string" then
    local dialog
    dialog = InputDialog:new {
      title = self.value_definition.title,
      input = self:getCurrentValue(),
      input_hint = self.value_definition.placeholder,
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

  self.on_value_changed_callback(new_value)
end

return SettingItemValue
