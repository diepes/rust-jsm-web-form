#!/bin/bash

echo "=== JSM Form Tool - Credential Prompting Demo ==="
echo

echo "1. First, let's create a fresh config:"
cargo run -- init --config demo_config.toml

echo
echo "2. Created config file contents:"
cat demo_config.toml

echo
echo "3. Now let's test the credential prompting (this will ask for username/password):"
echo "   Note: You can use fake credentials for this demo - it will fail at authentication"
echo "   but you'll see the prompting functionality working."
echo
echo "Run: cargo run -- submit --config demo_config.toml -d \"summary=Test Issue\" "
echo

# Clean up demo file
rm -f demo_config.toml