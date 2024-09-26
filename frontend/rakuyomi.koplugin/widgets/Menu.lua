local BaseMenu = require("ui/widget/menu")
local logger = require("logger")

local Icons = require("Icons")

local Menu = BaseMenu:extend {
  with_context_menu = false,
}

function Menu:init()
  if self.with_context_menu then
    self.align_baselines = true
  end

  BaseMenu.init(self)
end

function Menu:updateItems(select_number)
  for _, item in ipairs(self.item_table) do
    if self.with_context_menu then
      item.mandatory = (item.mandatory or "") .. Icons.FA_ELLIPSIS_VERTICAL
    end
  end

  BaseMenu.updateItems(self, select_number)
end

function Menu:onMenuSelect(entry, pos)
  local selected_context_menu = pos ~= nil and pos.x > 0.8

  if selected_context_menu then
    self:onContextMenuSelect(entry, pos)
  else
    self:onPrimaryMenuSelect(entry, pos)
  end
end

function Menu:onMenuHold(entry, pos)
  self:onContextMenuSelect(entry, pos)
end

function Menu:onPrimaryMenuSelect(entry, pos)
end

function Menu:onContextMenuSelect(entry, pos)
end

return Menu
