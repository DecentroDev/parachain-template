#!/bin/sh

WORKDIR="$(pwd)"
cd "$(cargo metadata --format-version 1 | sed 's/.*"workspace_root":"\([^"]*\)".*/\1/')" || exit

cargo run --release --features runtime-benchmarks -- benchmark pallet --chain dev --execution wasm \
    --wasm-execution compiled \
    --pallet pallet_dao \
    --extrinsic '*' \
    --steps 25 \
    --repeat 100 \
    --json-file="$WORKDIR/pallets/dao/src/benchmark_raw.json" \
    --output "$WORKDIR/pallets/dao/src/weights_new.rs"
