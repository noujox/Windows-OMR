#!/usr/bin/env bash
set -euo pipefail
# Run in MSYS2 MINGW64 on Windows; Rust target x86_64-pc-windows-gnu.
# Native GTK4/libadwaita, FFmpeg, CMake and MinGW are supplied by MSYS2.
cd "$(dirname "$0")/.."
export PATH="/mingw64/bin:${PATH}"
export PKG_CONFIG_PATH="/mingw64/lib/pkgconfig"
export CMAKE_GENERATOR=Ninja
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu --no-default-features
echo "Binary: target/x86_64-pc-windows-gnu/release/omarchy-meeting-recorder.exe"
echo "This binary is NOT standalone; GTK, FFmpeg, and ONNX runtime packaging still require validation."
