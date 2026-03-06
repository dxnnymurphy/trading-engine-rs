/// First Simple Implementation of OrderBook, for POC
/// This is a straightforward implementation using BTreeMap for price levels and Vec for order queues.
/// It is not optimized for performance or memory usage, but serves as a clear starting point for understanding the basic mechanics of an order book.
use std::collections::{BTreeMap, HashMap};
use std::cmp::Reverse;
use std::rc::Rc;
use slotmap::{SlotMap, new_key_type};
use crate::orderbook::OrderBook;
use crate::orderbook::models::{
    Order,
    OrderError,
    OrderId,
    OrderModify,
    OrderPtr,
    Price,
    Quantity,
    Side,
};

// =================================================================================================================================
// Tutorial - https://www.youtube.com/watch?v=XeLWe0Cx_Lg
// NB - The tutorial followed used cpp, and specifically a linked list with location as a stored iterator, which is not supported in
//      Rust, so the following implementation was used:
//      - A stable-node storage (SlotMap) keyed by NodeId, which allows for efficient insertion and deletion without invalidating references to other nodes.
//      - A HashMap to store OrderId to NodeId mapping for quick access during modifications
//      - BTreeMap for price levels, with each level containing a Doubly Linked List of NodeIds to maintain order of arrival at that price level.
// Operations:
//      - Add Order: Insert into SlotMap, update BTreeMap and HashMap. O(log l) for BTreeMap insertion, O(1) for SlotMap and HashMap.
//      - Modify Order: Retrieve NodeId from HashMap, update order in SlotMap, and if price changes, update BTreeMap accordingly. O(log l) for BTreeMap updates, O(1) for SlotMap and HashMap.
//      - Cancel Order: Retrieve NodeId from HashMap, remove from SlotMap and HashMap. O(1) for SlotMap and HashMap.
new_key_type! { struct NodeId; }

struct OrderNode {
    order: OrderPtr,
    prev: Option<NodeId>,
    next: Option<NodeId>,
}
impl OrderNode {
    pub fn new(order: OrderPtr) -> Self {
        Self { order, prev: None, next: None }
    }
}

#[derive(Debug)]
struct LevelQueue {
    head: Option<NodeId>,
    tail: Option<NodeId>,
}

impl LevelQueue {
    fn new() -> Self {
        Self {
            head: None,
            tail: None,
        }
    }

    fn insert(&mut self, node_id: NodeId, level_nodes: &mut SlotMap<NodeId, OrderNode>) {
        let previous_tail = self.tail;
        if let Some(tail_id) = previous_tail {
            if let Some(tail_node) = level_nodes.get_mut(tail_id) {
                tail_node.next = Some(node_id);
            }
        }
        if let Some(new_node) = level_nodes.get_mut(node_id) {
            new_node.prev = previous_tail;
        }
        if self.head.is_none() {
            self.head = Some(node_id);
        }
        self.tail = Some(node_id);
    }

    fn remove(&mut self, order_id: OrderId, node_id: NodeId, level_nodes: &mut SlotMap<NodeId, OrderNode>) -> Result<bool, OrderError> {
        // Removal logic is handled in the cancel_order method of SimpleOrderBook, which updates the linked list pointers accordingly.
        let (prev, next) = level_nodes
            .get(node_id)
            .map(|node| (node.prev, node.next))
            .ok_or(OrderError::OrderNotFound { order_id })?;

        if let Some(prev_id) = prev {
            if let Some(prev_node) = level_nodes.get_mut(prev_id) {
                prev_node.next = next;
            }
        }
        if let Some(next_id) = next {
            if let Some(next_node) = level_nodes.get_mut(next_id) {
                next_node.prev = prev;
            }
        }

        if self.head == Some(node_id) {
            self.head = next;
        }
        if self.tail == Some(node_id) {
            self.tail = prev;
        }
        Ok(self.head.is_none())
    }
}

struct OrderEntry {
    pub order: OrderPtr,
    pub location: NodeId,
}

// =================================================================================================================================

pub struct SimpleOrderBook {
    bids: BTreeMap<Reverse<Price>, LevelQueue>, // Highest first
    asks: BTreeMap<Price, LevelQueue>,
    level_nodes: SlotMap<NodeId, OrderNode>, // Stable-node storage keyed by NodeId
    orders: HashMap<OrderId, OrderEntry>, // For quick access to orders for modification and cancellation
}

impl SimpleOrderBook {
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            level_nodes: SlotMap::with_key(),
            orders: HashMap::new(),
        }
    }
}   


impl OrderBook for SimpleOrderBook {
    fn add_order(&mut self, order: OrderPtr) -> Result<(), OrderError> {
        let order_ref = order.borrow();
        if self.orders.contains_key(&order_ref.id) {
            return Err(OrderError::OrderExists { order_id: order_ref.id });
        }

        match order_ref.side {
            Side::Buy => {
                let node_id = self.level_nodes.insert(OrderNode::new(Rc::clone(&order)));
                let level = self
                    .bids
                    .entry(Reverse(order_ref.price))
                    .or_insert_with(LevelQueue::new);
                level.insert(node_id, &mut self.level_nodes);
                self.orders.insert(order_ref.id, OrderEntry { order: Rc::clone(&order), location: node_id });
            }
            Side::Sell => {
                let node_id = self.level_nodes.insert(OrderNode::new(Rc::clone(&order)));
                let level = self
                    .asks
                    .entry(order_ref.price)
                    .or_insert_with(LevelQueue::new);
                level.insert(node_id, &mut self.level_nodes);
                self.orders.insert(order_ref.id, OrderEntry { order: Rc::clone(&order), location: node_id });
            }
        }
        Ok(())
    }

