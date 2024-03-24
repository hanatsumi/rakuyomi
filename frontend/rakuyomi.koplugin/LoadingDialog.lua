local InfoMessage = require("ui/widget/infomessage")
local UIManager = require("ui/uimanager")
local Trapper = require("ui/trapper")

local LoadingDialog = {}

--- Shows a message in a info dialog, while running the given `runnable` function.
--- Must be called from inside a function wrapped with `Trapper:wrap()`.
---
--- @generic T: any
--- @param message string The message to be shown on the dialog.
--- @param runnable fun(): T The function to be ran while showing the dialog.
--- @return T
function LoadingDialog:showAndRun(message, runnable)
    local message_dialog = InfoMessage:new {
        text = message,
    }

    UIManager:show(message_dialog)
    UIManager:forceRePaint()

    local completed, return_values = Trapper:dismissableRunInSubprocess(runnable, message_dialog)
    assert(completed, "Expected runnable to run to completion")

    UIManager:close(message_dialog)

    return return_values
end

return LoadingDialog
