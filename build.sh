#!/usr/bin/env bash
TARGET=${1:-desktop}

set -ve
nix build ".#rakuyomi.$TARGET" -o build
