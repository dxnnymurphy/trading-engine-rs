use std::collections::{HashMap, BTreeMap};
use std::cmp::Reverse;
use chrono::{Datelike, Utc};
use crate::models::messages::{CancelOrderCommand, ModifyOrderCommand, NewOrderCommand};
use crate::models::types::{OrderId, Price, Side};
use crate::orderbook::types::OrderBookError;
use crate::orderbook::OrderBook;
use super::arena_node::ArenaNode;
use super::price_level::PriceLevel;
use super::types::{ArenaId, LevelId};

#[derive(Debug)]
enum BookInternalError {
    ArenaNodeNotFound { node_id: ArenaId },
    LevelNotFound { level_id: Option<LevelId> },
    UnknownSide,
}

type InternalResult<T> = Result<T, BookInternalError>;

impl From<BookInternalError> for OrderBookError {
    fn from(error: BookInternalError) -> Self {
        match error {
            BookInternalError::ArenaNodeNotFound { node_id: _ } => {
                OrderBookError::InternalState { reason: "arena node missing " }
            }
            BookInternalError::LevelNotFound { level_id: _ } => {
                OrderBookError::InternalState { reason: "price level missing" }
            }
            BookInternalError::UnknownSide => OrderBookError::InternalState { reason: "unknown side" },
        }
    }
}

/// Implementation of OrderBook, an arena based approach containing nodes with preallocated Orders.
/// Using a BTreeMap to store ordered Pricee levels
struct Book {
    /// price levels and arena nodes
    levels: Vec<PriceLevel>,
    arena: Vec<ArenaNode>,
    /// lists of free nodes and price levels to be fetched
    free_arena_ids: Vec<ArenaId>,
    free_level_ids: Vec<LevelId>,
    /// order_id -> arena node id map
    order_index: HashMap<OrderId, ArenaId>,
    /// price -> level map
    /// bids are reversed because highest bid comes first
    asks: BTreeMap<Price, LevelId>,
    bids: BTreeMap<Reverse<Price>, LevelId>,
    /// internal order id to assign to orders as they come in
    current_date_prefix: u64,
    current_daily_seq: u64
}

impl Book {
    /// Used to avoid reallocation and copying
    const LEVELS: usize = 1000;
    const ARENA_SIZE: usize = 10000;
    /// Number of sequence slots per day (`YYYYMMDD * DAILY_SEQ_SCALE + seq`).
    const DAILY_SEQ_SCALE: u64 = 1_000_000;

    pub fn new() -> Self {
        Self {
            levels: Vec::with_capacity(Self::LEVELS),
            arena: Vec::with_capacity(Self::LEVELS),
            free_arena_ids: Vec::with_capacity(Self::ARENA_SIZE),
            free_level_ids: Vec::with_capacity(Self::LEVELS),
            order_index: HashMap::with_capacity(Self::ARENA_SIZE),
            // No preallocation for the BreeMap because is node based, only will be allocated on a new
            // price level so performance implication should be less
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
            current_date_prefix: Self::generate_date_prefix(),
            current_daily_seq: 0
        }
    }

    // Helper methods

    /// Generate a date prefix in the form yyyymmdd
    fn generate_date_prefix() -> u64 {
        let d = Utc::now().date_naive();
        (d.year() as u64) * 10000 + (d.month() as u64) * 100 + d.day() as u64
    }

    /// Generate an order id in the format yyyymmddX where X is the order number of the day, currently configured to take
    /// 1 million per day
    fn generate_order_id(&mut self) -> OrderId {
        let today = Self::generate_date_prefix();
        if self.current_date_prefix != today {
            self.current_date_prefix = today;
            self.current_daily_seq = 0;
        }

        self.current_daily_seq += 1;
        debug_assert!(
            self.current_daily_seq < Self::DAILY_SEQ_SCALE,
            "daily order-id sequence exhausted"
        );

        self.current_date_prefix * Self::DAILY_SEQ_SCALE + self.current_daily_seq
    }

    /// Get an immutable reference to an Arena Node
    fn arena_node(&self, arena_id: ArenaId) -> InternalResult<&ArenaNode> {
        self.arena
            .get(arena_id)
            .ok_or(BookInternalError::ArenaNodeNotFound { node_id: arena_id })
    }

    /// Get a mutable reference to an Arena Node
    fn arena_node_mut(&mut self, arena_id: ArenaId) -> InternalResult<&mut ArenaNode> {
        self.arena
            .get_mut(arena_id)
            .ok_or(BookInternalError::ArenaNodeNotFound { node_id: arena_id })
    }

