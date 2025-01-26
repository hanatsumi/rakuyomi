import json
import os
import logging
import subprocess
import pyautogui
import pywinctl
import time

logging.basicConfig(level=logging.DEBUG)

from agent import Agent
from pydantic import BaseModel

class Manga(BaseModel):
    name: str
    source: str

class MangaQueryResponse(BaseModel):
    mangas: list[Manga]

class PageResponse(BaseModel):
    title: str

koreader_process = subprocess.Popen(
    ['devbox', 'run', 'dev'],
    stdout=subprocess.PIPE,
    env={
        **os.environ,
        'RAKUYOMI_IS_TESTING': '1',
    }
)

def request_ui_contents() -> str:
    koreader_window = next(w for w in pywinctl.getAllWindows() if w.title.endswith('KOReader'))
    koreader_window.activate()

    pyautogui.hotkey('shift', 'f8')

    while True:
        line = koreader_process.stdout.readline()

        try:
            json_message = json.loads(line)
            if json_message.get('type') == 'ui_contents':
                return json_message['contents']
        except:
            pass

time.sleep(30)

ui_contents = request_ui_contents()
print('Got UI contents:', ui_contents)

raise Exception('Stop here')

agent = Agent()

response = agent.query(
    ui_contents=ui_contents,
    query="What is the current page?",
    response_class=PageResponse
)

print(response)
p
response = agent.query(
    ui_contents=ui_contents,
    query="What mangas are listed in the current page?",
    response_class=MangaQueryResponse
)

print(response)