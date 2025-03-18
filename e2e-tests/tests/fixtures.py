from pathlib import Path
from typing import AsyncGenerator
import pytest
from pytest import FixtureRequest

from .agent import Agent
from .koreader_driver import KOReaderDriver
from .phase_report_hook import phase_report_key

@pytest.fixture
def agent() -> Agent:
    return Agent()

@pytest.fixture
async def koreader_driver(request: FixtureRequest, agent: Agent, tmp_path: Path) -> AsyncGenerator[KOReaderDriver]:
    async with KOReaderDriver(agent, tmp_path) as driver:
        yield driver

        # Screenshot the KOReader window on failure
        test_call = request.node.stash[phase_report_key].get('call')

        if test_call is not None and test_call.failed:
            screenshot_folder = Path('screenshots')
            screenshot_folder.mkdir(parents=True, exist_ok=True)

            screenshot_path = screenshot_folder / f'{request.node.name}.png'
            await driver.screenshot(screenshot_path)

            request.node.add_report_section('call', 'screenshot', f'![Screenshot]({screenshot_path})')
