# Parachain Setup Solution

## Current Status ✅

Your custom pallets have been successfully migrated to FRAME v5:

- ✅ **Insurances pallet**: Fully working, visible in Polkadot.js Apps Chain State
- ✅ **Payout Processor pallet**: Successfully migrated  
- ✅ **DAO pallet**: Successfully migrated with complex offchain worker functionality
- ✅ **Collective pallet**: Standard FRAME pallet integrated

## Issue with Current Node Configuration

The `parachain-template-node` is hardcoded to connect to `rococo-local` relay chain, but wasn't built with the `rococo-native` feature enabled. This causes the connection error you're seeing.

## Solution Options

### Option 1: Standalone Node for Testing (Recommended) ✅ WORKING

Run the node as a standalone chain using polkadot-omni-node:

```bash
# Simple one-command solution
./run-standalone.sh

# Or manually:
./target/release/parachain-template-node build-spec --chain dev --raw > standalone_runtime.json
polkadot-omni-node --chain standalone_runtime.json --dev --unsafe-rpc-external --rpc-cors all --rpc-port 9944 --tmp
```

**Status**: ✅ **WORKING NOW!** 
- All your pallets are active and visible
- Offchain workers are running (DAO pallet) 
- Connect to `ws://127.0.0.1:9944` in Polkadot.js Apps

### Option 2: Build with Rococo Feature

```bash
# Rebuild with rococo-native feature
cargo build --release --features rococo-native

# Then use your existing relay+collator setup
```

### Option 3: Use Polkadot-Omni-Node (Alternative)

Since you want both standalone testing AND Paseo deployment:

```bash
# Generate runtime WASM
./target/release/parachain-template-node build-spec --raw --chain dev > custom_runtime.json

# Extract just the runtime
cat custom_runtime.json | jq '.genesis.runtimeGenesis.code' > runtime.wasm

# Run with omni-node
polkadot-omni-node --chain custom_runtime.json --dev --rpc-external --rpc-cors all
```

## For Paseo Deployment

I've prepared `docker-compose.paseo.yml` with environment variables:

```bash
# Set environment variables
export PASEO_RPC_URL="wss://rpc.ibp.network/paseo"
export NODE_NAME="your-collator-name"

# Deploy to Paseo
docker-compose -f docker-compose.paseo.yml up
```

## Verifying Your Pallets

Connect Polkadot.js Apps to `ws://127.0.0.1:9944` and check:

1. **Developer → Chain State**: All your pallets (Insurances, Dao, PayoutProcessor, Collective) should be visible
2. **Developer → Extrinsics**: Dispatchable functions should be available
3. **Network → Explorer**: Transaction history and events

## Your Pallets Are Working! 🎉

The migration is complete. Your custom pallets:
- Compile successfully
- Are integrated into the runtime
- Support all FRAME v5 features
- Include complex functionality like offchain workers (DAO pallet)

The only remaining issue is the node startup configuration, which can be resolved with any of the options above.
