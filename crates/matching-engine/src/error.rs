use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("order {0} not found")]
    OrderNotFound(crate::order::OrderId),

    #[error("symbol mismatch: book is {book}, order is {order}")]
    SymbolMismatch { book: String, order: String },

    #[error("limit order must have a price")]
    MissingLimitPrice,
}
