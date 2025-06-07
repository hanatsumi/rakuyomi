import time

from . import queries
from .queries.locate_button import LocateButtonResponse
from .koreader_driver import KOReaderDriver
import json
import re
import logging
from pydantic import BaseModel, Field, TypeAdapter
from typing import Optional

logger = logging.getLogger(__name__)

class UnreadChaptersResponse(BaseModel):
    unread_chapters: Optional[int]

async def test_library_view(koreader_driver: KOReaderDriver):
    await koreader_driver.install_source('multi.batoto')
    await koreader_driver.open_library_view()

    library_view_mangas = await queries.list_mangas(koreader_driver)
    assert len(library_view_mangas) == 0, "Expected no mangas in library view"

    # Click on the menu button
    menu_button = await queries.locate_button(koreader_driver, "menu")
    koreader_driver.click_element(menu_button)

    # Click on the "Search" button in the menu
    search_button = await queries.locate_button(koreader_driver, "Search")
    koreader_driver.click_element(search_button)
    time.sleep(1)

    # Type and click on the search button in the dialog
    koreader_driver.type('houseki no kuni')
    search_button = await queries.locate_button(koreader_driver, "Search")
    koreader_driver.click_element(search_button)

    await koreader_driver.wait_for_event('manga_search_results_shown')

    # Click-and-hold on first manga
    mangas = await queries.list_mangas(koreader_driver)
    location = await koreader_driver.query(
        f"Find the menu item with the text '{mangas[0].name} ({mangas[0].source})'",
        LocateButtonResponse
    )
    koreader_driver.click_and_hold_element(location)

    time.sleep(1)

    response = await queries.describe_dialog(koreader_driver)
    assert response.type == 'dialog_visible', "Dialog not visible"
    assert re.match(r"^Do you want to add .+ to your library\?$", response.text), f"Dialog text '{response.text}' does not match expected format"

    add_button = next((button for button in response.buttons if button.text == 'Add'), None)
    assert add_button is not None, "Add button not found in dialog"

    koreader_driver.click_element(add_button)
    await koreader_driver.wait_for_event('manga_added_to_library')

    back_button = await queries.locate_button(koreader_driver, "Back")
    koreader_driver.click_element(back_button)
    await koreader_driver.wait_for_event('library_view_shown')

    library_view_mangas = await queries.list_mangas(koreader_driver)
    assert len(library_view_mangas) == 1, "Expected one manga in library view"
    assert library_view_mangas[0].name == mangas[0].name, f"Expected manga name '{mangas[0].name}', got '{library_view_mangas[0].name}'"

    # Open chapter listing from library view, then go back
    location = await koreader_driver.query(
        f"Find the menu item with the text '{library_view_mangas[0].name} ({library_view_mangas[0].source})'",
        LocateButtonResponse
    )
    koreader_driver.click_element(location)
    await koreader_driver.wait_for_event('chapter_listing_shown')

    back_button = await queries.locate_button(koreader_driver, "Back")
    koreader_driver.click_element(back_button)

    await koreader_driver.wait_for_event('library_view_shown')

    # Query how many unread chapters there are for the first manga in the library view,
    # if it is shown.
    response = await koreader_driver.query(
        "How many unread chapters are listed for the first manga in the library view? Return null if there's no information.",
        UnreadChaptersResponse
    )

    assert response.unread_chapters is not None, "Expected unread chapters count to be present"
    assert response.unread_chapters > 0, f"Expected unread chapters count to be greater than 0"