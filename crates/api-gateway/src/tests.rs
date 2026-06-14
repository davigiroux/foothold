//! Integration tests for the REST handlers.
//!
//! Each test builds an `AppState`, seeds it (wallets / positions) through the
//! public `risk` handle, then drives a real HTTP request through the router via
//! `oneshot`. Assertions are on the HTTP response and on observable state
//! (locked funds in the risk engine, resting orders in the book).
//!
//! The handlers are `todo!()` until you implement them — these are the red
//! state to make green.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde_json::{json, Value};
use tower::ServiceExt; // brings `.oneshot()` onto the Router
use uuid::Uuid;

use crate::app;
use crate::state::AppState;

// --- helpers ---

fn seed_wallet(state: &AppState, user_id: Uuid, cash: Decimal) {
    state.risk.lock().unwrap().add_wallet(user_id, cash);
}

fn seed_position(state: &AppState, user_id: Uuid, symbol: &str, qty: Decimal) {
    state
        .risk
        .lock()
        .unwrap()
        .add_position(user_id, symbol.to_string(), qty)
        .unwrap();
}

fn post_order(body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/orders")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

fn delete_order(id: Uuid) -> Request<Body> {
    Request::builder()
        .method("DELETE")
        .uri(format!("/orders/{id}"))
        .body(Body::empty())
        .unwrap()
}

/// Drive a request through a fresh router sharing `state`'s Arcs.
async fn call(state: &AppState, req: Request<Body>) -> (StatusCode, Value) {
    let resp = app(state.clone()).oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

fn limit_buy(user_id: Uuid, price: &str, qty: &str) -> Value {
    json!({
        "user_id": user_id,
        "symbol": "AAPL",
        "side": "buy",
        "order_type": "limit",
        "price": price,
        "quantity": qty,
    })
}

fn limit_sell(user_id: Uuid, price: &str, qty: &str) -> Value {
    json!({
        "user_id": user_id,
        "symbol": "AAPL",
        "side": "sell",
        "order_type": "limit",
        "price": price,
        "quantity": qty,
    })
}

// --- place_order ---

#[tokio::test]
async fn place_buy_with_funds_returns_202_and_order_id() {
    let state = AppState::new();
    let user = Uuid::new_v4();
    seed_wallet(&state, user, dec!(1000));

    let (status, body) = call(&state, post_order(limit_buy(user, "100", "5"))).await;

    assert_eq!(status, StatusCode::ACCEPTED);
    assert!(
        body.get("order_id").is_some(),
        "response should carry an order_id"
    );
}

#[tokio::test]
async fn place_buy_insufficient_funds_returns_400() {
    let state = AppState::new();
    let user = Uuid::new_v4();
    seed_wallet(&state, user, dec!(100)); // needs 500

    let (status, _) = call(&state, post_order(limit_buy(user, "100", "5"))).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn place_invalid_side_returns_422() {
    let state = AppState::new();
    let user = Uuid::new_v4();
    seed_wallet(&state, user, dec!(1000));

    let bad = json!({
        "user_id": user, "symbol": "AAPL", "side": "sideways",
        "order_type": "limit", "price": "100", "quantity": "5",
    });
    let (status, _) = call(&state, post_order(bad)).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn place_buy_locks_funds() {
    let state = AppState::new();
    let user = Uuid::new_v4();
    seed_wallet(&state, user, dec!(1000));

    call(&state, post_order(limit_buy(user, "100", "5"))).await; // locks 500

    let risk = state.risk.lock().unwrap();
    let wallet = risk.wallet(user).unwrap();
    assert_eq!(wallet.cash_available, dec!(500));
    assert_eq!(wallet.cash_locked, dec!(500));
}

#[tokio::test]
async fn crossing_orders_match_and_clear_book() {
    let state = AppState::new();
    let seller = Uuid::new_v4();
    let buyer = Uuid::new_v4();
    seed_wallet(&state, seller, dec!(0));
    seed_position(&state, seller, "AAPL", dec!(10));
    seed_wallet(&state, buyer, dec!(2000));

    // resting sell, then a fully-crossing buy
    call(&state, post_order(limit_sell(seller, "100", "10"))).await;
    call(&state, post_order(limit_buy(buyer, "100", "10"))).await;

    let books = state.books.lock().unwrap();
    let book = books.get("AAPL").expect("book should exist after trading");
    assert_eq!(book.best_ask(), None, "ask should be fully consumed");
    assert_eq!(
        book.best_bid(),
        None,
        "buy should fully fill, leaving nothing resting"
    );
}

// --- cancel_order ---

#[tokio::test]
async fn cancel_resting_order_returns_200() {
    let state = AppState::new();
    let user = Uuid::new_v4();
    seed_wallet(&state, user, dec!(1000));

    // non-crossing buy (empty book) -> rests
    let (_, body) = call(&state, post_order(limit_buy(user, "50", "5"))).await;
    let id = Uuid::parse_str(body["order_id"].as_str().unwrap()).unwrap();

    let (status, _) = call(&state, delete_order(id)).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn cancel_releases_locked_funds() {
    let state = AppState::new();
    let user = Uuid::new_v4();
    seed_wallet(&state, user, dec!(1000));

    let (_, body) = call(&state, post_order(limit_buy(user, "50", "5"))).await; // locks 250
    let id = Uuid::parse_str(body["order_id"].as_str().unwrap()).unwrap();

    call(&state, delete_order(id)).await;

    let risk = state.risk.lock().unwrap();
    let wallet = risk.wallet(user).unwrap();
    assert_eq!(wallet.cash_available, dec!(1000));
    assert_eq!(wallet.cash_locked, dec!(0));
}

#[tokio::test]
async fn cancel_unknown_order_returns_404() {
    let state = AppState::new();
    let (status, _) = call(&state, delete_order(Uuid::new_v4())).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn cancel_removes_order_from_book() {
    let state = AppState::new();
    let user = Uuid::new_v4();
    seed_wallet(&state, user, dec!(1000));

    let (_, body) = call(&state, post_order(limit_buy(user, "50", "5"))).await;
    let id = Uuid::parse_str(body["order_id"].as_str().unwrap()).unwrap();

    {
        let books = state.books.lock().unwrap();
        assert_eq!(books.get("AAPL").unwrap().best_bid(), Some(dec!(50)));
    }

    call(&state, delete_order(id)).await;

    let books = state.books.lock().unwrap();
    assert_eq!(books.get("AAPL").unwrap().best_bid(), None);
}
