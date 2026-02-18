#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

echo "[eval-extraction] Running extraction dataset scorer..."
cargo test -p memoryos-adapters extraction_eval_dataset_report -- --nocapture
echo "[eval-extraction] Done."
