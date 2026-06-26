pub type OrderBookResult<T> = Result<T, OrderBookError>;

use crate::models::order::OrderError;
use crate::models::types::{OrderId, Price};
use std::io;

#[derive(Debug)]
pub enum OrderBookError {
    Io(io::Error),
    OrderNotFound { order_id: OrderId },
    PriceLevelNotFound { price: Price },
    OrderError { error: OrderError },
    InternalState { reason: &'static str },
}

impl From<io::Error> for OrderBookError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<OrderError> for OrderBookError {
    fn from(error: OrderError) -> Self {
        Self::OrderError { error }
    }
}