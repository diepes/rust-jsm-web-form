#!/bin/bash
# Build script for JSM Form Automation Tool

echo "Building JSM Form Automation Tool..."

# Clean previous build
cargo clean

# Build in release mode
cargo build --release

if [ $? -eq 0 ]; then
    echo "Build successful!"
    echo "Run './target/release/jsm_form --help' for usage information"
    echo ""
    echo "Quick start:"
    echo "1. ./target/release/jsm_form init"
    echo "2. Edit jsm_config.pvt.toml with your credentials"
    echo "3. ./target/release/jsm_form submit -d 'summary=Test' -d 'description=Test description'"
else
    echo "Build failed!"
    exit 1
fi