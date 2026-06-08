use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use uuid::Uuid;

use crate::order::{Order, OrderType, Side};
use crate::order_book::OrderBook;

fn make_limit_order(side: Side, price: Decimal, qty: Decimal, seq: u64) -> Order {
    Order {
        id: Uuid::new_v4(),
        symbol: "AAPL".to_string(),
        side,
        order_type: OrderType::Limit,
        price: Some(price),
        quantity: qty,
        remaining: qty,
        sequence_id: seq,
        timestamp_us: seq,
    }
}

fn make_market_order(side: Side, qty: Decimal, seq: u64) -> Order {
    Order {
        id: Uuid::new_v4(),
        symbol: "AAPL".to_string(),
        side,
        order_type: OrderType::Market,
        price: None,
        quantity: qty,
        remaining: qty,
        sequence_id: seq,
        timestamp_us: seq,
    }
}

// ── best_bid / best_ask / spread ─────────────────────────────────────────────

#[test]
fn empty_book_has_no_best_prices() {
    let book = OrderBook::new("AAPL");
    assert_eq!(book.best_bid(), None);
    assert_eq!(book.best_ask(), None);
    assert_eq!(book.spread(), None);
}

#[test]
fn best_bid_returns_highest_bid() {
    let mut book = OrderBook::new("AAPL");
    book.add_limit_order(make_limit_order(Side::Bid, dec!(99), dec!(10), 1));
    book.add_limit_order(make_limit_order(Side::Bid, dec!(101), dec!(10), 2));
    book.add_limit_order(make_limit_order(Side::Bid, dec!(100), dec!(10), 3));
    assert_eq!(book.best_bid(), Some(dec!(101)));
}

#[test]
fn best_ask_returns_lowest_ask() {
    let mut book = OrderBook::new("AAPL");
    book.add_limit_order(make_limit_order(Side::Ask, dec!(103), dec!(10), 1));
    book.add_limit_order(make_limit_order(Side::Ask, dec!(101), dec!(10), 2));
    book.add_limit_order(make_limit_order(Side::Ask, dec!(102), dec!(10), 3));
    assert_eq!(book.best_ask(), Some(dec!(101)));
}

#[test]
fn spread_is_ask_minus_bid() {
    let mut book = OrderBook::new("AAPL");
    book.add_limit_order(make_limit_order(Side::Bid, dec!(100), dec!(10), 1));
    book.add_limit_order(make_limit_order(Side::Ask, dec!(102), dec!(10), 2));
    assert_eq!(book.spread(), Some(dec!(2)));
}

// ── add_limit_order: resting (no cross) ──────────────────────────────────────

#[test]
fn bid_below_ask_rests_in_book() {
    let mut book = OrderBook::new("AAPL");
    book.add_limit_order(make_limit_order(Side::Ask, dec!(102), dec!(10), 1));
    let result = book.add_limit_order(make_limit_order(Side::Bid, dec!(101), dec!(5), 2));
    assert!(result.trades.is_empty());
    assert!(result.remaining.is_some());
    assert_eq!(book.best_bid(), Some(dec!(101)));
}

#[test]
fn ask_above_bid_rests_in_book() {
    let mut book = OrderBook::new("AAPL");
    book.add_limit_order(make_limit_order(Side::Bid, dec!(100), dec!(10), 1));
    let result = book.add_limit_order(make_limit_order(Side::Ask, dec!(101), dec!(5), 2));
    assert!(result.trades.is_empty());
    assert!(result.remaining.is_some());
    assert_eq!(book.best_ask(), Some(dec!(101)));
}

// ── add_limit_order: full cross ───────────────────────────────────────────────

#[test]
fn incoming_bid_fully_matches_resting_ask() {
    let mut book = OrderBook::new("AAPL");
    book.add_limit_order(make_limit_order(Side::Ask, dec!(100), dec!(10), 1));
    let result = book.add_limit_order(make_limit_order(Side::Bid, dec!(100), dec!(10), 2));
    assert_eq!(result.trades.len(), 1);
    assert_eq!(result.trades[0].quantity, dec!(10));
    assert_eq!(result.trades[0].price, dec!(100));
    assert!(result.remaining.is_none());
    assert_eq!(book.best_ask(), None);
}

