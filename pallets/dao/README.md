# DAO Pallet

## Overview

This pallet implements various DAO related functionality, such as:
- requesting insurance
- (dis-)approving insurance requests
- insurance liquidity allocation

### Insurance Request Flow

Voting is executed via Substrate's pallet Collective.

1. User submits an insurance request for DAO using the `request_insurance` extrinsic. The approval callback must be set
   to `allocate_liquidity` extrinsic.
2. DAO members vote for the proposal.
3. After the voting period ended, the voting gets finalized and, if the proposal was approved, the insurance is created.

## Interface

### Dispatchable functions

- `request_insurance` - submits an insurance request to DAO.
- `allocate_liquidity` - used as a callback in the insurance request. Creates insurance for the given user. Only
  callable by the collective DAO decision.
- `vote` - submits a vote for the given insurance request. Only callable by DAO members.

## How to benchmark the pallet

For proper weight estimation, each extrinsic must be accompanied by a benchmark.
To generate weights for the extrinsics, do the following steps:

1. Build the node with the `runtime-benchmarks` feature

 ```bash
 $ cargo build --release --features=runtime-benchmarks
 ```

2. Run the benchmarks with the `benchmark.sh` script in the pallet's `src` directory.
3. A `weights_new.rs` file will be generated into the folder you've run the benchmark script from.
4. Use the weight information in the generated file to update the `weights.rs` file of the pallet.
