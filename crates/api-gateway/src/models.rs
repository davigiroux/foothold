use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request body for POST /orders.
/// `price` is optional — absent for market orders.
#[derive(Debug, Deserialize)]
pub struct PlaceOrderRequest {
    pub user_id: Uuid,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub price: Option<Decimal>,
    pub quantity: Decimal,
}

/// Response body for POST /orders — the client tracks fills via this id.
#[derive(Debug, Serialize)]
pub struct PlaceOrderResponse {
    pub order_id: Uuid,
}
