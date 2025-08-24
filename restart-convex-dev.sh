#!/bin/bash

# Convex Backend Development Environment Restart Script

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "🔄 Restarting Convex Backend Development Environment..."

# Kill existing Convex processes
echo "🛑 Stopping existing Convex processes..."
pkill -f "convex-local-backend" || true
pkill -f "convex dev" || true
pkill -f "convex mcp" || true

# Wait for processes to terminate
sleep 2

# Ensure correct Rust toolchain
export PATH="/Users/izutanikazuki/.cargo/bin:$PATH"

# Check if we need to build dependencies first
if [ ! -d "npm-packages/node_modules" ]; then
    echo "📦 Installing npm dependencies..."
    cd npm-packages
    ../scripts/node_modules/.bin/rush install
    cd ..
fi

# Start the local backend
echo "🚀 Starting Convex local backend..."
cargo run --bin convex-local-backend &
BACKEND_PID=$!

# Wait a moment for backend to start
sleep 3

# Check if backend started successfully
if ps -p $BACKEND_PID > /dev/null; then
    echo "✅ Convex backend started successfully (PID: $BACKEND_PID)"
    echo "📡 Backend listening on http://localhost:3210"
    echo "💾 Storage directory: $(pwd)/convex_local_storage"
    echo ""
    echo "To stop the backend, run: kill $BACKEND_PID"
    echo "Or use: pkill -f convex-local-backend"
else
    echo "❌ Failed to start Convex backend"
    exit 1
fi