#[test]
fn incoming_ask_fully_matches_resting_bid() {
    let mut book = OrderBook::new("AAPL");
    book.add_limit_order(make_limit_order(Side::Bid, dec!(100), dec!(10), 1));
    let result = book.add_limit_order(make_limit_order(Side::Ask, dec!(100), dec!(10), 2));
    assert_eq!(result.trades.len(), 1);
    assert_eq!(result.trades[0].quantity, dec!(10));
    assert!(result.remaining.is_none());
    assert_eq!(book.best_bid(), None);
}

// ── add_limit_order: partial cross ───────────────────────────────────────────

#[test]
fn incoming_larger_than_resting_remainder_rests() {
    let mut book = OrderBook::new("AAPL");
    book.add_limit_order(make_limit_order(Side::Ask, dec!(100), dec!(5), 1));
    let result = book.add_limit_order(make_limit_order(Side::Bid, dec!(100), dec!(10), 2));
    assert_eq!(result.trades.len(), 1);
    assert_eq!(result.trades[0].quantity, dec!(5));
    let remaining = result.remaining.unwrap();
    assert_eq!(remaining.remaining, dec!(5));
    assert_eq!(book.best_bid(), Some(dec!(100)));
    assert_eq!(book.best_ask(), None);
}

#[test]
fn incoming_smaller_than_resting_resting_partially_stays() {
    let mut book = OrderBook::new("AAPL");
    book.add_limit_order(make_limit_order(Side::Ask, dec!(100), dec!(10), 1));
    let result = book.add_limit_order(make_limit_order(Side::Bid, dec!(100), dec!(4), 2));
    assert_eq!(result.trades.len(), 1);
    assert_eq!(result.trades[0].quantity, dec!(4));
    assert!(result.remaining.is_none());
    let depth = book.depth(1);
    assert_eq!(depth.asks[0].1, dec!(6));
}

// ── price priority ────────────────────────────────────────────────────────────

#[test]
fn bid_fills_cheapest_ask_first() {
    let mut book = OrderBook::new("AAPL");
    book.add_limit_order(make_limit_order(Side::Ask, dec!(102), dec!(5), 1));
    book.add_limit_order(make_limit_order(Side::Ask, dec!(101), dec!(5), 2));
    let result = book.add_limit_order(make_limit_order(Side::Bid, dec!(102), dec!(5), 3));
    assert_eq!(result.trades.len(), 1);
    assert_eq!(result.trades[0].price, dec!(101));
}

#[test]
fn ask_fills_highest_bid_first() {
    let mut book = OrderBook::new("AAPL");
    book.add_limit_order(make_limit_order(Side::Bid, dec!(99), dec!(5), 1));
    book.add_limit_order(make_limit_order(Side::Bid, dec!(100), dec!(5), 2));
    let result = book.add_limit_order(make_limit_order(Side::Ask, dec!(99), dec!(5), 3));
    assert_eq!(result.trades.len(), 1);
    assert_eq!(result.trades[0].price, dec!(100));
}

// ── FIFO / time priority ──────────────────────────────────────────────────────

#[test]
fn fifo_at_same_price_earlier_order_fills_first() {
    let mut book = OrderBook::new("AAPL");
    let first = make_limit_order(Side::Ask, dec!(100), dec!(5), 1);
    let second = make_limit_order(Side::Ask, dec!(100), dec!(5), 2);
    let first_id = first.id;
    book.add_limit_order(first);
    book.add_limit_order(second);
    let result = book.add_limit_order(make_limit_order(Side::Bid, dec!(100), dec!(5), 3));
    assert_eq!(result.trades.len(), 1);
    assert_eq!(result.trades[0].maker_order_id, first_id);
}

// ── add_market_order ──────────────────────────────────────────────────────────

