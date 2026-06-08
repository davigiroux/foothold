use criterion::{criterion_group, criterion_main, Criterion};
use matching_engine::order_book::OrderBook;
use matching_engine::{Order, OrderType, Side};
use rust_decimal_macros::dec;
use uuid::Uuid;

fn make_limit_order(
    side: Side,
    price: rust_decimal::Decimal,
    qty: rust_decimal::Decimal,
    seq: u64,
) -> Order {
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

fn bench_limit_order_fill(c: &mut Criterion) {
    c.bench_function("limit order — single fill", |b| {
        b.iter_batched(
            // Setup: runs before each measured iteration, not included in timing
            || {
                let mut book = OrderBook::new("AAPL");
                // TODO: add a resting ask so the incoming bid crosses it
                // hint: make_limit_order(Side::Ask, dec!(100), dec!(10), 1)
                book.add_limit_order(make_limit_order(Side::Ask, dec!(100), dec!(10), 1));
                book
            },
            // Measured: this is what gets timed
            |mut book| book.add_limit_order(make_limit_order(Side::Bid, dec!(100), dec!(10), 2)),
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, bench_limit_order_fill);
criterion_main!(benches);