    fn modify_order(&mut self, order_modify: OrderModify) -> Result<(), OrderError> {
        let new_order_ptr = {
            let order_entry = self
                .orders
                .get(&order_modify.order_id)
                .ok_or(OrderError::OrderNotFound {
                    order_id: order_modify.order_id,
                })?;

            let existing_order = order_entry.order.borrow();
            order_modify.to_order_pointer(existing_order.order_type, &existing_order)
        };
        self.cancel_order(order_modify.order_id)?;
        self.add_order(new_order_ptr)?;
        Ok(())
    }

    fn cancel_order(&mut self, order_id: OrderId) -> Result<(), OrderError> {
        let entry = self
            .orders
            .remove(&order_id)
            .ok_or(OrderError::OrderNotFound { order_id })?;

        let (side, price) = {
            let order = entry.order.borrow();
            (order.side, order.price)
        };

        let remove_level = match side {
            Side::Buy => {
                if let Some(level) = self.bids.get_mut(&Reverse(price)) {
                    level.remove(order_id, entry.location, &mut self.level_nodes)?
                } else {
                    false
                }
            }
            Side::Sell => {
                if let Some(level) = self.asks.get_mut(&price) {
                    level.remove(order_id, entry.location, &mut self.level_nodes)?
                } else {
                    false
                }
            }
        };

        if remove_level {
            match side {
                Side::Buy => {
                    self.bids.remove(&Reverse(price));
                }
                Side::Sell => {
                    self.asks.remove(&price);
                }
            }
        }

        self.level_nodes.remove(entry.location);

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::orderbook::{self, models::{OrderType, Side}};

    #[test]
    fn test_add_orders() {
        let mut order_book = SimpleOrderBook::new();
        let order1 = Order::to_order_pointer(Order::new(1, Side::Buy, OrderType::Limit, 100, 10));
        let order2 = Order::to_order_pointer(Order::new(2, Side::Sell, OrderType::Limit, 101, 5));

        order_book.add_order(order1).unwrap();
        order_book.add_order(order2).unwrap();

        assert_eq!(order_book.orders.len(), 2);
        assert!(order_book.bids.contains_key(&Reverse(100)));
        assert!(order_book.asks.contains_key(&101));
    }

    #[test]
    fn test_cancel_order() {
        let mut order_book = SimpleOrderBook::new();
        let order = Order::to_order_pointer(Order::new(1, Side::Buy, OrderType::Limit, 100, 10));
        order_book.add_order(order).unwrap();

        assert!(order_book.bids.contains_key(&Reverse(100)));

        order_book.cancel_order(1).unwrap();

        println!("Bids after cancellation: {:?}", order_book.bids);

        assert!(!order_book.bids.contains_key(&Reverse(100)));
    }

    #[test]
    fn test_modify_order_price() {
        let mut order_book = SimpleOrderBook::new();
        let order = Order::to_order_pointer(Order::new(1, Side::Buy, OrderType::Limit, 100, 10));
        order_book.add_order(order).unwrap();

        assert!(order_book.bids.contains_key(&Reverse(100)));

        let modify = OrderModify { order_id: 1, side: Side::Buy, price: Some(99), quantity: None };
        order_book.modify_order(modify).unwrap();

        println!("Bids after modification: {:?}", order_book.bids);

        assert!(!order_book.bids.contains_key(&Reverse(100)));
        assert!(order_book.bids.contains_key(&Reverse(99)));
    }

    #[test]
    fn test_modify_order_quantity() {
        let mut order_book = SimpleOrderBook::new();
        let order = Order::to_order_pointer(Order::new(1, Side::Buy, OrderType::Limit, 100, 10));
        order_book.add_order(order).unwrap();

        assert!(order_book.bids.contains_key(&Reverse(100)));

        let modify = OrderModify { order_id: 1, side: Side::Buy, price: None, quantity: Some(5) };
        order_book.modify_order(modify).unwrap();

        println!("Bids after modification: {:?}", order_book.bids);

        assert!(order_book.bids.contains_key(&Reverse(100)));
        assert!(order_book.orders.get(&1).unwrap().order.borrow().remaining_quantity == 5);
    }

    /// Conceptually valid, but in a real system...
    #[test]
    fn test_modify_order_side() {
        let mut orderbook = SimpleOrderBook::new();
        let order = Order::to_order_pointer(Order::new(1, Side::Buy, OrderType::Market, 100, 10));
        orderbook.add_order(order).unwrap();

        let modify = OrderModify { order_id: 1, side: Side::Sell, price: None, quantity: None };
        orderbook.modify_order(modify).unwrap();
        println!("Bids after modification: {:?}", orderbook.bids);
        println!("Asks after modification: {:?}", orderbook.asks);
        assert!(!orderbook.bids.contains_key(&Reverse(100)));
        assert!(orderbook.asks.contains_key(&100));

    }
}

// Benchmarking Results 
// Add-only throughput
// 1k: ~5.46 Melem/s
// 5k: ~6.36 Melem/s
// 10k: ~6.32 Melem/s
// Cancel-heavy throughput
// 1k: ~7.60 Melem/s
// 5k: ~9.63 Melem/s
// 10k: ~9.83 Melem/s
// Modify-heavy throughput
// 1k: ~5.65 Melem/s
// 5k: ~5.83 Melem/s
// 10k: ~5.43 Melem/s
// Mixed throughput
// 1k: ~6.31 Melem/s
// 5k: ~6.50 Melem/s
// 10k: ~6.33 Melem/s
// Overall solid, useful first benchmark!