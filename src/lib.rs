//! Uniform-price clearing for a single frequent batch auction.
//!
//! The implementation follows the market-clearing rule described by Budish,
//! Cramton, and Shim: choose the maximum executable quantity and use the
//! midpoint of the market-clearing price interval as the uniform price.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub type Price = u128;
pub type Quantity = u128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Order {
    pub price: Price,
    pub quantity: Quantity,
}

impl Order {
    pub const fn new(price: Price, quantity: Quantity) -> Self {
        Self { price, quantity }
    }
}

/// The full set of prices that clear the maximum executable quantity.
///
/// The auction's uniform price is the midpoint of this inclusive interval.
/// Prices use integer ticks, so `midpoint()` reports whether the exact midpoint
/// falls halfway between two ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClearingPrice {
    pub lower: Price,
    pub upper: Price,
}

impl ClearingPrice {
    pub fn midpoint(self) -> Midpoint {
        let distance = self.upper - self.lower;
        Midpoint {
            whole_ticks: self.lower + distance / 2,
            half_tick: distance % 2 == 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Midpoint {
    pub whole_ticks: Price,
    pub half_tick: bool,
}

impl fmt::Display for Midpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.half_tick {
            write!(formatter, "{}.5", self.whole_ticks)
        } else {
            write!(formatter, "{}", self.whole_ticks)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClearingOutcome {
    pub price: ClearingPrice,
    pub executed_quantity: Quantity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuctionError {
    QuantityOverflow,
}

impl fmt::Display for AuctionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QuantityOverflow => write!(formatter, "aggregate order quantity exceeds u128"),
        }
    }
}

impl Error for AuctionError {}

/// Clears one batch of limit orders in `O(n log n)` time.
///
/// Zero-quantity orders are ignored. Returns `Ok(None)` when the two sides do
/// not cross, so the batch should carry forward without a trade.
pub fn clear_batch(
    bids: &[Order],
    asks: &[Order],
) -> Result<Option<ClearingOutcome>, AuctionError> {
    let bids_by_price = aggregate(bids)?;
    let asks_by_price = aggregate(asks)?;

    if bids_by_price.is_empty() || asks_by_price.is_empty() {
        return Ok(None);
    }

    let mut prices = BTreeSet::new();
    prices.extend(bids_by_price.keys().copied());
    prices.extend(asks_by_price.keys().copied());

    let mut demand = checked_sum(bids_by_price.values().copied())?;
    let mut supply = 0_u128;
    let mut maximum_quantity = 0_u128;
    let mut clearing_price_lower = 0_u128;
    let mut clearing_price_upper = 0_u128;

    // Before evaluating p, demand contains bids >= p. After adding asks at p,
    // supply contains asks <= p.
    for price in prices {
        if let Some(ask_quantity) = asks_by_price.get(&price) {
            supply = supply
                .checked_add(*ask_quantity)
                .ok_or(AuctionError::QuantityOverflow)?;
        }

        let executable_quantity = demand.min(supply);
        if executable_quantity > maximum_quantity {
            maximum_quantity = executable_quantity;
            clearing_price_lower = price;
            clearing_price_upper = price;
        } else if executable_quantity == maximum_quantity && maximum_quantity > 0 {
            clearing_price_upper = price;
        }

        if let Some(bid_quantity) = bids_by_price.get(&price) {
            demand -= bid_quantity;
        }
    }

    if maximum_quantity == 0 {
        return Ok(None);
    }

    Ok(Some(ClearingOutcome {
        price: ClearingPrice {
            lower: clearing_price_lower,
            upper: clearing_price_upper,
        },
        executed_quantity: maximum_quantity,
    }))
}

fn aggregate(orders: &[Order]) -> Result<BTreeMap<Price, Quantity>, AuctionError> {
    let mut levels = BTreeMap::<Price, Quantity>::new();

    for order in orders.iter().filter(|order| order.quantity > 0) {
        let quantity = levels.entry(order.price).or_default();
        *quantity = quantity
            .checked_add(order.quantity)
            .ok_or(AuctionError::QuantityOverflow)?;
    }

    Ok(levels)
}

fn checked_sum(values: impl IntoIterator<Item = Quantity>) -> Result<Quantity, AuctionError> {
    values.into_iter().try_fold(0_u128, |total, value| {
        total
            .checked_add(value)
            .ok_or(AuctionError::QuantityOverflow)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(price: Price, quantity: Quantity) -> Order {
        Order::new(price, quantity)
    }

    #[test]
    fn returns_no_trade_when_book_does_not_cross() {
        let bids = [order(99, 10)];
        let asks = [order(100, 10)];

        assert_eq!(clear_batch(&bids, &asks), Ok(None));
    }

    #[test]
    fn clears_at_a_unique_price_and_rations_the_long_side() {
        let bids = [order(100, 70), order(99, 30)];
        let asks = [order(100, 80), order(102, 45)];

        assert_eq!(
            clear_batch(&bids, &asks),
            Ok(Some(ClearingOutcome {
                price: ClearingPrice {
                    lower: 100,
                    upper: 100,
                },
                executed_quantity: 70,
            }))
        );
    }

    #[test]
    fn uses_the_midpoint_of_a_vertical_clearing_interval() {
        let bids = [order(110, 65)];
        let asks = [order(105, 40), order(108, 20), order(109, 30)];

        let outcome = clear_batch(&bids, &asks).unwrap().unwrap();

        assert_eq!(
            outcome,
            ClearingOutcome {
                price: ClearingPrice {
                    lower: 109,
                    upper: 110,
                },
                executed_quantity: 65,
            }
        );
        assert_eq!(outcome.price.midpoint().to_string(), "109.5");
    }

    #[test]
    fn finds_the_entire_clearing_interval() {
        let bids = [order(105, 20), order(98, 55), order(96, 20)];
        let asks = [order(97, 20), order(102, 45), order(105, 15)];

        let outcome = clear_batch(&bids, &asks).unwrap().unwrap();

        assert_eq!(outcome.price.lower, 97);
        assert_eq!(outcome.price.upper, 105);
        assert_eq!(outcome.price.midpoint().to_string(), "101");
        assert_eq!(outcome.executed_quantity, 20);
    }

    #[test]
    fn aggregates_duplicate_price_levels_and_ignores_zero_quantity() {
        let bids = [order(100, 50), order(100, 30), order(100, 0)];
        let asks = [order(100, 80)];

        let outcome = clear_batch(&bids, &asks).unwrap().unwrap();

        assert_eq!(outcome.executed_quantity, 80);
        assert_eq!(outcome.price.lower, 100);
        assert_eq!(outcome.price.upper, 100);
    }

    #[test]
    fn returns_no_trade_for_an_empty_side() {
        assert_eq!(clear_batch(&[], &[order(100, 10)]), Ok(None));
        assert_eq!(clear_batch(&[order(100, 10)], &[]), Ok(None));
    }

    #[test]
    fn detects_quantity_overflow() {
        let bids = [order(100, u128::MAX), order(100, 1)];

        assert_eq!(
            clear_batch(&bids, &[order(100, 1)]),
            Err(AuctionError::QuantityOverflow)
        );
    }
}