#[test]
fn market_bid_fully_filled() {
    let mut book = OrderBook::new("AAPL");
    book.add_limit_order(make_limit_order(Side::Ask, dec!(100), dec!(10), 1));
    let result = book.add_market_order(make_market_order(Side::Bid, dec!(10), 2));
    assert_eq!(result.trades.len(), 1);
    assert_eq!(result.trades[0].quantity, dec!(10));
    assert!(result.remaining.is_none());
}

#[test]
fn market_bid_partially_filled_book_exhausted() {
    let mut book = OrderBook::new("AAPL");
    book.add_limit_order(make_limit_order(Side::Ask, dec!(100), dec!(5), 1));
    let result = book.add_market_order(make_market_order(Side::Bid, dec!(10), 2));
    assert_eq!(result.trades.len(), 1);
    assert_eq!(result.trades[0].quantity, dec!(5));
    let remaining = result.remaining.unwrap();
    assert_eq!(remaining.remaining, dec!(5));
}

#[test]
fn market_bid_on_empty_book_returns_order() {
    let mut book = OrderBook::new("AAPL");
    let result = book.add_market_order(make_market_order(Side::Bid, dec!(10), 1));
    assert!(result.trades.is_empty());
    assert!(result.remaining.is_some());
}

// ── cancel_order ──────────────────────────────────────────────────────────────

#[test]
fn cancel_resting_ask_removes_it() {
    let mut book = OrderBook::new("AAPL");
    let order = make_limit_order(Side::Ask, dec!(100), dec!(10), 1);
    let id = order.id;
    book.add_limit_order(order);
    let cancelled = book.cancel_order(id);
    assert!(cancelled.is_some());
    assert_eq!(book.best_ask(), None);
}

#[test]
fn cancel_resting_bid_removes_it() {
    let mut book = OrderBook::new("AAPL");
    let order = make_limit_order(Side::Bid, dec!(100), dec!(10), 1);
    let id = order.id;
    book.add_limit_order(order);
    let cancelled = book.cancel_order(id);
    assert!(cancelled.is_some());
    assert_eq!(book.best_bid(), None);
}

#[test]
fn cancel_unknown_id_returns_none() {
    let mut book = OrderBook::new("AAPL");
    assert!(book.cancel_order(Uuid::new_v4()).is_none());
}

#[test]
fn cancel_last_order_at_level_removes_price_level() {
    let mut book = OrderBook::new("AAPL");
    let order = make_limit_order(Side::Ask, dec!(100), dec!(10), 1);
    let id = order.id;
    book.add_limit_order(order);
    book.cancel_order(id);
    let depth = book.depth(10);
    assert!(depth.asks.is_empty());
}

// ── depth ─────────────────────────────────────────────────────────────────────

#[test]
fn depth_empty_book() {
    let book = OrderBook::new("AAPL");
    let depth = book.depth(5);
    assert!(depth.bids.is_empty());
    assert!(depth.asks.is_empty());
}

#[test]
fn depth_capped_at_levels() {
    let mut book = OrderBook::new("AAPL");
    for price in [dec!(101), dec!(102), dec!(103)] {
        book.add_limit_order(make_limit_order(Side::Ask, price, dec!(10), 1));
    }
    let depth = book.depth(2);
    assert_eq!(depth.asks.len(), 2);
    assert_eq!(depth.asks[0].0, dec!(101));
    assert_eq!(depth.asks[1].0, dec!(102));
}

#[test]
fn depth_fewer_levels_than_requested() {
    let mut book = OrderBook::new("AAPL");
    book.add_limit_order(make_limit_order(Side::Ask, dec!(101), dec!(10), 1));
    book.add_limit_order(make_limit_order(Side::Ask, dec!(102), dec!(10), 2));
    let depth = book.depth(10);
    assert_eq!(depth.asks.len(), 2);
}

#[test]
fn depth_sums_quantities_at_level() {
    let mut book = OrderBook::new("AAPL");
    book.add_limit_order(make_limit_order(Side::Ask, dec!(100), dec!(3), 1));
    book.add_limit_order(make_limit_order(Side::Ask, dec!(100), dec!(7), 2));
    let depth = book.depth(1);
    assert_eq!(depth.asks[0].1, dec!(10));
}
