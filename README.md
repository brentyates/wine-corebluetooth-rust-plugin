# SquareGolf Bluetooth Plugin (Rust)

A drop-in replacement for the `plugin.dll` used in **SquareGolf**, written in Rust.

## Purpose

This project provides a specialized Bluetooth Low Energy (BLE) plugin designed to run SquareGolf under **Wine** (macOS/Linux). It replaces the original game DLL to resolve critical stability and compatibility issues that occur when bridging Unity's BLE calls through the Wine emulation layer.

### Key Solved Issues
1.  **Double-Pointer Marshaling**: The original game employs a specific (and somewhat fragile) memory marshaling pattern for reading characteristic data, often treating buffers as pointers-to-pointers. This implementation detects this pattern and handles the dereferencing safely, preventing Segfaults/Access Violations common in the original DLL under Wine.
2.  **UI Freezing**: Bluetooth operations (Connecting, Writing, Subscribing) are offloaded to background threads to prevent the Unity Main Thread from locking up while Wine initializes the CoreBluetooth bridge or performs I/O.
3.  **Specific Filtering**: The scanner is hardcoded to prioritize and filter for "SquareGolf" devices to ensure reliable discovery in noisy environments.

## Architecture

*   **Language**: Rust (targeting `x86_64-pc-windows-gnu`)
*   **ABI**: C-compatible DLL exports (`_connect`, `_read`, `_write`, etc.) matching the specific signature expected by the SquareGolf Unity client.
*   **Backend**: Uses the `windows` crate to interface with the Windows Runtime (WinRT) `Windows.Devices.Bluetooth` APIs, which Wine translates to the host OS's Bluetooth stack (e.g., CoreBluetooth on macOS).

## Building

You need a Rust toolchain with the Windows GNU target installed.

```bash
# Add the target
rustup target add x86_64-pc-windows-gnu

# Build release DLL
cargo build --release
```

The output file will be located at `target/x86_64-pc-windows-gnu/release/plugin.dll`.

## Installation

1.  Locate your SquareGolf installation directory.
2.  Navigate to `SquareGolf HE_Data/Plugins/x86_64`.
3.  Backup the existing `plugin.dll` (e.g., rename to `plugin.dll.bak`).
4.  Copy the built `plugin.dll` from this project into that folder.

## Usage

This plugin is specific to the hardware UUIDs and logic of the SquareGolf hardware. 
*   **Auto-Subscribe**: It automatically subscribes to notifications for the specific characteristic UUID `86602102...` upon discovery.
*   **Device Matching**: Connects to devices advertising the local name "SquareGolf".
