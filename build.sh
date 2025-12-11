#!/bin/bash
set -e

cd "$(dirname "$0")"

echo "🔨 Building frontend..."
npm run build

echo ""
echo "🍎 Building for macOS..."
unset CI
npm run tauri build

echo ""
echo "🪟 Building for Windows (x64)..."
npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc --no-bundle

echo ""
echo "📦 Copying artifacts to release folder..."
rm -rf release
mkdir -p release

cp src-tauri/target/release/bundle/dmg/*.dmg release/
cp src-tauri/target/x86_64-pc-windows-msvc/release/marketeer.exe release/Marketeer.exe

echo ""
echo "✅ Build complete!"
echo ""
echo "Release artifacts:"
ls -lh release/


