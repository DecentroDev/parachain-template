# PRMX Parachain with Omni Node

This document describes how to run the PRMX parachain using the new Polkadot Omni Node approach, which provides a more modern and simplified way to run parachains.

## Overview

The Omni Node approach eliminates the need for custom node binaries and uses a pre-built `polkadot-omni-node` binary that can run any parachain using an external chain specification file. This makes the setup more maintainable and easier to deploy.

## Prerequisites

1. **Rust and Cargo**: Latest stable version
2. **Polkadot Omni Node**: Version 0.7.0 or later
3. **Chain Spec Builder**: For generating chain specifications
4. **Docker and Docker Compose**: For containerized deployment (optional)

## Installation

### 1. Install Omni Node

```bash
cargo install --locked polkadot-omni-node@0.7.0
```

### 2. Install Chain Spec Builder

```bash
cargo install --locked staging-chain-spec-builder
```

### 3. Build the Runtime

```bash
# Build the runtime with production profile
cargo build --profile production
```

### 4. Generate Chain Spec

```bash
# Generate the chain spec for Omni Node
chain-spec-builder create \
    --relay-chain "rococo-local" \
    --para-id 1000 \
    --runtime target/production/wbuild/parachain-template-runtime/parachain_template_runtime.wasm \
    named-preset development
```

This will create a `chain_spec.json` file that the Omni Node will use.

## Running the Omni Node

### Option 1: Direct Execution

Use the provided startup script:

```bash
./start-omni-node.sh
```

Or run manually:

```bash
polkadot-omni-node \
    --chain ./chain_spec.json \
    --collator \
    --name "prmx-omni-collator" \
    --ws-port 9944 \
    --rpc-port 9933 \
    --port 30333 \
    --rpc-cors all \
    --ws-external \
    --rpc-external \
    --unsafe-rpc-external \
    --unsafe-ws-external \
    --base-path ./data \
    --log info
```

### Option 2: Docker Deployment

Build and run using Docker:

```bash
# Build the Docker image
docker build -f Dockerfile.omni -t prmx-omni-node .

# Run with Docker Compose
docker-compose -f docker-compose.omni.yml up -d
```

## Port Configuration

The Omni Node exposes the following ports:

- **9944**: WebSocket RPC (for applications)
- **9933**: HTTP RPC (for debugging)
- **30333**: P2P networking

## Integration with Docker Services

To integrate with your existing Docker services, update the `POLKADOT_PROVIDER` environment variable in your Docker Compose files:

```yaml
environment:
  - POLKADOT_PROVIDER=ws://prmx-omni-node:9944  # When using Docker
  # or
  - POLKADOT_PROVIDER=ws://host.docker.internal:9944  # When running locally
```

## Custom Pallets Migration Status

The custom pallets from the original PRMX blockchain have been temporarily disabled during the migration to the Omni Node approach. This includes:

- `pallet-dao`
- `pallet-insurances` 
- `pallet-marketplace`
- `pallet-payout-processor`
- `offchain-utils`

### Re-enabling Custom Pallets

To re-enable the custom pallets, you'll need to:

1. Update the pallet code to use the new Polkadot SDK syntax
2. Fix the FRAME v4 to v5 migration issues
3. Update the runtime configuration
4. Rebuild the runtime and regenerate the chain spec

This is a significant migration that requires updating the pallet code itself, not just the dependencies.

## Troubleshooting

### Common Issues

1. **Omni Node not found**: Ensure you've installed it with the correct version
2. **Chain spec not found**: Make sure you've built the runtime and generated the chain spec
3. **Port conflicts**: Check that ports 9944, 9933, and 30333 are available
4. **Permission issues**: Ensure the startup script is executable

### Logs

The Omni Node will output logs to the console. Look for:
- Successful connection to relay chain
- Block production messages
- Any error messages

## Next Steps

1. **Test the basic Omni Node setup** with the template pallet
2. **Migrate custom pallets** to the new Polkadot SDK syntax
3. **Update Docker services** to connect to the Omni Node
4. **Deploy to production** with proper security configurations

## Benefits of Omni Node

- **Simplified deployment**: No need for custom node binaries
- **Better maintainability**: Uses standard Polkadot tooling
- **Easier updates**: Chain spec updates don't require rebuilding the node
- **Better compatibility**: Works with the latest Polkadot ecosystem tools







