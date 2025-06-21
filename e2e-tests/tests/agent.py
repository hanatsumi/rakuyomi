import curlify
from datetime import datetime
import hashlib
import inspect
import logging
import os
from pathlib import Path
import time
from typing import Any, overload
from pydantic import BaseModel, TypeAdapter

import requests
from requests.auth import AuthBase
from requests.adapters import Retry, HTTPAdapter
from requests.models import HTTPError

logger = logging.getLogger(__name__)

SYSTEM_PROMPT = '''
You are a software engineer specialized in UI testing. The UI contents are described in a Lua table format, inside `<ui_contents>` XML tags.
Your mission is to perform general actions and queries upon the described UI contents.

Queries will be sent inside a `<query>` XML tag, which will describe both the query contents and the expected JSON response schema. An example
query is:
<query>
  <contents>What mangas are listed in the current page?</contents>
  <response_schema>
    {
      "type": "object",
      "properties": {
        "mangas": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "name": {
                "type": "string"
              }
            },
            "required": [
              "name"
            ]
          }
        }
      },
      "required": [
        "mangas"
      ]
    }
  </response_schema>
</query>

The performed actions should be described as clicks and keyboard events that should be performed, such as:
- click on X and Y coordinates
- write "XYZ"

When clicking or attempting to find the area of a button, consider the associated element's padding as non-clickable.

Your replies should be in a valid JSON format.
'''

QUERY_TEMPLATE = '''
<ui_contents>{ui_contents}</ui_contents>

<query>
  <contents>{contents}</contents>
  <response_schema>{response_schema}</response_schema>
</query>
'''

class Agent:
    def __init__(self):
        class OpenAIAuth(AuthBase):
            def __init__(self, api_key):
                self.api_key = api_key

            def __call__(self, r):
                r.headers['Authorization'] = f'Bearer {self.api_key}'
                return r

        self.session = requests.Session()
        self.session.auth = OpenAIAuth(os.environ['OPENAI_API_KEY'])
        self.session.headers.update({'Content-Type': 'application/json'})
        retries = Retry(
            total=5,
            backoff_factor=1,
            status_forcelist=[429, 500, 502, 503, 504],
            allowed_methods=["POST"]
        )
        self.session.mount('https://', HTTPAdapter(max_retries=retries))

        self.base_url = os.environ['OPENAI_BASE_URL']
        self.model = os.environ['OPENAI_MODEL']

    @overload
    def query[T: BaseModel](self, ui_contents: str, query: str, response_class: type[T]) -> T:
        ...

    @overload
    def query[T](self, ui_contents: str, query: str, response_class: TypeAdapter[T]) -> T:
        ...

    def query(self, ui_contents: str, query: str, response_class: type[BaseModel] | TypeAdapter):
        start = datetime.now()
        schema = self._json_schema(response_class)
        response = self.session.post(
            f'{self.base_url}/chat/completions',
            json={
                'model': self.model,
                'messages': [
                    {'role': 'system', 'content': SYSTEM_PROMPT},
                    {
                      'role': 'user',
                      'content': QUERY_TEMPLATE.format(
                        ui_contents=ui_contents,
                        contents=query,
                        response_schema=schema
                      )
                    }
                ],
                'response_format': {
                    'type': 'json_object',
                },
                'temperature': 0.2,
                'stream': False
            },
        )

        duration = datetime.now() - start

        try:
            response.raise_for_status()
        except HTTPError as e:
            logger.error(f'HTTP error occurred: {e}')
            logger.error(f'Response content: {response.text}')
            raise

        response_json = response.json()

        screenshot_folder = Path('screenshots')
        screenshot_folder.mkdir(parents=True, exist_ok=True)

        screenshot_path = screenshot_folder / f'{hashlib.sha256(ui_contents.encode()).hexdigest()}.txt'
        with open(screenshot_path, 'wb') as f:
            f.write(ui_contents.encode())
            logger.info(f'UI contents saved to {screenshot_path}')

        logger.info(f'Performed query in {duration.total_seconds()}, response: {response.text}')

        return self._validate_json(
            response_class,
            response_json['choices'][0]['message']['content']
        )

    def _json_schema[T](self, response_class: type[BaseModel] | TypeAdapter) -> dict[str, Any]:
        if isinstance(response_class, type) and issubclass(response_class, BaseModel):
            return response_class.model_json_schema()

        return response_class.json_schema()

    def _validate_json[T](
        self,
        response_class: type[BaseModel] | TypeAdapter,
        json_data: str | bytes | bytearray
    ):
        if isinstance(response_class, type) and issubclass(response_class, BaseModel):
            return response_class.model_validate_json(json_data)

        return response_class.validate_json(json_data)
