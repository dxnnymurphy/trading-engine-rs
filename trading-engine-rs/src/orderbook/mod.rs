pub mod models;
use models::{OrderError, OrderId, OrderModify, OrderPtr};

// Order Modify TBC...

pub trait OrderBook {
    fn add_order(&mut self, order: OrderPtr) -> Result<(), OrderError>;
    fn modify_order(&mut self, order_modify: OrderModify) -> Result<(), OrderError>;
    fn cancel_order(&mut self, order_id: OrderId) -> Result<(), OrderError>;
}

pub mod simple_order_book;
