from pydantic import BaseModel
from typing import Literal

from ..koreader_driver import KOReaderDriver

class AvailableSource(BaseModel):
    name: str
    version: int
    status: Literal['installable', 'update_available', 'installed']

class AvailableSourcesQueryResponse(BaseModel):
    available_sources: list[AvailableSource]

async def list_available_sources(driver: KOReaderDriver) -> list[AvailableSource]:
    response = await driver.query(
        "What sources are listed in the current page?",
        AvailableSourcesQueryResponse
    )

    return response.available_sources
