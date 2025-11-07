#!/bin/bash
set -e

echo "Building FuckScanDrive..."
echo ""

echo "[1/3] Building Hook DLL..."
cargo build --release --manifest-path hook_dll/Cargo.toml

echo ""
echo "[2/3] Copying Hook DLL to target directory..."
cp hook_dll/target/release/fuck_scan_hook.dll target/release/ || \
cp hook_dll/target/release/libfuck_scan_hook.dylib target/release/ 2>/dev/null || true

echo ""
echo "[3/3] Building main application..."
cargo build --release

echo ""
echo "========================================"
echo "Build completed successfully!"
echo "========================================"
echo ""
echo "Output files:"
echo "  - target/release/fuck_scan_drive.exe"
echo "  - target/release/fuck_scan_hook.dll"
echo ""
echo "Don't forget to configure fuck.ini before running!"
echo ""
