use crate::models::messages::{NewOrderCommand, ModifyOrderCommand, CancelOrderCommand};

/// Trait defining generic behaviour of a L2 OrderBook, can be implemented in different ways 
/// and benchmarked for performance.
pub trait OrderBook {
    type OrderId;
    type Error;


    fn new_order(&mut self, command: NewOrderCommand) -> Result<Self::OrderId, Self::Error>;
    fn replace_order(&mut self, command: ModifyOrderCommand) -> Result<(), Self::Error>;
    fn cancel_order(&mut self, command: CancelOrderCommand) -> Result<(), Self::Error>;
}

pub mod types;

/// Implementation Modules
pub mod v1;
