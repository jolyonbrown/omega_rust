#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_dir"

CARGO_NET_OFFLINE=true "$HOME/.cargo/bin/cargo" build --release --target wasm32-unknown-unknown

mkdir -p web/dist
cp web/index.html web/mq_js_bundle.js web/dist/
cp target/wasm32-unknown-unknown/release/omega_rust.wasm web/dist/
