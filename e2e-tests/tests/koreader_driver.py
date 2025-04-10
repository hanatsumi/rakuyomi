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
import sys
from collections import namedtuple
from pathlib import Path
from typing import TypeVar, Type
from pydantic import BaseModel, TypeAdapter
from PIL import ImageDraw

from tests.agent import Agent

logger = logging.getLogger(__name__)

T = TypeVar('T', bound=BaseModel)

WindowFrame = namedtuple('WindowFrame', ['left', 'top', 'right', 'bottom'])

class KOReaderDriver:
    def __init__(self, agent: Agent, koreader_home: Path):
        self.agent = agent
        self.koreader_home = koreader_home
        self.rakuyomi_home = koreader_home / 'rakuyomi'
        self.rakuyomi_home.mkdir(parents=True, exist_ok=True)
        self.ipc_socket_path = "/tmp/rakuyomi_testing_ipc.sock"
        self.reader = None
        self.writer = None

    async def __aenter__(self):
        """Context manager enter - starts the KOReader process."""
        # Set up IPC socket
        try:
            os.unlink(self.ipc_socket_path)
        except OSError:
            pass

        # Start server
        server = await asyncio.start_unix_server(
            self._handle_ipc_connection,
            path=self.ipc_socket_path,
            limit=512 * 1024,
        )
        self._server = server

        # Write KOReader's settings file
        koreader_settings = """
            return {
                ["color_rendering"] = true,
                ["quickstart_shown_version"] = 202407000289,
            }
        """

        with open(self.koreader_home / "settings.reader.lua", "w") as f:
            f.write(koreader_settings)

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
            stdout=sys.stderr,
            env={
                **os.environ,
                'RAKUYOMI_IS_TESTING': '1',
                'KO_HOME': self.koreader_home,
            },
            process_group=0
        )

        # Wait for KOReader to connect via IPC
        try:
            initialization_timeout = float(os.environ.get('RAKUYOMI_TEST_INITIALIZATION_TIMEOUT', '30'))

            async with asyncio.timeout(initialization_timeout):
                while not self.reader:
                    await asyncio.sleep(0.1)
                logger.info('KOReader connected to IPC socket')

                await self.wait_for_event('initialized', initialization_timeout)
        except TimeoutError:
            raise TimeoutError("Timeout waiting for IPC connection/initialization")

        # For some reason, sometimes KOReader decides to disable input handling on initialization.
        # Wait for a bit after initialization for hooks to work.
        await asyncio.sleep(1)
        logger.info('Waited for KOReader\'s user input handling to be enabled')

        self.window = next(w for w in pywinctl.getAllWindows() if w.title.endswith('KOReader'))
        self.window.moveTo(0, 0, wait=True)

        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit - cleans up the KOReader process."""
        if self.writer:
            self.writer.close()
            await self.writer.wait_closed()
        
        if hasattr(self, '_server'):
            self._server.close()
            await self._server.wait_closed()

        try:
            os.unlink(self.ipc_socket_path)
        except OSError:
            pass

        if hasattr(self, 'process') and self.process:
            os.killpg(os.getpgid(self.process.pid), signal.SIGTERM)
            await self.process.communicate()

    async def _handle_ipc_connection(self, reader: asyncio.StreamReader, writer: asyncio.StreamWriter):
        """Handles new IPC connections."""
        self.reader = reader
        self.writer = writer
        
    async def _send_ipc_command(self, command_type: str, params: dict | None = None):
        """Sends a command to KOReader via IPC."""
        if not self.writer:
            raise RuntimeError("No IPC connection available")
            
        message = {
            "type": command_type,
            "params": params or {}
        }
        self.writer.write(json.dumps(message).encode() + b'\n')
        await self.writer.drain()

    def activate_window(self):
        """Activates the KOReader window."""
        self.window.activate(wait=True)

    async def open_library_view(self):
        """Opens the library view in KOReader."""
        self.activate_window()
        pyautogui.hotkey('shift', 'f9')

        await self.wait_for_event('library_view_shown')

    async def request_ui_contents(self, timeout=15) -> str:
        """Requests and returns the current UI contents."""
        start = datetime.now()
        await self._send_ipc_command("dump_ui")

        event = await self.wait_for_event('ui_contents', timeout=timeout)
        ui_contents: str = event['params']['contents']
        duration = datetime.now() - start

        logger.info(f'Requested UI contents in {duration.total_seconds()}s, hash is {hashlib.sha256(ui_contents.encode()).hexdigest()}')

        return ui_contents
    
    async def screenshot(self, output: Path) -> None:
        """
        Takes a screenshot of the KOReader window and saves it to the specified output path.
        
        Args:
            output: Path to save the screenshot
        """
        self.activate_window()

        cursor_position = pyautogui.position()
        img = pyautogui.screenshot(None)
        draw = ImageDraw.Draw(img)
        radius = 10
        draw.ellipse([
            cursor_position.x - radius, 
            cursor_position.y - radius,
            cursor_position.x + radius, 
            cursor_position.y + radius
        ], fill='red')
        img.save(output)

    async def query(self, query: str, response_class: Type[T] | TypeAdapter[T], timeout=15) -> T:
        """
        Performs a query on the current UI contents and returns the response.
        
        Args:
            query: The query string to send to the agent
            response_class: The Pydantic model class to parse the response into
            timeout: How long to wait for the UI contents
            
        Returns:
            An instance of response_class containing the query results
        """
        ui_contents = await self.request_ui_contents(timeout=timeout)

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

        window_area = self._get_window_frame()
        x = window_area.left + window_offset_x
        y = window_area.top + window_offset_y

        logger.info(f'Clicking on {window_offset_x}x{window_offset_y} inside the window -> {x}x{y} real position (window frame: {window_area})')

        pyautogui.moveTo(x=x, y=y)
        # FIXME Using `click` here causes some weird bugs inside KOReader,
        # such as buttons getting stuck in the `hold` state
        pyautogui.click()
    
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

        if not self.reader:
            raise RuntimeError("No IPC connection available")

        try:
            async with asyncio.timeout(timeout):
                while True:
                    line = await self.reader.readline()

                    try:
                        json_message = json.loads(line)
                        if json_message.get('type') == event_type:
                            return json_message
                    except:
                        logger.debug(f'Couldn\'t parse line: "{line}"')
                        continue
        except TimeoutError:
            raise TimeoutError(f"Timeout waiting for event: {event_type}")
    
    def _get_window_frame(self) -> WindowFrame:
        if 'CI' in os.environ:
            # pywinctl fucks up when we're running under Fluxbox.
            # For now, hardcode the window dimensions when under CI.
            return WindowFrame(
                left=1,
                top=23,
                right=1 + 800,
                bottom=23 + 600,
            )

        frame = self.window.getClientFrame()
        return WindowFrame(
            left=frame.left,
            top=frame.top,
            right=frame.right,
            bottom=frame.bottom
        )

