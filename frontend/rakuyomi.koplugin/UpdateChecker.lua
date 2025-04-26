local ButtonDialog = require("ui/widget/buttondialog")
local UIManager = require("ui/uimanager")
local InfoMessage = require("ui/widget/infomessage")
local NetworkMgr = require("ui/network/manager")
local Trapper = require("ui/trapper")
local _ = require("gettext")

local Backend = require("Backend")
local ErrorDialog = require("ErrorDialog")
local Icons = require("Icons")
local LoadingDialog = require("LoadingDialog")

local UpdateChecker = {}

function UpdateChecker:checkForUpdates()
  if not NetworkMgr:isConnected() then
    ErrorDialog:show(_("Cannot check for updates while offline"))
    return
  end

  local response = Backend.checkForUpdates()

  if response.type == "ERROR" then
    ErrorDialog:show(response.message)
    return
  end

  --- @type UpdateInfo
  local update_info = response.body
  if not update_info.available then
    UIManager:show(InfoMessage:new {
      text = _("You're running the latest version!")
    })
    return
  end

  local dialog

  local buttons = {
    {
      {
        text = _("Later"),
        callback = function()
          UIManager:close(dialog)
        end
      },
      {
        text = _("Update Now"),
        callback = function()
          UIManager:close(dialog)
          self:installUpdate(update_info.latest_version)
        end
      }
    }
  }

  dialog = ButtonDialog:new {
    title = string.format(
      _("Version %s is available. You're currently running version %s.\n\nWould you like to update now?"),
      update_info.latest_version,
      update_info.current_version or "unknown"
    ),
    buttons = buttons
  }

  UIManager:show(dialog)
end

function UpdateChecker:installUpdate(version)
  Trapper:wrap(function()
    local response = LoadingDialog:showAndRun(
      _("Updating rakuyomi, please wait..."),
      function()
        return Backend.installUpdate(version)
      end
    )

    if response.type == "ERROR" then
      ErrorDialog:show(response.message)
      return
    end

    local dialog
    -- Show success and prompt for restart
    local buttons = {
      {
        {
          text = _("Restart Now"),
          callback = function()
            UIManager:close(dialog)

            self:showMessageAndRestart()
          end
        }
      }
    }

    dialog = ButtonDialog:new {
      title = _("The update has been installed successfully. KOReader needs to be restarted to apply the changes."),
      buttons = buttons
    }

    UIManager:show(dialog)
  end)
end

function UpdateChecker:showMessageAndRestart()
  UIManager:show(InfoMessage:new {
    text = _("Restarting..."),
    dismissable = false,
  })

  UIManager:nextTick(function()
    UIManager:restartKOReader()
  end)
end

return UpdateChecker
