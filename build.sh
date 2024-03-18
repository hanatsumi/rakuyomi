#!/usr/bin/env bash
set -ve

rm -r build
mkdir -p build

cp -r frontend/rakuyomi.koplugin build/

pushd backend
devbox run cargo build --package server
popd

mkdir -p build/rakuyomi.koplugin
cp backend/target/debug/server build/rakuyomi.koplugin/server
