#!/bin/bash
# Quick restart Convex backend dev

cd "$(dirname "$0")"
pkill -f "convex-local-backend" || true
sleep 1
export PATH="/Users/izutanikazuki/.cargo/bin:$PATH"
cargo run --bin convex-local-backend