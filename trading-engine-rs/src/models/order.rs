use super::messages::{NewOrderCommand, ModifyOrderCommand, CancelOrderCommand};
use super::types::{OrderId, OrderStatus, Price, Side};

const NULL_ID: u64 = u64::MAX;
const NULL_FLOAT: f64 = f64::MAX;

#[derive(Debug)]
pub enum OrderError {
    OrderMismatch { order_id: OrderId, expected_order_id: OrderId },
    NegativeQuantity,
    QuantityBelowFilled { quantity: f64 },
    UnknownSide,
}

pub type OrderResult<T> = Result<T, OrderError>;

/// An order representation to be stored in the book building towards a full FIX implementation
/// Currently ignoring Strings (and any type that is not Clone)
/// TBC - Add OrdStatus enum related to FIX OrdStatus
///       OrderType too (Limit/Market with optional price (?))
///       Symbol...
pub struct Order {
    pub client_order_id: OrderId,
    pub orig_client_order_id: Option<OrderId>,
    pub order_id: OrderId,
    pub ord_status: OrderStatus,
    pub side: Side,
    pub price: Price,
    pub order_qty: f64,
    pub cum_qty: f64,
    pub leaves_qty: f64
    //pub transact_time: i32 - TBC - should be on command or generated on creation / action?
}

impl Order {
    /// Initialise an empty Order array to be populated on command
    pub fn new() -> Self {
        Self {
            client_order_id: NULL_ID,
            orig_client_order_id: None,
            order_id: NULL_ID,
            ord_status: OrderStatus::Unknown,
            side: Side::Unknown,
            price: NULL_FLOAT.into(),
            order_qty: NULL_FLOAT,
            leaves_qty:NULL_FLOAT,
            cum_qty: NULL_FLOAT
        }
    }

    /// Allocate an order struct from a NewOrderCommand
    pub fn alloc(&mut self, command: NewOrderCommand, order_id: OrderId) -> OrderResult<()>{
        if command.quantity <= 0.0 {
            return Err(OrderError::NegativeQuantity);
        }

        self.client_order_id = command.client_order_id;
        self.order_id = order_id;
        self.ord_status = OrderStatus::New;
        self.side = command.side;
        self.price = command.price;
        self.order_qty = command.quantity;
        self.leaves_qty = command.quantity;
        self.cum_qty = 0.0;

        Ok(())
    }

    /// Reset an order struct to original/null values
    pub fn free(&mut self) {
        self.client_order_id = NULL_ID;
        self.orig_client_order_id = None;
        self.order_id = NULL_ID;
        self.ord_status = OrderStatus::Unknown;
        self.side = Side::Unknown;
        self.price = NULL_FLOAT.into();
        self.order_qty = NULL_FLOAT;
        self.leaves_qty = NULL_FLOAT;
        self.cum_qty =  NULL_FLOAT;
    }

    /// Modify an order according to a command
    pub fn modify(&mut self, modify_order_command: ModifyOrderCommand) -> OrderResult<()> {
        if self.client_order_id != modify_order_command.orig_client_order_id {
            return Err(OrderError::OrderMismatch { order_id: self.client_order_id, expected_order_id: modify_order_command.orig_client_order_id });
        }

        if let Some(new_price) = modify_order_command.price {
            self.price = new_price;
        }

        if let Some(new_quantity) = modify_order_command.quantity {
            if new_quantity <= 0.0 {
                return Err(OrderError::NegativeQuantity);
            }

            if new_quantity < self.cum_qty {
                return Err(OrderError::QuantityBelowFilled { quantity: new_quantity });
            }
            // TBC - check this logic, think it is ok
            self.order_qty = new_quantity;
            self.leaves_qty = new_quantity - self.cum_qty;
        }
        self.client_order_id = modify_order_command.client_order_id;
        self.orig_client_order_id = Some(modify_order_command.orig_client_order_id);

        Ok(())
    } 

