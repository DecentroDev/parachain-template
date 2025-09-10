#!/bin/bash
# Run the custom runtime as a standalone chain (no relay chain needed)

set -e

echo "🚀 Starting standalone node with your custom pallets..."

# Kill any existing processes
pkill -f polkadot-omni-node || true
pkill -f parachain-template-node || true

# Generate latest runtime spec
echo "📦 Building latest runtime spec..."
./parachain-template-node build-spec --chain dev --raw > standalone_runtime.json

# Start standalone node
echo "🔗 Starting standalone node on ws://127.0.0.1:9944..."
./polkadot-omni-node \
  --chain standalone_runtime.json \
  --dev \
  --unsafe-rpc-external \
  --rpc-cors all \
  --rpc-port 9944 \
  --tmp &

NODE_PID=$!

echo ""
echo "✅ Standalone node running!"
echo ""

# Wait for node to be ready and add keys for offchain workers
echo "🔑 Adding keys for DAO offchain workers..."
sleep 3
curl -s -H "Content-Type: application/json" \
     -d '{"id":1, "jsonrpc":"2.0", "method": "author_insertKey", "params":["dao", "//Alice", "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"]}' \
     http://localhost:9944 > /dev/null && echo "✅ Alice's key added for DAO signing" || echo "⚠️  Key insertion failed (node may not be ready yet)"

echo ""
echo "🔗 Connect Polkadot.js Apps to: ws://127.0.0.1:9944"
echo "📋 Your custom pallets available:"
echo "   - Insurances pallet"
echo "   - DAO pallet (with offchain workers)"
echo "   - PayoutProcessor pallet"
echo "   - Collective pallet"
echo ""
echo "📊 Check Developer → Chain State to see your pallets"
echo "⚡ Check Developer → Extrinsics to call pallet functions"
echo ""
echo "Press Ctrl+C to stop the node"

# Cleanup on exit
trap "kill $NODE_PID 2>/dev/null || true" EXIT

wait
