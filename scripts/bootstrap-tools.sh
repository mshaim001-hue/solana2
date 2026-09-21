#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "==> npm install"
npm install

if ! command -v cargo >/dev/null; then
  echo "Install Rust first: https://rustup.rs"
  exit 1
fi

if ! command -v avm >/dev/null; then
  echo "==> installing avm (Anchor Version Manager)"
  cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
fi

# Match program's Anchor 1.2 if available, else latest
echo "==> installing Anchor via avm"
avm install latest
avm use latest

if ! command -v solana >/dev/null; then
  echo "==> installing Solana CLI"
  sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
  export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
  echo 'export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"' >> ~/.zshrc
fi

echo
echo "OK. Open a new terminal (or source ~/.zshrc), then run:"
echo "  anchor --version"
echo "  solana --version"
echo "  anchor build"
echo "  anchor test"