    /// Cancel an order
    pub fn cancel(&mut self, cancel_order_command: CancelOrderCommand) -> OrderResult<()> {
        if self.client_order_id != cancel_order_command.orig_client_order_id {
            return Err(OrderError::OrderMismatch { order_id: self.client_order_id, expected_order_id: cancel_order_command.orig_client_order_id });
        }

        self.ord_status = OrderStatus::Cancelled;

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::{NULL_FLOAT, NULL_ID, Order, OrderError, OrderResult};
    use crate::models::messages::{CancelOrderCommand, ModifyOrderCommand, NewOrderCommand};
    use crate::models::types::{OrderStatus, Price, Side};

    #[test]
    fn test_new_order_success() -> OrderResult<()> {
        let command = NewOrderCommand {
            client_order_id: 1234,
            side: Side::Buy,
            price: Price::from_float(130.0),
            quantity: 15.9
        };

        let order_id = 9987;

        let mut order = Order::new();
        order.alloc(command, order_id);

        assert_eq!(order.client_order_id, 1234);
        assert_eq!(order.order_id, 9987);
        assert_eq!(order.side, Side::Buy);
        assert_eq!(order.price, Price::from_float(130.0));
        assert_eq!(order.order_qty, 15.9);
        assert_eq!(order.leaves_qty, 15.9);
        assert_eq!(order.cum_qty, 0.0);

        order.free();
        assert_eq!(order.client_order_id, NULL_ID);
        assert_eq!(order.orig_client_order_id, None);
        assert_eq!(order.order_id, NULL_ID);
        assert!(matches!(order.side, Side::Unknown));
        assert!(matches!(order.ord_status, OrderStatus::Unknown));
        assert_eq!(order.price, NULL_FLOAT.into());
        assert_eq!(order.order_qty, NULL_FLOAT);
        assert_eq!(order.leaves_qty, NULL_FLOAT);
        assert_eq!(order.cum_qty, NULL_FLOAT);

        Ok(())
    }

    #[test]
    fn test_new_order_neg_quantity() {
        let command = NewOrderCommand {
            client_order_id: 1234,
            side: Side::Buy,
            price: Price::from_float(130.0),
            quantity: -15.9
        };
        let order_id = 9987;

        let mut order = Order::new();
        let order_result = order.alloc(command, order_id);
        assert!(order_result.is_err());
        assert!(matches!(order_result.err(), Some(OrderError::NegativeQuantity)));
    }

    #[test]
    fn test_modify_order_price_success() -> OrderResult<()> {
        let mut order = Order {
            client_order_id: 1234,
            orig_client_order_id: None,
            order_id: 9987,
            ord_status: OrderStatus::New,
            side: Side::Buy,
            price: Price::from_float(130.0),
            order_qty: 15.9,
            leaves_qty: 15.9,
            cum_qty: 0.0
        };

        let command = ModifyOrderCommand {
            client_order_id: 2345,
            orig_client_order_id: 1234,
            order_id: 9987,
            price: Some(14.0.into()),
            quantity: None
        };

        order.modify(command)?;

        assert_eq!(order.price, 14.0.into());
        Ok(())
    }

    #[test]
    fn test_modify_order_quantity_success() -> OrderResult<()> {
        let mut order = Order {
            client_order_id: 1234,
            orig_client_order_id: None,
            order_id: 9987,
            ord_status: OrderStatus::New,
            side: Side::Buy,
            price: Price::from_float(130.0),
            order_qty: 15.9,
            leaves_qty: 15.9,
            cum_qty: 0.0
        };

        let command = ModifyOrderCommand {
            client_order_id: 2345,
            orig_client_order_id: 1234,
            order_id: 9987,
            price: None,
            quantity: Some(178.0)
        };

        order.modify(command)?;

        assert_eq!(order.cum_qty, 0.0);
        assert_eq!(order.leaves_qty, 178.0);
        assert_eq!(order.order_qty, 178.0);

        Ok(())
    }

    #[test]
    fn test_modify_order_negative_quantity() {
        let mut order = Order {
            client_order_id: 1234,
            orig_client_order_id: None,
            order_id: 9987,
            ord_status: OrderStatus::New,
            side: Side::Buy,
            price: Price::from_float(130.0),
            order_qty: 15.9,
            leaves_qty: 15.9,
            cum_qty: 0.0
        };

        let command = ModifyOrderCommand {
            client_order_id: 2345,
            orig_client_order_id: 1234,
            order_id: 9987,
            price: None,
            quantity: Some(-178.0)
        };

        assert!(matches!(order.modify(command), Err(OrderError::NegativeQuantity)));
    }

    #[test]
    fn test_modify_order_below_filled() {
        let mut order = Order {
            client_order_id: 1234,
            orig_client_order_id: None,
            order_id: 9987,
            ord_status: OrderStatus::PartiallyFilled,
            side: Side::Buy,
            price: Price::from_float(130.0),
            order_qty: 15.9,
            leaves_qty: 5.9,
            cum_qty: 10.0
        };

        let command = ModifyOrderCommand {
            client_order_id: 2345,
            orig_client_order_id: 1234,
            order_id: 9987,
            price: None,
            quantity: Some(9.0)
        };

        assert!(matches!(order.modify(command), Err(OrderError::QuantityBelowFilled { quantity: 9.0 })));
    }

    #[test]
    fn test_cancel_order_success() {
        let mut order = Order {
            client_order_id: 1234,
            orig_client_order_id: None,
            order_id: 9987,
            ord_status: OrderStatus::New,
            side: Side::Buy,
            price: Price::from_float(130.0),
            order_qty: 15.9,
            leaves_qty: 15.9,
            cum_qty: 0.0
        };

        let cancel_order_command = CancelOrderCommand { 
            client_order_id: 2345, 
            orig_client_order_id: 1234,
            order_id: 9987 
        };

        assert!(order.cancel(cancel_order_command).is_ok());
        assert!(matches!(order.ord_status, OrderStatus::Cancelled));
    }
}