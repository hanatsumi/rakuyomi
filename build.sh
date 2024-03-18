#!/usr/bin/env bash
set -ve

rm -r build
mkdir -p build

cp -r frontend/rakuyomi.koplugin build/

pushd backend
devbox run cargo build --package lua_module
popd

mkdir -p build/rakuyomi.koplugin/lib
cp backend/target/debug/liblua_module.so build/rakuyomi.koplugin/lib/backend.so
