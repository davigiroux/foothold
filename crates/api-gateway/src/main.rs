// Entry point: builds the router, wires shared state, binds the TCP listener.
//
// REST:      POST /orders  (202 Accepted), DELETE /orders/:id
// WebSocket: execution reports + market data (later rung)

mod handlers;
mod models;
mod state;

#[cfg(test)]
mod tests;

use axum::{
    routing::{delete, post},
    Router,
};
use state::AppState;

#[tokio::main]
async fn main() {
    // 1. build shared state (wraps the engine/sequencer/risk components)
    // 2. build the Router, mapping routes -> handlers, attaching state
    // 3. bind a TcpListener and serve
    let state = AppState::new();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    let _ = axum::serve(listener, app(state));
}

/// Assembles the route table. Split out from main so it can be reused in tests.
fn app(state: AppState) -> Router {
    Router::new()
        .route("/orders", post(handlers::place_order))
        .route("/orders/:id", delete(handlers::cancel_order))
        .with_state(state)
}