    /// Get an immutable reference to a Price Level
    fn level(&self, level_id: LevelId) -> InternalResult<&PriceLevel> {
        self.levels
            .get(level_id)
            .ok_or(BookInternalError::LevelNotFound { level_id: Some(level_id) })
    }

    /// Get a mutable reference to a Price LEvel
    fn level_mut(&mut self, level_id: LevelId) -> InternalResult<&mut PriceLevel> {
        self.levels
            .get_mut(level_id)
            .ok_or(BookInternalError::LevelNotFound { level_id: Some(level_id) })
    }
    
    /// Probably a better name, because will only create if arena is full, same with below
    fn create_arena_node(&mut self) -> ArenaId {
        if let Some(id) = self.free_arena_ids.pop() {
            id
        } else {
            self.arena.push(ArenaNode::new());
            self.arena.len() - 1
        }
    }

    /// Utility function to create a new level by popping from the free levels, or appending to the existing list if it is full
    fn create_level_raw (
        levels: &mut Vec<PriceLevel>, 
        free_level_ids: &mut Vec<LevelId>,
        price: Price, 
        side: Side
    ) -> LevelId {
        if let Some(id) = free_level_ids.pop() {
            levels[id] = PriceLevel::new(price, side);
            id
        } else {
            levels.push(PriceLevel::new(price, side));
            levels.len() - 1
        }
    }

    /// Utility function to get the Price level for a given Price, and create it if it does not exist
    fn get_or_create_level(&mut self, side: Side, price: Price) -> InternalResult<LevelId> {
        match side {
            Side::Buy => {
                let key = Reverse(price);
                let (bids, levels, free_level_ids) = (&mut self.bids, &mut self.levels, &mut self.free_level_ids);
                let level_id = *bids.entry(key).or_insert_with(|| Self::create_level_raw(levels, free_level_ids, price, side));
                Ok(level_id)
            },
            Side::Sell => {
                let key = price;
                let (asks, levels, free_level_ids) = (&mut self.asks, &mut self.levels, &mut self.free_level_ids);
                let level_id = *asks.entry(key).or_insert_with(|| Self::create_level_raw(levels, free_level_ids, price, side));
                Ok(level_id)
            },
            Side::Unknown => return Err(BookInternalError::UnknownSide),
        }
    }

    /// Utility function to append a node to a price level
    fn append_to_level(&mut self, arena_id: ArenaId, level_id: LevelId) -> InternalResult<()> {
        let prev_tail_id = {
            let level = self.level_mut(level_id)?;
            if level.is_empty() {
                level.head = Some(arena_id);
                level.tail = Some(arena_id);
                None
            } else {
                let prev_tail_id = level.tail.ok_or(BookInternalError::LevelNotFound { level_id: Some(level_id) })?;
                level.tail = Some(arena_id);
                Some(prev_tail_id)
            }
        };

        if let Some(tail_id) = prev_tail_id {
            self.arena_node_mut(tail_id)?.next = Some(arena_id);
        }

        Ok(())
    }

    /// Utility function to remove a node from a price level by unlinking and removing the level if empty
    fn remove_from_level(&mut self, prev: Option<ArenaId>, next: Option<ArenaId>, level_id: Option<LevelId>) -> InternalResult<()> {
        let level_id = level_id.ok_or(BookInternalError::LevelNotFound { level_id: None })?;
        match (prev, next) {
            (Some(n), Some(p)) => {
                // Middle of the level
                self.arena_node_mut(n)?.prev = Some(p);
                self.arena_node_mut(p)?.next = Some(n);
            },
            (Some(n), None) => {
                // First order in level
                self.arena_node_mut(n)?.prev = None;
                self.level_mut(level_id)?.head = Some(n);
            },
            (None, Some(p)) => {
                // Last order in level
                self.arena_node_mut(p)?.next = None;
                self.level_mut(level_id)?.tail = Some(p);
            },
            (None, None) => {
                // Only order in level
                let (level_price, level_side)  = {
                    let level = self.level_mut(level_id)?;
                    level.head = None;
                    level.tail = None;
                    (level.price, level.side)
                };
                match level_side {
                    Side::Buy => self.bids.remove(&Reverse(level_price)),
                    Side::Sell => self.asks.remove(&level_price),
                    Side::Unknown => return Err(BookInternalError::UnknownSide)
                };
                self.free_level_ids.push(level_id);
            }
        };
        Ok(())
    }
}

