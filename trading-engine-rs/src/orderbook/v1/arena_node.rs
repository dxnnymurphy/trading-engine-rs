use crate::models::order::Order;
use crate::models::messages::NewOrderCommand;
use crate::models::types::OrderId;
use super::types::{ArenaId, LevelId};

/// A single node holding one order in the preallocated arena
pub(super) struct ArenaNode {
    /// The preallocated order stored in the node.
    pub(super) order: Order,
    /// The price level the node is stored in (if stored).
    pub(super) level: Option<LevelId>,
    /// The ArenaId of the next node within the price level (if there is one).
    pub(super) next: Option<ArenaId>,
    /// The ArenaId of the previous node within the price level (if there is one).
    pub(super) prev: Option<ArenaId>,
    /// Boolean whether the node is active or not
    pub(super) active: bool
}

impl ArenaNode {
    /// Creates an empty node
    pub(super) fn new() -> Self {
        Self {
            order: Order::new(),
            level: None,
            next: None,
            prev: None,
            active: false
        }
    }

    /// Allocate the node by passing a new order command, and place within a price level.
    pub(super) fn alloc(&mut self, order_id: OrderId, new_order_command: NewOrderCommand, prev: Option<ArenaId>, next: Option<ArenaId>, level_id: LevelId) {
        debug_assert!(!self.active, "alloc called on active ArenaNode");
        self.order.alloc(new_order_command, order_id);
        self.prev = prev;
        self.next = next;
        self.level = Some(level_id);
        self.active = true;
    }

    /// Free the node for a no longer used order.
    pub(super) fn free(&mut self) -> (Option<LevelId>, Option<ArenaId>, Option<ArenaId>) {
        debug_assert!(self.active, "free called on inactive ArenaNode");
        self.order.free();
        let old_level = self.level.take();
        let old_prev = self.prev.take();
        let old_next = self.next.take();
        self.active = false;
        (old_level, old_prev, old_next)
    }
}