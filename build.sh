#!/bin/bash
set -e

cd "$(dirname "$0")"

echo "=== Building Rust plugin.dll ==="

# Ensure target is installed
rustup target add x86_64-pc-windows-gnu 2>/dev/null || true

# Build release
cargo build --release

# Copy to output
cp target/x86_64-pc-windows-gnu/release/plugin.dll ./plugin.dll 2>/dev/null || \
    cp target/release/plugin.dll ./plugin.dll 2>/dev/null || \
    echo "Note: DLL will be in target directory"

echo "=== Build complete ==="
ls -la plugin.dll 2>/dev/null || ls -la target/*/release/plugin.dll 2>/dev/null || true
