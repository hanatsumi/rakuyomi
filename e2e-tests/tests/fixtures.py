from pathlib import Path
from typing import AsyncGenerator
import pytest

from .agent import Agent
from .koreader_driver import KOReaderDriver

@pytest.fixture
def agent() -> Agent:
    return Agent()

@pytest.fixture
async def koreader_driver(agent: Agent, tmp_path: Path) -> AsyncGenerator[KOReaderDriver]:
    async with KOReaderDriver(agent, tmp_path) as driver:
        yield driver