impl OrderBook for Book {
    type OrderId = OrderId;
    type Error = OrderBookError;

    fn new_order(&mut self, command: NewOrderCommand) -> Result<Self::OrderId, Self::Error> {
        let order_id = self.generate_order_id();

        let level_id = self.get_or_create_level(command.side, command.price)
            .map_err(OrderBookError::from)?;
        let level_tail = self.level(level_id)
            .map_err(OrderBookError::from)?
            .tail;

        let arena_id = self.create_arena_node();
        self.arena_node_mut(arena_id)
            .map_err(OrderBookError::from)?
            .alloc(order_id, command, level_tail, None, level_id);

        self.order_index.insert(order_id, arena_id);

        self.append_to_level(arena_id, level_id)
            .map_err(OrderBookError::from)?;

        Ok(order_id)
    }

    fn cancel_order(&mut self, command: CancelOrderCommand) -> Result<(), Self::Error>  {
        let arena_id = self.order_index.remove(&command.order_id).ok_or(OrderBookError::OrderNotFound { order_id: command.order_id })?;
        // Is tis scoped operation returning values a correct pattern?
        let (level_id, prev, next) = {
            let node = self.arena_node_mut(arena_id)
                .map_err(OrderBookError::from)?;
            node.free()
        };
        self.remove_from_level(prev, next, level_id)
            .map_err(OrderBookError::from)?;
        self.free_arena_ids.push(arena_id);
        Ok(())
    }

