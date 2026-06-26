/// These are not inhrerently linked to the order book, but are used by the order book
/// will likely be moved in the future as the project grows

use thiserror::Error;

#[derive(Error, Debug)]
pub enum OrderError {
    #[error("Order {order_id} already exists")]
    OrderExists { order_id: OrderId },

    #[error("Order {order_id} not found")]
    OrderNotFound { order_id: OrderId },

    #[error("Cannot fill more than the remaining quantity of the order")]
    Overfill,

    #[error("Order pool exhausted")]
    PoolExhausted,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum OrderType {
    Limit, 
    Market
}
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Side {
    Buy,
    Sell
}

pub type Price = i32;
pub type Quantity = i32;
pub type OrderId = i64;

pub struct Order {
    pub id: OrderId,
    pub orig_order_id: Option<OrderId>,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Price,
    pub initial_quantity: Quantity,
    pub remaining_quantity: Quantity,
}

impl Order {
    pub fn new(id: OrderId, side: Side, order_type: OrderType, price: Price, quantity: Quantity) -> Self {
        Self { id, orig_order_id: None, side, order_type, price, initial_quantity: quantity, remaining_quantity: quantity }
    }

    pub fn get_filled_quantity(&self) -> Quantity {
        self.initial_quantity - self.remaining_quantity
    }

    pub fn fill(&mut self, quantity: Quantity) -> Result<(), OrderError> {
        if quantity > self.remaining_quantity {
            return Err(OrderError::Overfill);
        }
        self.remaining_quantity -= quantity;
        Ok(())
    }

    pub fn is_filled(&self) -> bool {
        self.remaining_quantity == 0
    }

    pub fn to_order_pointer(order: Order) -> OrderPtr {
        Rc::new(RefCell::new(order))
    }
}

pub struct OrderModify {
    pub order_id: OrderId,
    pub side: Side,
    pub price: Option<Price>,
    pub quantity: Option<Quantity>,
}

impl OrderModify {
    pub fn new(order_id: OrderId, side: Side, price: Option<Price>, quantity: Option<Quantity>) -> Self {
        Self {
            order_id,
            side,
            price,
            quantity,
        }
    }

    pub fn to_order_pointer(&self, order_type: OrderType, existing_order: &Order) -> OrderPtr {
        Rc::new(RefCell::new(Order::new(
            self.order_id,
            self.side,
            order_type,
            self.price.unwrap_or(existing_order.price),
            self.quantity.unwrap_or(existing_order.remaining_quantity),
        )))
    }
}   

use std::rc::Rc;
use std::cell::RefCell;
pub type OrderPtr = Rc<RefCell<Order>>;

pub struct TradeInfo {
    pub order_id: OrderId,
    pub price: Price,
    pub quantity: Quantity,
}

pub struct Trade {
    pub bid_trade: TradeInfo,
    pub ask_trade: TradeInfo,
}
impl Trade {
    pub fn new(bid_trade: TradeInfo, ask_trade: TradeInfo) -> Self {
        Self { bid_trade: bid_trade, ask_trade: ask_trade }
    }
}

pub type Trades = Vec<Trade>; // TBC...
