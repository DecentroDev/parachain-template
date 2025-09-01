# Insurances Pallet

## Overview

This pallet provides the means represent insurances. It's mostly used for storage management and does not have any
useful business logic.

Insurances are represented as NFTs, which are minted to the insurance holder on creation. Each insurance has an
associated metadata object, which, among other things, features a link to an external legal contract.

Each insurance can also be optionally associated with a secondary market token.

## How to benchmark the pallet

For proper weight estimation, each extrinsic must be accompanied by a benchmark.
To generate weights for the extrinsics, do the following steps:

1. Build the node with the `runtime-benchmarks` feature

 ```shell
 $ cargo build --release --features=runtime-benchmarks
 ```

2. Run the benchmarks with the `benchmark.sh` script in the pallet's `src` directory.
3. A `weights_new.rs` file will be generated into the folder you've run the benchmark script from.
4. Use the weight information in the generated file to update the `weights.rs` file of the pallet.
