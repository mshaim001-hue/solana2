#!/usr/bin/env bash
# Source this in your shell before build/test/deploy:
#   source scripts/dev-env.sh
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

export PATH="$ROOT/.tools/solana-release/bin:${CARGO_HOME:-$HOME/.cargo}/bin:$HOME/.avm/bin:$HOME/.cargo/bin:$PATH"

# Prefer workspace-local cargo/rustup if present (Cursor sandbox installs)
if [[ -d "$ROOT/.cargo-home/bin" ]]; then
  export CARGO_HOME="${CARGO_HOME:-$ROOT/.cargo-home}"
  export PATH="$CARGO_HOME/bin:$PATH"
fi
if [[ -d "$ROOT/.tools/rustup" ]]; then
  export RUSTUP_HOME="${RUSTUP_HOME:-$ROOT/.tools/rustup}"
fi

echo "solana: $(command -v solana || echo MISSING)"
echo "cargo-build-sbf: $(command -v cargo-build-sbf || echo MISSING)"
echo "anchor: $(command -v anchor || echo MISSING — install with: cargo install --git https://github.com/coral-xyz/anchor avm --locked --force && avm install latest && avm use latest)"
echo "node_modules: $([[ -d $ROOT/node_modules/@solana/web3.js ]] && echo ok || echo MISSING — run: npm install)"
