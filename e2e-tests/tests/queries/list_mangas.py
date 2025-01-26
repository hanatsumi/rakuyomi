from pydantic import BaseModel

from ..koreader_driver import KOReaderDriver

class Manga(BaseModel):
    name: str
    source: str

class MangaQueryResponse(BaseModel):
    mangas: list[Manga]

async def list_mangas(driver: KOReaderDriver) -> list[Manga]:
    response = await driver.query(
        "What mangas are listed in the current page?",
        MangaQueryResponse
    )

    return response.mangas
