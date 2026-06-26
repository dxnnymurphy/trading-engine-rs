use crate::models::types::{Price, Side};
use super::types::ArenaId;

/// A single price level in the order book. 
/// Stores FIFO-linked orders at one price
pub(super) struct PriceLevel {
    /// Price associated with this level.
    pub(super) price: Price,
    /// Side associated with this level (`Buy` or `Sell`).
    pub(super) side: Side,
    /// Arena ID of the first order in FIFO sequence.
    pub(super) head: Option<ArenaId>,
    /// Arena ID of the last order in FIFO sequence.
    pub(super) tail: Option<ArenaId>,
}

impl PriceLevel {
    /// Creates an empty price level for a given `price` and `side`.
    ///
    /// The level starts without any linked orders (`head`/`tail` are `None`).
    pub(super) fn new(price: Price, side: Side) -> Self {
        Self {
            price,
            side,
            head: None,
            tail: None
        }
    }

    /// Returns `true` when the level has no orders.
    ///
    /// A level is considered empty when both `head` and `tail` are `None`.
    pub(super) fn is_empty(&self) -> bool {
        self.head.is_none() && self.tail.is_none()
    }
}