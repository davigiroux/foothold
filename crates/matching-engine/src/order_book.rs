use std::cmp::{min, Reverse};
use std::collections::{BTreeMap, HashMap, VecDeque};

use rust_decimal::Decimal;

use crate::order::{Order, OrderId, Price, Side};
use crate::{MatchResult, Quantity, Trade};

/// Bids: highest price first  → keyed by Reverse<Price>
/// Asks: lowest price first   → keyed by Price (natural BTreeMap order)
pub struct OrderBook {
    pub symbol: String,
    bids: BTreeMap<Reverse<Price>, VecDeque<Order>>,
    asks: BTreeMap<Price, VecDeque<Order>>,
    /// O(1) cancel: maps OrderId to the price level it lives at
    index: HashMap<OrderId, (Side, Price)>,
}

impl OrderBook {
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            index: HashMap::new(),
        }
    }

    /// Best (highest) bid price, or None if the book is empty on that side
    pub fn best_bid(&self) -> Option<Price> {
        self.bids.first_key_value().map(|(key, _)| {
            let Reverse(price) = *key;
            price
        })
    }

    /// Best (lowest) ask price, or None if the book is empty on that side
    pub fn best_ask(&self) -> Option<Price> {
        self.asks.first_key_value().map(|(price, _)| *price)
    }

    /// Returns ask - bid, or None if either side is empty
    pub fn spread(&self) -> Option<Price> {
        self.best_ask().zip(self.best_bid()).map(|(ask, bid)| ask - bid)
    }

    fn is_crossable(&self, side: Side, order_price: Price) -> bool {
        match side {
            Side::Bid => self.best_ask().map_or(false, |ask| ask <= order_price),
            Side::Ask => self.best_bid().map_or(false, |bid| bid >= order_price),
        }
    }

    /// Fills the front resting order in `queue` against an incoming order.
    /// Returns the trade and whether the price level is now empty.
    fn fill_resting(
        index: &mut HashMap<OrderId, (Side, Price)>,
        queue: &mut VecDeque<Order>,
        incoming_remaining: &mut Decimal,
        taker_id: OrderId,
        taker_symbol: &str,
        taker_seq: u64,
    ) -> (Trade, bool) {
        let mut resting = queue
            .pop_front()
            .expect("fill_resting called on empty queue");
        let fill_qty = min(resting.remaining, *incoming_remaining);
        *incoming_remaining -= fill_qty;
        resting.remaining -= fill_qty;
        let trade = Trade {
            maker_order_id: resting.id,
            taker_order_id: taker_id,
            price: resting.price.unwrap(),
            quantity: fill_qty,
            symbol: taker_symbol.to_string(),
            sequence_id: taker_seq,
        };
        if resting.remaining > Decimal::ZERO {
            queue.push_front(resting);
        } else {
            index.remove(&resting.id);
        }
        (trade, queue.is_empty())
    }

    /// Insert a resting limit order and run matching.
    /// Returns any trades that result plus the (possibly modified) order.
    pub fn add_limit_order(&mut self, mut order: Order) -> MatchResult {
        let mut trades = vec![];
        let mut incoming_remaining = order.quantity;
        let order_price = order.price.unwrap();

        while incoming_remaining > Decimal::ZERO && self.is_crossable(order.side, order_price) {
            match order.side {
                Side::Bid => {
                    if let Some(mut entry) = self.asks.first_entry() {
                        let (trade, level_empty) = Self::fill_resting(
                            &mut self.index,
                            entry.get_mut(),
                            &mut incoming_remaining,
                            order.id,
                            &order.symbol,
                            order.sequence_id,
                        );
                        trades.push(trade);
                        if level_empty {
                            entry.remove();
                        }
                    }
                }
                Side::Ask => {
                    if let Some(mut entry) = self.bids.first_entry() {
                        let (trade, level_empty) = Self::fill_resting(
                            &mut self.index,
                            entry.get_mut(),
                            &mut incoming_remaining,
                            order.id,
                            &order.symbol,
                            order.sequence_id,
                        );
                        trades.push(trade);
                        if level_empty {
                            entry.remove();
                        }
                    }
                }
            }
        }

        if incoming_remaining > Decimal::ZERO {
            order.remaining = incoming_remaining;
            self.index.insert(order.id, (order.side, order_price));
            match order.side {
                Side::Ask => self.asks
                    .entry(order_price)
                    .or_insert_with(VecDeque::new)
                    .push_back(order.clone()),
                Side::Bid => self.bids
                    .entry(Reverse(order_price))
                    .or_insert_with(VecDeque::new)
                    .push_back(order.clone()),
            }
        }

        MatchResult {
            trades,
            remaining: (incoming_remaining > Decimal::ZERO).then_some(order),
        }
    }

    /// Execute a market order against the book immediately.
    /// Returns trades. A partially filled market order that exhausted the book
    /// is represented as trades with the order's `remaining` set to the leftover.
    pub fn add_market_order(&mut self, mut order: Order) -> MatchResult {
        let mut trades = vec![];
        let mut incoming_remaining = order.quantity;

        while incoming_remaining > Decimal::ZERO {
            let exhausted = match order.side {
                Side::Bid => match self.asks.first_entry() {
                    Some(mut entry) => {
                        let (trade, level_empty) = Self::fill_resting(
                            &mut self.index,
                            entry.get_mut(),
                            &mut incoming_remaining,
                            order.id,
                            &order.symbol,
                            order.sequence_id,
                        );
                        trades.push(trade);
                        if level_empty {
                            entry.remove();
                        }
                        false
                    }
                    None => true,
                },
                Side::Ask => match self.bids.first_entry() {
                    Some(mut entry) => {
                        let (trade, level_empty) = Self::fill_resting(
                            &mut self.index,
                            entry.get_mut(),
                            &mut incoming_remaining,
                            order.id,
                            &order.symbol,
                            order.sequence_id,
                        );
                        trades.push(trade);
                        if level_empty {
                            entry.remove();
                        }
                        false
                    }
                    None => true,
                },
            };
            if exhausted {
                break;
            }
        }

        order.remaining = incoming_remaining;
        MatchResult {
            trades,
            remaining: (incoming_remaining > Decimal::ZERO).then_some(order),
        }
    }

    /// Remove a resting order by ID. Returns the cancelled order, or None if
    /// the order was already filled or never existed.
    pub fn cancel_order(&mut self, id: OrderId) -> Option<Order> {
        let (side, price) = self.index.get(&id).copied()?;

        let (removed, empty) = match side {
            Side::Ask => {
                let queue = self.asks.get_mut(&price)?;
                let pos = queue.iter().position(|o| o.id == id)?;
                let order = queue.remove(pos)?;
                (order, queue.is_empty())
            }
            Side::Bid => {
                let queue = self.bids.get_mut(&Reverse(price))?;
                let pos = queue.iter().position(|o| o.id == id)?;
                let order = queue.remove(pos)?;
                (order, queue.is_empty())
            }
        };

        if empty {
            match side {
                Side::Ask => { self.asks.remove(&price); }
                Side::Bid => { self.bids.remove(&Reverse(price)); }
            }
        }
        self.index.remove(&id);
        Some(removed)
    }

    /// Depth snapshot: the top `levels` price levels on each side.
    /// Each entry is (price, total_quantity_at_that_level).
    pub fn depth(&self, levels: usize) -> Depth {
        let asks = self
            .asks
            .iter()
            .map(|(&price, queue)| {
                let total: Quantity = queue.iter().map(|o| o.remaining).sum();
                (price, total)
            })
            .take(levels)
            .collect();

        let bids = self
            .bids
            .iter()
            .map(|(key, queue)| {
                let Reverse(price) = *key;
                let total: Quantity = queue.iter().map(|o| o.remaining).sum();
                (price, total)
            })
            .take(levels)
            .collect();

        Depth { bids, asks }
    }
}

pub struct Depth {
    pub bids: Vec<(Price, crate::order::Quantity)>,
    pub asks: Vec<(Price, crate::order::Quantity)>,
}
