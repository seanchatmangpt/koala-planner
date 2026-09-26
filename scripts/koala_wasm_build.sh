#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cargo build \
  --manifest-path "$repo_root/crates/koala-wasm/Cargo.toml" \
  --target wasm32-wasip1 \
  --release

artifact="$repo_root/crates/koala-wasm/target/wasm32-wasip1/release/koala_wasm.wasm"

if [[ ! -s "$artifact" ]]; then
  echo "expected WASM artifact missing: $artifact" >&2
  exit 1
fi

printf '%s\n' "$artifact"
