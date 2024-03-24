#!/usr/bin/env python3
"""
Installs the plugin into a remote KOReader instance, using SCP to copy the
plugin files.
"""

from argparse import ArgumentParser
from pathlib import Path
import os
import subprocess

def main(host: str, ssh_port: int) -> None:
    DEVICE_TYPE = 'kindle'

    plugin_output_path = Path(subprocess.check_output(
        ['nix', 'build', f'.#rakuyomi.{DEVICE_TYPE}', '--print-out-paths', '--no-link']
    ).decode('utf-8').strip())

    REMOTE_OUTPUT_PATH = '/mnt/us/koreader/plugins/rakuyomi.koplugin/'

    subprocess.check_call([
        'ssh', '-p', str(ssh_port), f'root@{host}', f'rm -r {REMOTE_OUTPUT_PATH}'
    ])

    subprocess.check_call([
        'scp', '-r', '-P', str(ssh_port), plugin_output_path, f'root@{host}:/{REMOTE_OUTPUT_PATH}',
    ])

    print('Plugin successfully installed! Please restart KOReader on the target device.')

def create_argument_parser() -> ArgumentParser:
    parser = ArgumentParser()

    parser.add_argument('--host', **environ_or_required('REMOTE_KOREADER_HOST'))
    parser.add_argument('--ssh-port', type=int, **environ_or_required('REMOTE_KOREADER_SSH_PORT'))

    return parser

def environ_or_required(key: str):
    return (
        {'default': os.environ.get(key)} if os.environ.get(key)
        else {'required': True}
    )

if __name__ == '__main__':
    parser = create_argument_parser()
    args = parser.parse_args()

    main(**vars(args))
