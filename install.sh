#!/usr/bin/env bash
set -euo pipefail

REPO_URL="https://github.com/WesselBadenhorst/seed-cli.git"
TMP_DIR="$(mktemp -d)"

cleanup() { rm -rf "$TMP_DIR"; }
trap cleanup EXIT

echo "cloning seed-cli..."
git clone --depth 1 "$REPO_URL" "$TMP_DIR/seed-cli"

echo "building and installing..."
cargo install --path "$TMP_DIR/seed-cli"

echo ""
echo "seed installed successfully"
echo "run 'seed --help' to get started"
