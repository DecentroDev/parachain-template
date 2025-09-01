#!/bin/sh

WORKDIR="$(pwd)"
cd "$(cargo metadata --format-version 1 | sed 's/.*"workspace_root":"\([^"]*\)".*/\1/')" || exit

cargo run --release --features runtime-benchmarks -- benchmark pallet --chain dev --execution wasm \
    --wasm-execution compiled \
    --pallet pallet_payout_processor \
    --extrinsic '*' \
    --steps 25 \
    --repeat 100 \
    --json-file="$WORKDIR/pallets/payout_processor/src/benchmark_raw.json" \
    --output "$WORKDIR/pallets/payout_processor/src/weights_new.rs"
