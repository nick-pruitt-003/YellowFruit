#!/usr/bin/env bash
# Stats parity check: TS app vs yf-core on every .yft under a folder.
# Usage: rust/parity.sh [folder]        (default: ~/Downloads/_QuizBowl)
# Needs node_modules (npm ci --ignore-scripts is enough) and cargo.
# Dumps contain real team/player names; they stay in rust/target/parity, which git ignores.
set -euo pipefail
cd "$(dirname "$0")/.."
SRC="${1:-$HOME/Downloads/_QuizBowl}"
OUT=rust/target/parity
rm -rf "$OUT"
mkdir -p "$OUT"
find "$SRC" -name '*.yft' -print0 | sort -z > "$OUT/files.list0"
npx tsx rust/oracle/dump.ts "$OUT" --list "$OUT/files.list0"
cargo build -q --manifest-path rust/Cargo.toml -p yf-parity
rust/target/debug/yf-parity dump "$OUT" --list "$OUT/files.list0"
rust/target/debug/yf-parity diff "$OUT"
