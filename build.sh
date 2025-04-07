#!/bin/bash
set -e

# Build the Rust library
cargo build --release

# Copy the compiled library to the lua directory
\cp ./target/release/libaichat_nvim.so /home/ricardo/.config/nvim/lua/aichat_nvim.so

echo "Plugin built successfully"

