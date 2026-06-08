pub mod error;
pub mod order;
pub mod order_book;
#[cfg(test)]
mod tests;

pub use order::{Order, OrderId, Price, Quantity, Side, OrderType, Symbol, Trade};
pub use order_book::OrderBook;
pub use error::EngineError;

/// Returned by every order submission. Contains all trades that were
/// generated and the final state of the submitted order.
#[derive(Debug)]
pub struct MatchResult {
    pub trades: Vec<Trade>,
    /// None if the order was fully consumed; Some if it rests in the book
    /// (limit) or was partially filled against an empty book (market).
    pub remaining: Option<Order>,
}
