#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked --bin tournament-manager-rs
app_dir="${TMRS_PACKAGE_DIR:-$PWD/dist}/Tournament Manager Rust.app"
mkdir -p "$app_dir/Contents/MacOS" "$app_dir/Contents/Resources/Sounds"
clang -mmacosx-version-min=12.0 -fobjc-arc -O2 -framework AppKit -framework CoreText -framework CoreGraphics native/report_pdf.m -o "$app_dir/Contents/Resources/report-pdf"
cp frontend/fonts/ReadexPro.ttf "$app_dir/Contents/Resources/ReadexPro.ttf"
cp frontend/fonts/OFL.txt "$app_dir/Contents/Resources/font-OFL.txt"
cp assets/v5field-darwin-amd64.so "$app_dir/Contents/Resources/v5field-darwin-amd64.so"
codesign --force --sign - "$app_dir/Contents/Resources/report-pdf"
codesign --force --sign - "$app_dir/Contents/Resources/v5field-darwin-amd64.so"
cp assets/Sounds/*.wav "$app_dir/Contents/Resources/Sounds/"
cp "${CARGO_TARGET_DIR:-target}/debug/tournament-manager-rs" "$app_dir/Contents/MacOS/tournament-manager-rs"
clang -arch x86_64 -mmacosx-version-min=10.13 -std=c11 -O2 -o "$app_dir/Contents/Resources/v5-bridge" native/v5_bridge.c
cat > "$app_dir/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Tournament Manager Rust</string>
<key>CFBundleDisplayName</key><string>Tournament Manager Rust</string>
<key>CFBundleIdentifier</key><string>local.tmrs.proof-of-concept</string>
<key>CFBundleExecutable</key><string>tournament-manager-rs</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.3.0</string>
<key>CFBundleVersion</key><string>3</string>
<key>NSHighResolutionCapable</key><true/>
<key>LSMinimumSystemVersion</key><string>12.0</string>
<key>NSLocalNetworkUsageDescription</key><string>Connect to tournament servers and displays on your local network.</string>
<key>NSAppTransportSecurity</key><dict><key>NSAllowsLocalNetworking</key><true/><key>NSAllowsArbitraryLoadsInWebContent</key><true/></dict>
</dict></plist>
PLIST
codesign --force --sign - "$app_dir/Contents/Resources/v5-bridge"
codesign --force --sign - "$app_dir"
codesign --verify --deep --strict "$app_dir"
ditto -c -k --sequesterRsrc --keepParent "$app_dir" "$(dirname "$app_dir")/Tournament-Manager-Tauri-macOS.zip"
printf 'Packaged: %s\n' "$app_dir"
