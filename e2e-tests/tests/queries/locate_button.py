from pydantic import BaseModel

from ..koreader_driver import KOReaderDriver

class LocateButtonResponse(BaseModel):
    x: int
    y: int
    width: int
    height: int

async def locate_button(driver: KOReaderDriver, button_name: str) -> LocateButtonResponse:
    return await driver.query(
        f"Click on the '{button_name}' button",
        LocateButtonResponse
    )
