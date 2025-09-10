#!/bin/bash
# Run the standalone chain using Docker

set -e

echo "🐳 Building and starting standalone node with Docker..."

# Ensure we have the latest runtime spec
echo "📦 Building latest runtime spec..."
./target/release/parachain-template-node build-spec --chain dev --raw > standalone_runtime.json

# Build and start with docker-compose
echo "🔨 Building Docker image..."
docker-compose -f docker-compose.standalone.yml build

echo "🚀 Starting standalone node..."
docker-compose -f docker-compose.standalone.yml up

echo ""
echo "✅ Access your node:"
echo "🔗 Polkadot.js Apps (built-in): http://localhost:3000"
echo "🔗 RPC endpoint: ws://localhost:9944"
echo "📊 Prometheus metrics: http://localhost:9615"
echo ""
echo "Press Ctrl+C to stop all services"





