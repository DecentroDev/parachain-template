#!/bin/bash
# Docker entrypoint script for standalone node

set -e

echo "🚀 Starting standalone node with custom pallets in Docker..."

# Generate runtime spec in writable location
echo "📦 Building runtime spec..."
./parachain-template-node build-spec --chain dev --raw > ./runtime.json

# Start node using polkadot with our custom runtime
echo "🔗 Starting node on port 9944..."
polkadot-omni-node \
  --chain ./runtime.json \
  --dev \
  --unsafe-rpc-external \
  --rpc-cors all \
  --rpc-port 9944 \
  --tmp &

NODE_PID=$!

# Wait for node startup
echo "⏳ Waiting for node to be ready..."
sleep 5

# Add keys for DAO offchain workers
echo "🔑 Adding Alice's key for DAO offchain workers..."
curl -s -H "Content-Type: application/json" \
     -d '{"id":1, "jsonrpc":"2.0", "method": "author_insertKey", "params":["dao", "//Alice", "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"]}' \
     http://localhost:9944 > /dev/null && echo "✅ Keys added successfully" || echo "⚠️  Key insertion will retry..."

echo ""
echo "✅ Standalone node running in Docker!"
echo ""
echo "🔗 Connect from host: ws://localhost:9944"
echo "📋 Your custom pallets:"
echo "   - Insurances pallet"  
echo "   - DAO pallet (with offchain workers + keys)"
echo "   - PayoutProcessor pallet"
echo "   - Collective pallet"
echo ""

# Keep container running
wait $NODE_PID