    fn replace_order(&mut self, command : ModifyOrderCommand) -> Result<(), Self::Error> {
        let arena_id = self.order_index.get(&command.order_id).ok_or(OrderBookError::OrderNotFound { order_id: command.order_id })?.clone();
        let command_price = command.price;
        let (order_level, order_side, next, prev) = {
            let arena_node = self.arena_node_mut(arena_id)
                .map_err(OrderBookError::from)?;
            arena_node.order.modify(command)?;
            (arena_node.level, arena_node.order.side, arena_node.next, arena_node.prev)
        };
    
        // Modify Price: remove from original level, append to end of new level
        if let Some(new_price) = command_price {
            let orig_level_id = order_level.ok_or(OrderBookError::InternalState { reason: "missing level id for order" })?;
            self.remove_from_level(prev, next, Some(orig_level_id))
                .map_err(OrderBookError::from)?;

            let new_level_id = self.get_or_create_level(order_side, new_price)
                .map_err(OrderBookError::from)?;
            self.append_to_level(arena_id, new_level_id)
                .map_err(OrderBookError::from)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::models::messages::{CancelOrderCommand, ModifyOrderCommand, NewOrderCommand};
    use crate::models::types::Side;
    use crate::orderbook::OrderBook;
    use crate::orderbook::types::OrderBookError;
    use super::Book;
    use std::cmp::Reverse;

    #[test]
    fn test_new_order() {
        let mut orderbook = Book::new();

        let command = NewOrderCommand {
            client_order_id: 123,
            side: Side::Buy,
            price: 69.0.into(),
            quantity: 150.0
        };

        let order_id = orderbook.new_order(command);
        assert!(order_id.is_ok());
        assert_eq!(order_id.unwrap(), 1);
        assert!(orderbook.asks.is_empty());
        assert!(orderbook.bids.len() == 1);
        assert!(orderbook.bids.contains_key(&Reverse(69.0.into())));
        assert!(orderbook.order_index.contains_key(&1));
    }

    #[test]
    fn test_cancel_order() {
        let mut orderbook = Book::new();

        let new_order_command = NewOrderCommand {
            client_order_id: 1,
            side: Side::Sell,
            price: 67.0.into(),
            quantity: 1.0
        };

        let order_id = orderbook.new_order(new_order_command);
        assert!(order_id.is_ok());

        let cancel_order_command = CancelOrderCommand {
            client_order_id: 2,
            orig_client_order_id: 1,
            order_id: order_id.unwrap()
        };

        let cancel_result = orderbook.cancel_order(cancel_order_command);
        assert!(cancel_result.is_ok());
        assert!(orderbook.asks.is_empty());
        assert!(orderbook.bids.is_empty());
        assert!(orderbook.order_index.is_empty());
    }


    #[test]
    fn test_cancel_order_not_found() {
        let mut orderbook = Book::new();

        let cancel_order_command = CancelOrderCommand {
            client_order_id: 1,
            orig_client_order_id: 0,
            order_id: 202609260000001
        };

        let cancel_result = orderbook.cancel_order(cancel_order_command);
        assert!(cancel_result.is_err());
        assert!(matches!(cancel_result.err().unwrap(), OrderBookError::OrderNotFound { order_id: 202609260000001 }));
    }

    #[test]
    fn test_replace_order_price() {
        let mut orderbook = Book::new();

        let new_order_command = NewOrderCommand {
            client_order_id: 1,
            side: Side::Sell,
            price: 67.0.into(),
            quantity: 1.0
        };

        let order_id = orderbook.new_order(new_order_command).expect("Order not entered successfully!");

        let new_order_command_2 = NewOrderCommand {
            client_order_id: 2,
            side: Side::Sell,
            price: 123.0.into(),
            quantity: 100.0
        };

        let order_id_2 = orderbook.new_order(new_order_command_2).expect("Order not entered successfully!");

        let modify_order_command = ModifyOrderCommand {
            client_order_id: 3,
            orig_client_order_id: 1,
            order_id: order_id,
            price: Some(68.0.into()),
            quantity: None
        };

        let replace_result = orderbook.replace_order(modify_order_command);
        assert!(replace_result.is_ok());
        assert!(!orderbook.asks.is_empty());
        assert!(orderbook.bids.is_empty());
        assert_eq!(orderbook.asks.len(), 2);

        let ask_prices: Vec<_> = orderbook.asks.keys().copied().collect();
        assert_eq!(ask_prices, vec![68.0.into(), 123.0.into()]);
        assert!(!orderbook.asks.contains_key(&67.0.into()));
        assert!(orderbook.asks.contains_key(&68.0.into()));
        assert!(orderbook.asks.contains_key(&123.0.into()));
    }

    #[test]
    fn test_replace_order_quantity() {
        let mut orderbook = Book::new();

        let new_order_command = NewOrderCommand {
            client_order_id: 1,
            side: Side::Sell,
            price: 67.0.into(),
            quantity: 1.0
        };

        let order_id = orderbook.new_order(new_order_command).expect("Order not entered successfully!");

        let new_order_command_2 = NewOrderCommand {
            client_order_id: 2,
            side: Side::Sell,
            price: 67.0.into(),
            quantity: 100.0
        };

        let order_id_2 = orderbook.new_order(new_order_command_2).expect("Order not entered successfully!");

        let modify_order_command = ModifyOrderCommand {
            client_order_id: 3,
            orig_client_order_id: 1,
            order_id: order_id,
            price: None,
            quantity: Some(45.0)
        };

        let replace_result = orderbook.replace_order(modify_order_command);
        assert!(replace_result.is_ok());
        assert!(!orderbook.asks.is_empty());
        assert!(orderbook.bids.is_empty());
        assert_eq!(orderbook.asks.len(), 1);

        let ask_prices: Vec<_> = orderbook.asks.keys().copied().collect();
        assert_eq!(ask_prices, vec![67.0.into()]);
        assert!(orderbook.asks.contains_key(&67.0.into()));

        let level_id = *orderbook.asks.get(&67.0.into()).expect("Level not found!");
        let level = orderbook.level(level_id).expect("Level not found!");
        assert!(!level.is_empty());

        // A) Quantity was updated on the replaced order
        let replaced_arena_id = *orderbook
            .order_index
            .get(&order_id)
            .expect("Replaced order missing from index");
        let replaced_node = orderbook
            .arena_node(replaced_arena_id)
            .expect("Replaced arena node missing");
        assert_eq!(replaced_node.order.order_qty, 45.0);
        assert_eq!(replaced_node.order.leaves_qty, 45.0);
        assert_eq!(replaced_node.order.cum_qty, 0.0);

        // B) Replaced order is appended to the end of the same level (FIFO tail)
        let head_id = level.head.expect("Expected level head");
        let tail_id = level.tail.expect("Expected level tail");

        let head_node = orderbook.arena_node(head_id).expect("Head node missing");
        let tail_node = orderbook.arena_node(tail_id).expect("Tail node missing");

        assert_eq!(head_node.order.order_id, order_id_2);
        assert_eq!(tail_node.order.order_id, order_id);
        assert_eq!(head_node.prev, None);
        assert_eq!(head_node.next, Some(tail_id));
        assert_eq!(tail_node.prev, Some(head_id));
        assert_eq!(tail_node.next, None);
        

    }
}