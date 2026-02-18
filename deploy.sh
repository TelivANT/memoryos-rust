#!/bin/bash
set -e

echo "🚀 Starting Deployment on Remote Server..."

# 1. Install Rust (if missing)
if ! command -v cargo &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# 2. Start Infrastructure (Docker)
echo "Starting Infrastructure (Redis + Qdrant)..."
if [ ! -f docker-compose.yml ]; then
    cat <<EOF > docker-compose.yml
version: '3'
services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    command: redis-server --appendonly yes

  qdrant:
    image: qdrant/qdrant:latest
    ports:
      - "6333:6333"
      - "6334:6334"
    volumes:
      - ./qdrant_data:/qdrant/storage
EOF
fi

if command -v docker-compose &> /dev/null; then
    docker-compose up -d
else
    docker compose up -d
fi

# 3. Build & Run
echo "Building MemoryOS-Rust..."
cargo build --release --bin memoryos-gateway

echo "🚀 Starting Gateway..."
./target/release/memoryos-gateway
