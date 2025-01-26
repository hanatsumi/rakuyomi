from pydantic import BaseModel
from ..koreader_driver import KOReaderDriver

class PageCountResponse(BaseModel):
    total_pages: int

async def count_listing_pages(driver: KOReaderDriver) -> int:
    response = await driver.query(
        "How many pages are there in the current listing?",
        PageCountResponse
    )

    return response.total_pages