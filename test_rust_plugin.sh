#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PARENT_DIR="$(dirname "$SCRIPT_DIR")"
RUST_DLL="$SCRIPT_DIR/target/x86_64-pc-windows-gnu/release/plugin.dll"

source "$PARENT_DIR/.env" 2>/dev/null || true

GAME_DIR="$WINEPREFIX/drive_c/Program Files/Invant Inc/SquareGolf HE/SquareGolf HE_Data/Plugins/x86_64"
ORIGINAL_DLL="$GAME_DIR/plugin.dll"
BACKUP_DLL="$GAME_DIR/plugin.dll.original"

echo "=== Rust Plugin Test Script ==="

# Build the Rust plugin
echo "[1/4] Building Rust plugin..."
cd "$SCRIPT_DIR"
cargo build --release 2>&1 | tail -5

if [ ! -f "$RUST_DLL" ]; then
    echo "ERROR: Build failed - no DLL produced"
    exit 1
fi

# Backup original if not already done
if [ -f "$ORIGINAL_DLL" ] && [ ! -f "$BACKUP_DLL" ]; then
    echo "[2/4] Backing up original plugin.dll..."
    cp "$ORIGINAL_DLL" "$BACKUP_DLL"
    echo "    Backed up to: $BACKUP_DLL"
elif [ -f "$BACKUP_DLL" ]; then
    echo "[2/4] Original already backed up"
else
    echo "[2/4] WARNING: No original plugin.dll found"
fi

# Copy Rust DLL
echo "[3/4] Installing Rust plugin.dll..."
cp "$RUST_DLL" "$ORIGINAL_DLL"
echo "    Installed: $(ls -la "$ORIGINAL_DLL" | awk '{print $5, "bytes"}')"

# Run wine test
echo "[4/4] Running Wine test..."
echo "========================================"
cd "$PARENT_DIR/scripts"
./run_wine_test.sh "$@"
