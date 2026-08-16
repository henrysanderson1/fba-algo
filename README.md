# Frequent Batch Auction Clearing

A compact Rust implementation of the uniform-price clearing rule for a single
frequent batch auction (FBA).

The design is based on Eric Budish, Peter Cramton, and John Shim's paper,
[“The High-Frequency Trading Arms Race: Frequent Batch Auctions as a Market
Design Response”](https://doi.org/10.1093/qje/qjv027), *The Quarterly Journal of
Economics* 130(4), 2015.

## What the algorithm does

Given limit bids and asks collected during one batch, the algorithm:

1. aggregates orders at identical price levels;
2. sorts the distinct prices;
3. sweeps upward through price, maintaining demand at or above the price and
   supply at or below it;
4. computes executable quantity as `min(demand, supply)` at each price;
5. selects the full interval of prices that maximizes executable quantity; and
6. uses the interval midpoint as the auction's uniform price.

If the book does not cross, it returns no trade. If the midpoint falls between
integer price ticks, the result preserves the exact half-tick rather than
silently rounding it.

Sorting dominates the runtime, so clearing takes `O(n log n)` time and `O(n)`
space for `n` submitted orders.

## Run it

```bash
cargo run
```

Example output:

```text
uniform price: 109.5
executed quantity: 65
clearing interval: [109, 110]
```

Run the test suite and lints with:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Scope

This repository implements price and quantity discovery for one auction batch.
It does not implement a complete exchange: batch scheduling, order persistence,
cancellation, price-time priority, and allocation of partial fills are outside
its current scope.
