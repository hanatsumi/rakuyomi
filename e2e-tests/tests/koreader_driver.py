import asyncio
from datetime import datetime
import json
import logging
import hashlib
import os
import subprocess
import platform
import pyautogui
import pywinctl
import signal
import time
import traceback
from pathlib import Path
from typing import TypeVar, Type
from pydantic import BaseModel, TypeAdapter
from tests.agent import Agent

logger = logging.getLogger(__name__)

T = TypeVar('T', bound=BaseModel)

class KOReaderDriver:
    def __init__(self, agent: Agent, rakuyomi_home: Path):
        self.agent = agent
        self.rakuyomi_home = rakuyomi_home
    
    async def __aenter__(self):
        """Context manager enter - starts the KOReader process."""
        # Write settings file
        settings = {
            "source_lists": [
                "https://raw.githubusercontent.com/Skittyblock/aidoku-community-sources/refs/heads/gh-pages/index.min.json",
            ],
            "languages": ["en"],
            "storage_size_limit": "2GB",
        }

        with open(self.rakuyomi_home / "settings.json", "w") as f:
            json.dump(settings, f, indent=2)

        self.process = await asyncio.create_subprocess_exec(
            'devbox', 'run', 'dev',
            stdout=subprocess.PIPE,
            limit=512*1024, # 512 KiB
            env={
                **os.environ,
                'RAKUYOMI_IS_TESTING': '1',
                'RAKUYOMI_TEST_HOME_DIRECTORY': self.rakuyomi_home,
            },
            process_group=0
        )
        await self.wait_for_event('initialized')

        self.window = next(w for w in pywinctl.getAllWindows() if w.title.endswith('KOReader'))
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit - cleans up the KOReader process."""
        if hasattr(self, 'process') and self.process:
            os.killpg(os.getpgid(self.process.pid), signal.SIGTERM)
            await self.process.communicate()

    def activate_window(self):
        """Activates the KOReader window."""
        self.window.activate()

    async def open_library_view(self):
        """Opens the library view in KOReader."""
        self.activate_window()
        pyautogui.hotkey('shift', 'f9')

        await self.wait_for_event('library_view_shown')

    async def request_ui_contents(self) -> str:
        """Requests and returns the current UI contents."""
        start = datetime.now()
        self.activate_window()
        pyautogui.hotkey('shift', 'f8')

        event = await self.wait_for_event('ui_contents')
        ui_contents: str = event['params']['contents']
        duration = datetime.now() - start

        logger.info(f'Requested UI contents in {duration.total_seconds()}s, hash is {hashlib.sha256(ui_contents.encode()).hexdigest()}')

        return ui_contents

    async def query(self, query: str, response_class: Type[T] | TypeAdapter[T]) -> T:
        """
        Performs a query on the current UI contents and returns the response.
        
        Args:
            query: The query string to send to the agent
            response_class: The Pydantic model class to parse the response into
            
        Returns:
            An instance of response_class containing the query results
        """
        ui_contents = await self.request_ui_contents()

        return self.agent.query(
            ui_contents=ui_contents,
            query=query,
            response_class=response_class
        )

    def type(self, text: str):
        """
        Types the given text into the active window.
        
        Args:
            text: The text to type
        """
        self.activate_window()

        if platform.system() == 'Darwin':
            # Workaround for https://github.com/asweigart/pyautogui/issues/796
            pyautogui.keyUp('fn')
        pyautogui.write(text)

    def click_element(self, element_location):
        """
        Clicks on an element based on its location response.
        
        Args:
            element_location: A LocateButtonResponse or similar object containing x, y, width, height
        """
        self.activate_window()
        window_offset_x = element_location.x + element_location.width // 2
        window_offset_y = element_location.y + element_location.height // 2

        # Handle retina displays on macOS
        is_retina = platform.system() == 'Darwin' and subprocess.call(
            "system_profiler SPDisplaysDataType | grep 'retina'", 
            shell=True
        )
        if is_retina:
            window_offset_x /= 2
            window_offset_y /= 2

        window_area = self.window.getClientFrame()
        x = window_area.left + window_offset_x
        y = window_area.top + window_offset_y

        pyautogui.moveTo(x=x, y=y)
        # FIXME Using `click` here causes some weird bugs inside KOReader,
        # such as buttons getting stuck in the `hold` state
        pyautogui.mouseDown()
        time.sleep(0.2)
        pyautogui.mouseUp()
    
    async def wait_for_event(self, event_type: str, timeout: float = 15.0) -> dict:
        """
        Waits for a specific event from stdout with timeout.

        Args:
            event_type: The type of event to wait for
            timeout: Maximum time to wait in seconds

        Returns:
            The event message as a dictionary

        Raises:
            TimeoutError: If event is not received within timeout
        """
        try:
            async with asyncio.timeout(timeout):
                while True:
                    line = await self.process.stdout.readline() # type: ignore

                    try:
                        json_message = json.loads(line)
                        if json_message.get('type') == event_type:
                            return json_message
                    except:
                        continue
        except TimeoutError:
            raise TimeoutError(f"Timeout waiting for event: {event_type}")
