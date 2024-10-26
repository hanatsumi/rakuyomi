#!/usr/bin/env bash

export RAKUYOMI_SERVER_COMMAND_OVERRIDE="$(which cargo) run --manifest-path backend/Cargo.toml -p server --"
export RAKUYOMI_SERVER_WORKING_DIRECTORY="$(pwd)"
export RAKUYOMI_SERVER_STARTUP_TIMEOUT="600"

exec nix run .#rakuyomi.koreader-with-plugin
