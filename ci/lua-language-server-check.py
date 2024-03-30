#!/usr/bin/env python3
from argparse import ArgumentParser
from pathlib import Path
from dataclasses import dataclass
from tempfile import TemporaryDirectory
from typing import Any, Dict, List, Tuple
import json
import os
import shutil
import sys 
import subprocess

@dataclass
class Location:
    line: int
    character: int

    @staticmethod
    def from_json(json: Dict[Any, Any]) -> 'Location':
        return Location(
            line=json['line'],
            character=json['character'],
        )

@dataclass
class Diagnostic:
    code: str
    message: str
    range: Tuple[Location, Location]

    def from_json(json: Dict[Any, Any]) -> 'Diagnostic':
        range_start = Location.from_json(json['range']['start'])
        range_end = Location.from_json(json['range']['end'])

        return Diagnostic(
            code=json['code'],
            message=json['message'],
            range=(range_start, range_end),
        )

def main(project_path: Path) -> None: 
    file_diagnostics = collect_diagnostics(project_path)

    if len(file_diagnostics) > 0:
        print(f'❌ Found problems in {len(file_diagnostics)} files:')
    else:
        print('✔️ No problems found!')

    for file, diagnostics in file_diagnostics.items():
        print(f'📄 {str(file)}:')

        for diagnostic in diagnostics:
            print(
                f'- {diagnostic.range[0].line}:{diagnostic.range[0].character}'
                '-'
                f'{diagnostic.range[1].line}:{diagnostic.range[1].character}'
                ': '
                f'{diagnostic.message} ({diagnostic.code})'
            )
        
        print()
    
    exit_code = 2 if len(file_diagnostics) > 0 else 0
    sys.exit(exit_code)

def collect_diagnostics(project_path: Path) -> Dict[Path, List[Diagnostic]]:
    with TemporaryDirectory() as temporary_dir:
        temporary_dir_path = Path(temporary_dir)
        log_path = temporary_dir_path / 'check.json'
        luarc_ci_path = project_path / '.luarc.ci.json'

        project_copy_target = temporary_dir_path / project_path.name
        luarc_copy_target = temporary_dir_path / project_path.name / '.luarc.json'

        shutil.copytree(project_path, project_copy_target)
        shutil.copy(luarc_ci_path, luarc_copy_target)

        subprocess.check_call([
            'lua-language-server',
            '--check', project_copy_target,
            '--logpath', str(temporary_dir_path)
        ], stdout=subprocess.DEVNULL)

        # No diagnostics found, everything is OK.
        if not log_path.exists():
            return {}

        with open(log_path) as log:
            json_file_diagnostics: Dict[str, Dict[Any, Any]] = json.load(log)

            return {
                Path(source_path.removeprefix('file://')).relative_to(temporary_dir_path): [Diagnostic.from_json(json_diagnostic) for json_diagnostic in json_diagnostics]
                for source_path, json_diagnostics in json_file_diagnostics.items()
            }

def create_argument_parser() -> ArgumentParser:
    parser = ArgumentParser()
    parser.add_argument('project_path', type=Path)

    return parser

if __name__ == '__main__':
    parser = create_argument_parser()
    args = parser.parse_args()

    main(**vars(args))
