#!/usr/bin/env bash
set -euo pipefail
# Run in MSYS2 UCRT64 on Windows (GTK4/libadwaita packages are UCRT64).
cd "$(dirname "$0")/.."
export PATH="/ucrt64/bin:$(cygpath -u "$USERPROFILE")/.cargo/bin:${PATH}"
export PKG_CONFIG_PATH="/ucrt64/lib/pkgconfig:/ucrt64/share/pkgconfig"
export PKG_CONFIG_ALLOW_CROSS=1
export CMAKE_GENERATOR=Ninja
export LIBCLANG_PATH="/ucrt64/bin"
rustup target add x86_64-pc-windows-gnu
cargo build --locked --release --target x86_64-pc-windows-gnu --no-default-features
echo "Binary: target/x86_64-pc-windows-gnu/release/omarchy-meeting-recorder.exe"
echo "The binary needs GTK, FFmpeg and AI runtime DLLs before distribution."
