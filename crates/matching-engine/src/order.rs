use rust_decimal::Decimal;
use uuid::Uuid;

pub type OrderId = Uuid;
pub type Symbol = String;
pub type Price = Decimal;
pub type Quantity = Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Bid,
    Ask,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderType {
    Limit,
    Market,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub id: OrderId,
    pub symbol: Symbol,
    pub side: Side,
    pub order_type: OrderType,
    /// None for market orders
    pub price: Option<Price>,
    pub quantity: Quantity,
    pub remaining: Quantity,
    /// Monotonically increasing; set by the sequencer before the engine sees it
    pub sequence_id: u64,
    /// Microseconds since Unix epoch
    pub timestamp_us: u64,
}

#[derive(Debug, Clone)]
pub struct Trade {
    pub maker_order_id: OrderId,
    pub taker_order_id: OrderId,
    pub symbol: Symbol,
    pub price: Price,
    pub quantity: Quantity,
    pub sequence_id: u64,
}
