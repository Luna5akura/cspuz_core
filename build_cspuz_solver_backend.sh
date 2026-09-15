#!/bin/bash

set -eux

MODE=${1:-debug}

if [ "$MODE" = "release" ]; then
    cargo build -p cspuz_solver_backend --bin cspuz_solver_backend_wasm --target wasm32-unknown-emscripten --release --no-default-features
elif [ "$MODE" = "debug" ]; then
    cargo build -p cspuz_solver_backend --bin cspuz_solver_backend_wasm --target wasm32-unknown-emscripten --no-default-features
else
    echo "Invalid mode: $MODE"
    exit 1
fi

mkdir -p build/cspuz_solver_backend
if [ -f target/wasm32-unknown-emscripten/${MODE}/deps/cspuz_solver_backend_wasm.js ]; then
    cp target/wasm32-unknown-emscripten/${MODE}/deps/cspuz_solver_backend_wasm.js build/cspuz_solver_backend/cspuz_solver_backend.js
elif [ -f target/wasm32-unknown-emscripten/${MODE}/cspuz_solver_backend_wasm.js ]; then
    cp target/wasm32-unknown-emscripten/${MODE}/cspuz_solver_backend_wasm.js build/cspuz_solver_backend/cspuz_solver_backend.js
else
    echo "cspuz_solver_backend_wasm.js was not found after build"
    find target/wasm32-unknown-emscripten/${MODE} -maxdepth 2 -type f | sort
    exit 1
fi

if [ -f target/wasm32-unknown-emscripten/${MODE}/deps/cspuz_solver_backend_wasm.wasm ]; then
    cp target/wasm32-unknown-emscripten/${MODE}/deps/cspuz_solver_backend_wasm.wasm build/cspuz_solver_backend/cspuz_solver_backend.wasm
elif [ -f target/wasm32-unknown-emscripten/${MODE}/cspuz_solver_backend_wasm.wasm ]; then
    cp target/wasm32-unknown-emscripten/${MODE}/cspuz_solver_backend_wasm.wasm build/cspuz_solver_backend/cspuz_solver_backend.wasm
else
    echo "cspuz_solver_backend_wasm.wasm was not found after build"
    find target/wasm32-unknown-emscripten/${MODE} -maxdepth 2 -type f | sort
    exit 1
fi

# Keep the browser's local solver copy in sync when this script is run
# directly from cspuz_core (the dev command performs this copy itself, but a
# manual rebuild otherwise leaves pzprjs/dist/wasm on the previous backend).
if [ -d ../pzprjs/dist/wasm ]; then
    cp build/cspuz_solver_backend/cspuz_solver_backend.js ../pzprjs/dist/wasm/cspuz_solver_backend.js
    cp build/cspuz_solver_backend/cspuz_solver_backend.wasm ../pzprjs/dist/wasm/cspuz_solver_backend.wasm
fi
