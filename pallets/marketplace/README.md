# Marketplace Pallet

## Overview

This pallet provides a way to buy out the insurance liquidity from DAO and implements an order book to trade secondary market tokens.

When buying out the insurance liquidity, a secondary market token will be associated with the said insurance and minted
into the buyers account.

When creating orders, the specified amounts of tokens/currency will be locked in pallet specific storage and unlocked on
order fulfillment/cancelation.

## Interface

### Dispatchable functions

- `provide_liquidity` - buys out the specified insurance from DAO and creates a secondary market token.
- `create_order` - adds a buy/sell order to the order book, locks the specified tokens/currency.
- `fulfill_order` - unlocks the tokens/currency, transfers respective amounts to the order creator and fulfiller.
- `cancel_order` - removes order from the order book, unlocks tokens/currency and returns them to the order creator.

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
