import time
from typing import Optional, Literal, Annotated, Union
from pydantic import BaseModel, TypeAdapter, Field

from . import queries
from .queries.locate_button import LocateButtonResponse
from .koreader_driver import KOReaderDriver

class Chapter(BaseModel):
    volume_number: Optional[int]
    chapter_number: Optional[int]
    title: str
    scanlator: str

class ChapterQueryResponse(BaseModel):
    chapters: list[Chapter]

class WindowResponse(BaseModel):
    title: str

class ReaderVisibleResponse(BaseModel):
    type: Literal['reader_visible']
    current_file: str

class ReaderNotVisibleResponse(BaseModel):
    type: Literal['reader_not_visible']

ReaderResponse: TypeAdapter[Annotated[ReaderVisibleResponse | ReaderNotVisibleResponse, Field(discriminator='type')]] = TypeAdapter(Annotated[ReaderVisibleResponse | ReaderNotVisibleResponse, Field(discriminator='type')])

async def test_open_chapter(koreader_driver: KOReaderDriver):
    # Open library view
    await koreader_driver.open_library_view()

    # Click on the menu button
    menu_button = await queries.locate_button(koreader_driver, "menu")
    koreader_driver.click_element(menu_button)
    time.sleep(0.1)

    # Click on the "Manage Sources" button
    manage_sources_button = await queries.locate_button(koreader_driver, "Manage Sources")
    koreader_driver.click_element(manage_sources_button)
    await koreader_driver.wait_for_event('installed_sources_listing_shown')

    # Click on the add button
    add_button = await koreader_driver.query("Locate the 'plus' button, in the top left corner", LocateButtonResponse)
    koreader_driver.click_element(add_button)
    await koreader_driver.wait_for_event('available_sources_listing_shown')

    # Get list of sources
    available_sources = await queries.list_available_sources(koreader_driver)
    batoto_source = next(
        (source for source in available_sources if source.name == 'Bato.to'),
        None
    )
    assert batoto_source is not None, "Couldn't find the Bato.to source"

    install_batoto_source = await koreader_driver.query(
        f"Find the menu item with the text '{batoto_source.name} (version {batoto_source.version})'",
        LocateButtonResponse
    )
    koreader_driver.click_element(install_batoto_source)
    await koreader_driver.wait_for_event('source_installed')

    # Go back to the Library View
    await koreader_driver.open_library_view()

    # Click on the menu button
    menu_button = await queries.locate_button(koreader_driver, "menu")
    koreader_driver.click_element(menu_button)
    time.sleep(1)

    # Click on the "Search" button
    search_button = await queries.locate_button(koreader_driver, "Search")
    koreader_driver.click_element(search_button)
    time.sleep(1)

    koreader_driver.type('houseki no kuni')
    search_button = await queries.locate_button(koreader_driver, "Search")
    koreader_driver.click_element(search_button)

    await koreader_driver.wait_for_event('manga_search_results_shown')

    # Click on first manga
    mangas = await queries.list_mangas(koreader_driver)
    print(mangas)
    location = await koreader_driver.query(
        f"Find the menu item with the text '{mangas[0].name} ({mangas[0].source})'",
        LocateButtonResponse
    )
    koreader_driver.click_element(location)
    await koreader_driver.wait_for_event('chapter_listing_shown')

    # Get chapters and click first one
    chapter_response = await koreader_driver.query(
        "What chapters are listed in the current page?",
        ChapterQueryResponse
    )
    assert len(chapter_response.chapters) > 0, "No chapters found for manga"
    
    # Click on first chapter
    location = await koreader_driver.query(
        f"Click on the chapter {chapter_response.chapters[0].chapter_number}",
        LocateButtonResponse
    )
    koreader_driver.click_element(location)
    await koreader_driver.wait_for_event('manga_reader_shown', timeout=30)
    
    # Verify we're in reader view
    response = await koreader_driver.query(
        "Is the reader visible? If so, what is the current file being displayed?",
        ReaderResponse
    )

    assert response.type == 'reader_visible'
    assert response.current_file is not None

if __name__ == '__main__':
    import asyncio

    async def main():
        from .agent import Agent

        import logging
        from pathlib import Path
        import tempfile

        logging.basicConfig(level=logging.DEBUG)

        agent = Agent()

        with tempfile.TemporaryDirectory() as temp_dir:
            async with KOReaderDriver(agent, Path(temp_dir)) as driver:
                await test_open_chapter(driver)
    
    asyncio.run(main())