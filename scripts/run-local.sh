#!/bin/bash
# Run local relay + collator for testing

set -e

# Kill existing processes on 9944/9933
pkill -f polkadot-omni-n || true
pkill -f polkadot || true
pkill -f parachain-template-node || true

echo "Starting local relay chain on ws://127.0.0.1:9955..."
polkadot --dev --no-telemetry --tmp \
  --experimental-rpc-endpoint listen-addr=127.0.0.1:9955,cors=all &

RELAY_PID=$!
sleep 3

echo "Generating parachain chainspec..."
./target/release/parachain-template-node build-spec --chain dev --disable-default-bootnode > /tmp/para.json

echo "Starting parachain collator on ws://127.0.0.1:9944..."
./target/release/parachain-template-node \
  --chain /tmp/para.json \
  --no-telemetry --tmp \
  --experimental-rpc-endpoint listen-addr=127.0.0.1:9944,cors=all \
  --relay-chain-rpc-urls ws://127.0.0.1:9955 &

PARA_PID=$!

echo "✅ Local stack running:"
echo "   Relay: ws://127.0.0.1:9955"
echo "   Parachain: ws://127.0.0.1:9944"
echo "   Connect Polkadot.js Apps to: ws://127.0.0.1:9944"

# Cleanup on exit
trap "kill $RELAY_PID $PARA_PID 2>/dev/null || true" EXIT

wait
