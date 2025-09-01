#!/bin/bash

# Start Omni Node with the generated chain spec
echo "Starting PRMX Omni Node..."

# Check if polkadot-omni-node is installed
if ! command -v polkadot-omni-node &> /dev/null; then
    echo "Error: polkadot-omni-node not found. Please install it first:"
    echo "cargo install --locked polkadot-omni-node@0.7.0"
    exit 1
fi

# Check if chain spec exists
if [ ! -f "./chain_spec.json" ]; then
    echo "Error: chain_spec.json not found. Please build the runtime first:"
    echo "cargo build --profile production"
    exit 1
fi

# Start the Omni Node in dev mode (standalone)
echo "Starting Omni Node in dev mode on port 9944 (RPC/WebSocket)..."
polkadot-omni-node \
    --chain ./chain_spec.json \
    --dev \
    --name "prmx-omni-collator" \
    --rpc-port 9944 \
    --port 30333 \
    --rpc-cors all \
    --unsafe-rpc-external \
    --unsafe-force-node-key-generation \
    --base-path ./data \
    --log info
