import os
import json
from pydantic import BaseModel

from dataclasses_jsonschema import JsonSchemaMixin
from openai import OpenAI
import requests
from requests.auth import AuthBase

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

        self.base_url = os.environ['OPENAI_BASE_URL']

    def query[T: BaseModel](self, ui_contents: str, query: str, response_class: type[T]) -> T:
        response = self.session.post(
            f'{self.base_url}/chat/completions',
            json={
                'model': 'deepseek-chat',
                'messages': [
                    {'role': 'system', 'content': SYSTEM_PROMPT},
                    {
                      'role': 'user',
                      'content': QUERY_TEMPLATE.format(
                        ui_contents=ui_contents,
                        contents=query,
                        response_schema=response_class.model_json_schema()
                      )
                    }
                ],
                'response_format': {
                    'type': 'json_object',
                },
                'stream': False
            }
        )

        print(response.text)

        response.raise_for_status()
        response_json = response.json()

        return response_class.model_validate_json(
            response_json['choices'][0]['message']['content']
        )

    def reset(self):
        self.client.reset()