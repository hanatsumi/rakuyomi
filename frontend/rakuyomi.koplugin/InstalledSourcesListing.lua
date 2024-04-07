local ConfirmBox = require("ui/widget/confirmbox")
local Menu = require("ui/widget/menu")
local UIManager = require("ui/uimanager")
local Screen = require("device").screen
local Trapper = require("ui/trapper")
local AvailableSourcesListing = require("AvailableSourcesListing")

local Backend = require("Backend")
local ErrorDialog = require("ErrorDialog")
local LoadingDialog = require("LoadingDialog")
local ChapterListing = require("ChapterListing")

-- FIXME maybe rename to screen i think ill do it
--- @class InstalledSourcesListing: { [any]: any }
--- @field installed_sources SourceInformation[]
--- @field on_return_callback fun(): nil
local InstalledSourcesListing = Menu:extend {
  name = "installed_sources_listing",
  is_enable_shortcut = false,
  is_popout = false,
  title = "Installed sources",

  installed_sources = nil,
  -- callback to be called when pressing the back button
  on_return_callback = nil,
}

function InstalledSourcesListing:init()
  self.installed_sources = self.installed_sources or {}
  self.title_bar_left_icon = "plus"
  self.onLeftButtonTap = function()
    self:openAvailableSourcesListing()
  end

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

-- Updates the menu item contents with the sources information
function InstalledSourcesListing:updateItems()
  self.item_table = self:generateItemTableFromInstalledSources(self.installed_sources)

  Menu.updateItems(self)
end

--- Generates the item table for displaying the search results.
--- @param installed_sources SourceInformation[]
--- @return table
function InstalledSourcesListing:generateItemTableFromInstalledSources(installed_sources)
  local item_table = {}
  for _, source_information in ipairs(installed_sources) do
    table.insert(item_table, {
      source_information = source_information,
      text = source_information.name .. " (version " .. source_information.version .. ")",
    })
  end

  return item_table
end

function InstalledSourcesListing:onReturn()
  local path = table.remove(self.paths)

  self:onClose()
  path.callback()
end

function InstalledSourcesListing:openAvailableSourcesListing()
  Trapper:wrap(function()
    local response = LoadingDialog:showAndRun("Fetching available sources...", function()
      return Backend.listAvailableSources()
    end)

    if response.type == 'ERROR' then
      ErrorDialog:show(response.message)

      return
    end

    local available_sources = response.body

    AvailableSourcesListing:show(self.installed_sources, available_sources)
  end)
end

--- Shows the installed sources.
--- @param installed_sources SourceInformation[] The results to be shown.
--- @param onReturnCallback any
function InstalledSourcesListing:show(installed_sources, onReturnCallback)
  UIManager:show(InstalledSourcesListing:new {
    installed_sources = installed_sources,
    on_return_callback = onReturnCallback,
    covers_fullscreen = true, -- hint for UIManager:_repaint()
  })
end

return InstalledSourcesListing
