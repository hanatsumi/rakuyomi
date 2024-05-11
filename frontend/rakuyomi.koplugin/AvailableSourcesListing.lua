local ConfirmBox = require("ui/widget/confirmbox")
local Menu = require("ui/widget/menu")
local UIManager = require("ui/uimanager")
local Screen = require("device").screen
local Trapper = require("ui/trapper")
local Icons = require("Icons")

local Backend = require("Backend")
local ErrorDialog = require("ErrorDialog")
local LoadingDialog = require("LoadingDialog")

--- @class AvailableSourcesListing: { [any]: any }
--- @field installed_sources SourceInformation[]
--- @field available_sources SourceInformation[]
--- @field on_return_callback fun(): nil
local AvailableSourcesListing = Menu:extend {
  name = "available_sources_listing",
  is_enable_shortcut = false,
  is_popout = false,
  title = "Available sources",

  available_sources = nil,
  installed_sources = nil,
  -- callback to be called when pressing the back button
  on_return_callback = nil,
}

function AvailableSourcesListing:init()
  self.available_sources = self.available_sources or {}
  self.installed_sources = self.installed_sources or {}

  self.width = Screen:getWidth()
  self.height = Screen:getHeight()
  Menu.init(self)

  -- see `ChapterListing` for an explanation on this
  -- FIXME we could refactor this into a single class
  self.paths = {
    { callback = self.on_return_callback },
  }
  self.on_return_callback = nil

  self:updateItems()
end

--- Updates the menu item contents with the sources information.
--- @private
function AvailableSourcesListing:updateItems()
  if #self.available_sources > 0 then
    self.item_table = self:generateItemTableFromInstalledAndAvailableSources(self.installed_sources, self
      .available_sources)
    self.multilines_show_more_text = false
    self.items_per_page = nil
    self.single_line = true
  else
    self.item_table = self:generateEmptyViewItemTable()
    self.multilines_show_more_text = true
    self.items_per_page = 1
    self.single_line = false
  end

  Menu.updateItems(self)
end

--- Generates the item table for displaying the search results.
--- @private
--- @param installed_sources SourceInformation[]
--- @param available_sources SourceInformation[]
--- @return table
function AvailableSourcesListing:generateItemTableFromInstalledAndAvailableSources(installed_sources, available_sources)
  --- @type table<string, SourceInformation>
  local installed_sources_by_id = {}

  for _, source_information in ipairs(installed_sources) do
    installed_sources_by_id[source_information.id] = source_information
  end

  local item_table = {}
  for _, source_information in ipairs(available_sources) do
    local mandatory = ""
    local callback = nil

    if installed_sources_by_id[source_information.id] ~= nil then
      local installed_source_info = installed_sources_by_id[source_information.id]

      if installed_source_info.version < source_information.version then
        mandatory = mandatory .. Icons.FA_ARROW_UP .. " Update available!"

        callback = function() self:installSource(source_information) end
      else
        mandatory = mandatory .. Icons.FA_CHECK .. " Latest version installed"
      end
    else
      mandatory = mandatory .. Icons.FA_DOWNLOAD .. " Installable"

      callback = function() self:installSource(source_information) end
    end

    table.insert(item_table, {
      source_information = source_information,
      text = source_information.name .. " (version " .. source_information.version .. ")",
      mandatory = mandatory,
      callback = callback,
    })
  end

  return item_table
end

--- @private
function AvailableSourcesListing:generateEmptyViewItemTable()
  return {
    {
      text = "No available sources found. Try adding some source lists by looking at our README!",
      dim = true,
      select_enabled = false,
    }
  }
end

--- @private
function AvailableSourcesListing:onReturn()
  local path = table.remove(self.paths)

  self:onClose()
  path.callback()
end

--- @private
--- @param source_information SourceInformation
function AvailableSourcesListing:installSource(source_information)
  Trapper:wrap(function()
    local response = LoadingDialog:showAndRun(
      "Installing source...",
      function() return Backend.installSource(source_information.id) end
    )

    if response.type == 'ERROR' then
      ErrorDialog:show(response.message)

      return
    end

    -- TODO refresh the listing
  end)
end

--- Fetches and shows the available sources. Must be called from a function wrapped with `Trapper:wrap()`.
--- @param onReturnCallback any
function AvailableSourcesListing:fetchAndShow(onReturnCallback)
  local installed_sources_response = Backend.listInstalledSources()
  if installed_sources_response.type == 'ERROR' then
    ErrorDialog:show(installed_sources_response.message)

    return
  end

  local installed_sources = installed_sources_response.body

  local available_sources_response = LoadingDialog:showAndRun("Fetching available sources...", function()
    return Backend.listAvailableSources()
  end)

  if available_sources_response.type == 'ERROR' then
    ErrorDialog:show(available_sources_response.message)

    return
  end

  local available_sources = available_sources_response.body

  UIManager:show(AvailableSourcesListing:new {
    installed_sources = installed_sources,
    available_sources = available_sources,
    on_return_callback = onReturnCallback,
    covers_fullscreen = true, -- hint for UIManager:_repaint()
  })
end

return AvailableSourcesListing
