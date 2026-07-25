#!/usr/bin/env bash
# flash.sh — build + flash an SDK example.
# usage: scripts/flash.sh <example> [ap|sta|none]
set -euo pipefail

EXAMPLE="${1:?usage: $0 <example> [ap|sta|none]}"
MODE="${2:-none}"
PORT="${ESP_S3_PORT:-/dev/ttyACM0}"
FLAGS="--min-chip-rev 0.0 --port ${PORT}"
BUILD_FLAGS="-Zbuild-std=core,compiler_builtins,alloc"

cd "$(dirname "$0")/.."
source ~/export-esp.sh 2>/dev/null || true

case "$MODE" in
	ap)  FEATURES="--features wifi-ap" ;;
	sta) FEATURES="--no-default-features --features wifi-sta" ;;
	none) FEATURES="--no-default-features" ;;
	*) echo "usage: $0 <example> [ap|sta|none]"; exit 1 ;;
esac

echo ">> build example: $EXAMPLE ($MODE)"
cargo +esp build --release $FEATURES --example "$EXAMPLE" $BUILD_FLAGS
BIN="target/xtensa-esp32s3-none-elf/release/examples/$EXAMPLE"

echo ">> flash $BIN"
espflash flash $FLAGS "$BIN"
