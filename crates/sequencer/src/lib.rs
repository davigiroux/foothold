use matching_engine::{Order, OrderBook, OrderType};

/// A single entry in the sequence log: an order that has been assigned a
/// global sequence ID and is ready to be fed into the matching engine.
#[derive(Debug, Clone)]
pub struct SequencedOrder {
    pub sequence_id: u64,
    pub order: Order,
}

/// Assigns strictly increasing sequence IDs to incoming orders and records
/// them in an append-only in-memory log.
pub struct Sequencer {
    next_seq: u64,
    pub log: Vec<SequencedOrder>,
}

impl Sequencer {
    pub fn new() -> Self {
        Self {
            next_seq: 1,
            log: vec![],
        }
    }

    /// Assign the next sequence ID to `order`, append to log, return the entry.
    pub fn sequence(&mut self, mut order: Order) -> SequencedOrder {
        order.sequence_id = self.next_seq;
        self.next_seq += 1;
        self.log.push(SequencedOrder {
            sequence_id: order.sequence_id,
            order,
        });
        self.log.last().unwrap().clone()
    }
}

/// Replay the log into a fresh OrderBook and return it.
/// Identical log → identical book state, every time.
pub fn replay(log: &[SequencedOrder], symbol: &str) -> OrderBook {
    let mut book = OrderBook::new(symbol);

    for entry in log {
        match entry.order.order_type {
            OrderType::Limit => book.add_limit_order(entry.order.clone()),
            OrderType::Market => book.add_market_order(entry.order.clone()),
        };
    }
    book
}

#[cfg(test)]
mod tests;
