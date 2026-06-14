use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use matching_engine::{Order, OrderBook};
use risk_engine::RiskError;
use uuid::Uuid;

use crate::models::{PlaceOrderRequest, PlaceOrderResponse};
use crate::state::AppState;

pub enum AppError {
    BadRequest(String),
    Risk(RiskError),
    NotFound,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::BadRequest(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
            AppError::Risk(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            AppError::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
        };
        (status, message).into_response()
    }
}

impl From<RiskError> for AppError {
    fn from(value: RiskError) -> Self {
        AppError::Risk(value)
    }
}

/// POST /orders — validate funds, sequence the order, return 202 + order_id.
///
/// Extractors (the function arguments) are how axum hands you request data:
///   - `State(state)` pulls the shared AppState
///   - `Json(req)`    deserializes the request body into PlaceOrderRequest
///
/// Return type `(StatusCode, Json<...>)` becomes the HTTP response.
pub async fn place_order(
    State(state): State<AppState>,
    Json(req): Json<PlaceOrderRequest>,
) -> Result<(StatusCode, Json<PlaceOrderResponse>), AppError> {
    let (book_side, side) = match req.side.as_str() {
        "buy" => (matching_engine::Side::Bid, risk_engine::Side::Buy),
        "sell" => (matching_engine::Side::Ask, risk_engine::Side::Sell),
        _ => return Err(AppError::BadRequest(format!("invalid side: {}", req.side))),
    };

    let order_type = match req.order_type.as_str() {
        "limit" => matching_engine::OrderType::Limit,
        "market" => matching_engine::OrderType::Market,
        _ => {
            return Err(AppError::BadRequest(format!(
                "invalid order_type: {}",
                req.order_type
            )))
        }
    };

    // TODO: market orders
    let price = req
        .price
        .ok_or(AppError::BadRequest("limit order requires a price".into()))?;

    {
        let mut risk = state.risk.lock().unwrap();
        risk.check_and_lock(req.user_id, &req.symbol, side, price, req.quantity)?;
    }

    let order = Order {
        id: Uuid::new_v4(),
        symbol: req.symbol,
        price: req.price,
        side: book_side,
        order_type: order_type.clone(),
        quantity: req.quantity,
        remaining: req.quantity,
        sequence_id: 0,
        timestamp_us: 0,
    };
    let order_id = order.id;

    let sequenced = {
        let mut seq = state.sequencer.lock().unwrap();
        seq.sequence(order)
    };

    {
        let mut books = state.books.lock().unwrap();
        let book = books
            .entry(sequenced.order.symbol.clone())
            .or_insert_with(|| OrderBook::new(&sequenced.order.symbol));

        let _ = match sequenced.order.order_type.clone() {
            matching_engine::OrderType::Limit => book.add_limit_order(sequenced.order),
            matching_engine::OrderType::Market => book.add_market_order(sequenced.order),
        };
    }

    Ok((StatusCode::ACCEPTED, Json(PlaceOrderResponse { order_id })))
}

/// DELETE /orders/:id — cancel a resting order, release its locked funds.
///
/// `Path(id)` extracts the `:id` path segment and parses it as a Uuid.
pub async fn cancel_order(State(state): State<AppState>, Path(id): Path<Uuid>) -> StatusCode {
    todo!()
}
