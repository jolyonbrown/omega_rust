#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_dir"

run_capture() {
  local name="$1"
  local frames="$2"
  local shot_every="$3"
  local script="verify/${name}.script"
  local output="verify/out/${name}"

  rm -rf "$output"
  mkdir -p "$output"
  CARGO_NET_OFFLINE=true cargo run --release -- \
    --headless \
    --frames "$frames" \
    --shot-every "$shot_every" \
    --out "$output" \
    --seed 0x4f4d454741525553 \
    --script "$script"
}

run_capture flight 300 30
run_capture attract 1200 120
run_capture wave1 720 15
run_capture death 3000 30
