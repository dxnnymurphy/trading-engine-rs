/// These messages are commands, meant to represent FIX commands.
/// TODO -> Migrate these to actually use FIX messages once FIX engine is implemented.

use super::types::{OrderId, Price, Side};

/// New Order Command - to create a new order (equivalent to New Order Single)
pub struct NewOrderCommand {
    pub client_order_id: OrderId,
    pub side: Side,
    pub price: Price,
    pub quantity: f64
}

/// Modify Order Command - to modify an existing order (equivalent to Order Cancel/Replace Request)
pub struct ModifyOrderCommand {
    pub client_order_id: OrderId,
    pub orig_client_order_id: OrderId,
    pub order_id: OrderId,
    pub price: Option<Price>,
    pub quantity: Option<f64>
}

/// Cancel Order Command - to cancel an existing order (equivalent to Order Cancel Request)
pub struct CancelOrderCommand {
    pub client_order_id: OrderId,
    pub orig_client_order_id: OrderId,
    pub order_id: OrderId // this would be great, need to check with FIX if actually in 35=F
}