from typing import List, Literal, Annotated
from pydantic import BaseModel, Field, TypeAdapter

from ..koreader_driver import KOReaderDriver

class DialogButton(BaseModel):
    text: str
    x: int
    y: int
    width: int
    height: int

class DialogVisibleResponse(BaseModel):
    type: Literal['dialog_visible']
    text: str
    buttons: List[DialogButton]

class NoDialogResponse(BaseModel):
    type: Literal['no_dialog']

DialogResponse: TypeAdapter[Annotated[DialogVisibleResponse | NoDialogResponse, Field(discriminator='type')]] = TypeAdapter(Annotated[DialogVisibleResponse | NoDialogResponse, Field(discriminator='type')])

async def describe_dialog(driver: KOReaderDriver) -> DialogVisibleResponse | NoDialogResponse:
    """
    Get a description of the currently displayed dialog.
    Returns NoDialogResponse if no dialog is shown.
    """
    return await driver.query(
        "Is there a dialog visible on screen? If yes, describe: 1) The dialog text, and 2) For each button in the dialog, provide its text and location (x, y coordinates, width, and height). If no dialog is visible, indicate that.",
        DialogResponse
    )
