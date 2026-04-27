#!/bin/bash

set -eux

MODE=${1:-debug}

if [ "$MODE" = "release" ]; then
    cargo build -p cspuz_solver_backend --target wasm32-unknown-emscripten --release --no-default-features
elif [ "$MODE" = "debug" ]; then
    cargo build -p cspuz_solver_backend --target wasm32-unknown-emscripten --no-default-features
else
    echo "Invalid mode: $MODE"
    exit 1
fi

mkdir -p build/cspuz_solver_backend
cp target/wasm32-unknown-emscripten/${MODE}/deps/cspuz_solver_backend.js build/cspuz_solver_backend/
if [ -f target/wasm32-unknown-emscripten/${MODE}/deps/cspuz_solver_backend.wasm ]; then
    cp target/wasm32-unknown-emscripten/${MODE}/deps/cspuz_solver_backend.wasm build/cspuz_solver_backend/
elif [ -f target/wasm32-unknown-emscripten/${MODE}/cspuz_solver_backend.wasm ]; then
    cp target/wasm32-unknown-emscripten/${MODE}/cspuz_solver_backend.wasm build/cspuz_solver_backend/
else
    echo "cspuz_solver_backend.wasm was not found after build"
    find target/wasm32-unknown-emscripten/${MODE} -maxdepth 2 -type f | sort
    exit 1
fi
