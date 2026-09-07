#!/bin/sh
set -eu
API_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
rustup target add wasm32-unknown-unknown
cargo build --release --locked --target wasm32-unknown-unknown --manifest-path "$API_ROOT/web-engine/Cargo.toml"
wasm-bindgen "$API_ROOT/web-engine/target/wasm32-unknown-unknown/release/noroc_web_engine.wasm" --target web --out-dir "$API_ROOT/web/engine"
