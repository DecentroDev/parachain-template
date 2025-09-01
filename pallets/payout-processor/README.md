# Payout Processor Pallet

## Overview

This pallet implements insurance payouts.

There are 2 cases in which insurance needs to be paid out:

- peril event occurrence
- insurance expiration

If the peril event occurs, all the known active insurances, that match this event, will be paid out. The payout
unlocks the underwrite amount and premium amount from DAO account and transfers it to the insured user.

The liquidity provider will be paid out only if the insurance expires. If no one has bought out the liquidity for
this insurance, then DAO is the liquidity provider and will have their funds unlocked, and premium amount transferred to
their balance. If the liquidity for the insurance was bought out, then the secondary market token holders are allowed to
redeem their tokens.

## Interface

### Dispatchable functions

- `feed_event` - notifies payout processor about an event that can trigger some insurances. This can be only invoked by
  a privileged operator.
- `claim_premium_amount` - burns the secondary market tokens and transfers the appropriate amount of currency to the
  token holder.

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
