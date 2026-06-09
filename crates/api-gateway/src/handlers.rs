use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::models::{PlaceOrderRequest, PlaceOrderResponse};
use crate::state::AppState;

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
) -> (StatusCode, Json<PlaceOrderResponse>) {
    todo!()
}

/// DELETE /orders/:id — cancel a resting order, release its locked funds.
///
/// `Path(id)` extracts the `:id` path segment and parses it as a Uuid.
pub async fn cancel_order(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> StatusCode {
    todo!()
}
