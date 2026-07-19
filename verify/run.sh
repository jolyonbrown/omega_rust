#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_dir"

mkdir -p verify/out
rm -f verify/out/frame_*.png
CARGO_NET_OFFLINE=true cargo run --release -- \
  --headless \
  --frames 300 \
  --shot-every 30 \
  --out verify/out \
  --seed 0x4f4d454741525553 \
  --script verify/flight.script
