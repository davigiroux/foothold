#[cfg(test)]
mod tests {
    use matching_engine::order::{Order, OrderType, Side};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use uuid::Uuid;

    use crate::{replay, Sequencer};

    fn make_limit_order(side: Side, price: Decimal, qty: Decimal) -> Order {
        Order {
            id: Uuid::new_v4(),
            symbol: "AAPL".to_string(),
            side,
            order_type: OrderType::Limit,
            price: Some(price),
            quantity: qty,
            remaining: qty,
            sequence_id: 0, // sequencer will overwrite this
            timestamp_us: 0,
        }
    }

    // --- Sequencer ---

    #[test]
    fn sequence_ids_are_strictly_increasing() {
        let mut seq = Sequencer::new();
        seq.sequence(make_limit_order(Side::Ask, dec!(100), dec!(10)));
        seq.sequence(make_limit_order(Side::Bid, dec!(100), dec!(10)));
        assert_eq!(seq.log[0].sequence_id, 1);
        assert_eq!(seq.log[1].sequence_id, 2);
    }

    #[test]
    fn sequence_overwrites_incoming_sequence_id() {
        let mut seq = Sequencer::new();
        let mut order = make_limit_order(Side::Ask, dec!(100), dec!(10));
        order.sequence_id = 999; // caller-supplied value must be ignored
        seq.sequence(order);
        assert_eq!(seq.log[0].sequence_id, 1);
        assert_eq!(seq.log[0].order.sequence_id, 1);
    }

    #[test]
    fn log_grows_with_each_sequenced_order() {
        let mut seq = Sequencer::new();
        seq.sequence(make_limit_order(Side::Ask, dec!(100), dec!(5)));
        seq.sequence(make_limit_order(Side::Ask, dec!(101), dec!(5)));
        seq.sequence(make_limit_order(Side::Bid, dec!(99), dec!(5)));
        assert_eq!(seq.log.len(), 3);
    }

    // --- Replay ---

    #[test]
    fn replay_empty_log_returns_empty_book() {
        let book = replay(&[], "AAPL");
        assert!(book.best_bid().is_none());
        assert!(book.best_ask().is_none());
    }

    #[test]
    fn replay_restores_resting_orders() {
        let mut seq = Sequencer::new();
        seq.sequence(make_limit_order(Side::Ask, dec!(101), dec!(10)));
        seq.sequence(make_limit_order(Side::Bid, dec!(99), dec!(5)));

        let book = replay(&seq.log, "AAPL");
        assert_eq!(book.best_ask(), Some(dec!(101)));
        assert_eq!(book.best_bid(), Some(dec!(99)));
    }

    #[test]
    fn replay_is_deterministic() {
        let mut seq = Sequencer::new();
        seq.sequence(make_limit_order(Side::Ask, dec!(100), dec!(10)));
        seq.sequence(make_limit_order(Side::Ask, dec!(101), dec!(5)));
        seq.sequence(make_limit_order(Side::Bid, dec!(99), dec!(8)));

        let book_a = replay(&seq.log, "AAPL");
        let book_b = replay(&seq.log, "AAPL");

        assert_eq!(book_a.best_ask(), book_b.best_ask());
        assert_eq!(book_a.best_bid(), book_b.best_bid());
        assert_eq!(book_a.spread(), book_b.spread());
    }

    #[test]
    fn replay_reproduces_fills() {
        let mut seq = Sequencer::new();
        seq.sequence(make_limit_order(Side::Ask, dec!(100), dec!(10)));
        seq.sequence(make_limit_order(Side::Bid, dec!(100), dec!(10)));

        let book = replay(&seq.log, "AAPL");
        assert!(book.best_ask().is_none());
        assert!(book.best_bid().is_none());
    }

    #[test]
    fn replay_depth_matches_original() {
        let mut seq = Sequencer::new();
        seq.sequence(make_limit_order(Side::Ask, dec!(101), dec!(3)));
        seq.sequence(make_limit_order(Side::Ask, dec!(102), dec!(7)));
        seq.sequence(make_limit_order(Side::Bid, dec!(99), dec!(4)));

        let book = replay(&seq.log, "AAPL");
        let depth = book.depth(5);
        assert_eq!(depth.asks.len(), 2);
        assert_eq!(depth.bids.len(), 1);
        assert_eq!(depth.asks[0], (dec!(101), dec!(3)));
        assert_eq!(depth.asks[1], (dec!(102), dec!(7)));
    }
}
