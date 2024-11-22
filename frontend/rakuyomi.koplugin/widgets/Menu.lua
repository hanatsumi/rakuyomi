local BaseMenu = require("ui/widget/menu")

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
    if self.with_context_menu and item.select_enabled ~= false then
      item.mandatory = (item.mandatory or "") .. Icons.FA_ELLIPSIS_VERTICAL
    end
  end

  BaseMenu.updateItems(self, select_number)
end

function Menu:onMenuSelect(entry, pos)
  if entry.select_enabled == false then
    return true
  end

  local selected_context_menu = pos ~= nil and pos.x > 0.8

  if selected_context_menu then
    self:onContextMenuChoice(entry, pos)
  else
    self:onPrimaryMenuChoice(entry, pos)
  end
end

function Menu:onMenuHold(entry, pos)
  self:onContextMenuChoice(entry, pos)
end

function Menu:onPrimaryMenuChoice(entry, pos)
end

function Menu:onContextMenuChoice(entry, pos)
end

return Menu
