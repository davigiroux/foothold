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
        if let Some((key, _)) = self.bids.first_key_value() {
            let Reverse(price) = *key;
            Some(price)
        } else {
            None
        }
    }

    /// Best (lowest) ask price, or None if the book is empty on that side
    pub fn best_ask(&self) -> Option<Price> {
        if let Some((price, _)) = self.asks.first_key_value() {
            Some(*price)
        } else {
            None
        }
    }

    /// Returns ask - bid, or None if either side is empty
    pub fn spread(&self) -> Option<Price> {
        let ask_price = self.best_ask();
        let bid_price = self.best_bid();

        if let (Some(ask), Some(bid)) = (ask_price, bid_price) {
            Some(ask - bid)
        } else {
            None
        }
    }

    /// Insert a resting limit order and run matching.
    /// Returns any trades that result plus the (possibly modified) order.
    pub fn add_limit_order(&mut self, mut order: Order) -> MatchResult {
        let mut trades = vec![];
        let mut incoming_remaining = order.quantity;
        let best_ask = self.best_ask();
        let best_bid = self.best_bid();
        let order_price = order.price.unwrap();
        let mut is_crossable = false;

        if order.side == Side::Bid {
            is_crossable = match best_ask {
                Some(ask) => ask <= order_price,
                None => false,
            };
        } else if order.side == Side::Ask {
            is_crossable = match best_bid {
                Some(bid) => bid >= order_price,
                None => false,
            };
        }

        while incoming_remaining > Decimal::ZERO && is_crossable {
            if order.side == Side::Bid {
                if let Some(mut entry) = self.asks.first_entry() {
                    let queue = entry.get_mut();
                    if let Some(mut resting) = queue.pop_front() {
                        // Has resting order to fill
                        let fill_qty = min(resting.remaining, incoming_remaining);
                        incoming_remaining -= fill_qty;
                        resting.remaining -= fill_qty;
                        let new_trade = Trade {
                            maker_order_id: resting.id,
                            taker_order_id: order.id,
                            price: resting.price.unwrap(),
                            quantity: fill_qty,
                            symbol: order.symbol.clone(),
                            sequence_id: order.sequence_id,
                        };

                        trades.push(new_trade);

                        if resting.remaining > Decimal::ZERO {
                            queue.push_front(resting);
                        } else {
                            self.index.remove(&resting.id);
                            if queue.is_empty() {
                                entry.remove();
                            }
                        }
                    }
                }
                is_crossable = match self.best_ask() {
                    Some(ask) => ask <= order_price,
                    None => false,
                };
            } else if order.side == Side::Ask {
                if let Some(mut entry) = self.bids.first_entry() {
                    let queue = entry.get_mut();
                    if let Some(mut resting) = queue.pop_front() {
                        // Has resting order to fill
                        let fill_qty = min(resting.remaining, incoming_remaining);
                        incoming_remaining -= fill_qty;
                        resting.remaining -= fill_qty;
                        let new_trade = Trade {
                            maker_order_id: resting.id,
                            taker_order_id: order.id,
                            price: resting.price.unwrap(),
                            quantity: fill_qty,
                            symbol: order.symbol.clone(),
                            sequence_id: order.sequence_id,
                        };

                        trades.push(new_trade);

                        if resting.remaining > Decimal::ZERO {
                            queue.push_front(resting);
                        } else {
                            self.index.remove(&resting.id);
                            if queue.is_empty() {
                                entry.remove();
                            }
                        }
                    }
                }
                is_crossable = match self.best_bid() {
                    Some(bid) => bid >= order_price,
                    None => false,
                };
            }
        }

        if incoming_remaining > Decimal::ZERO {
            order.remaining = incoming_remaining;
            self.index.insert(order.id, (order.side, order_price));

            if order.side == Side::Ask {
                self.asks
                    .entry(order_price)
                    .or_insert_with(VecDeque::new)
                    .push_back(order.clone());
            } else {
                self.bids
                    .entry(Reverse(order_price))
                    .or_insert_with(VecDeque::new)
                    .push_back(order.clone());
            }
        }

        return MatchResult {
            trades,
            remaining: if incoming_remaining > Decimal::ZERO {
                Some(order)
            } else {
                None
            },
        };
    }

    /// Execute a market order against the book immediately.
    /// Returns trades. A partially filled market order that exhausted the book
    /// is represented as trades with the order's `remaining` set to the leftover.
    pub fn add_market_order(&mut self, mut order: Order) -> MatchResult {
        let mut trades = vec![];
        let mut incoming_remaining = order.quantity;

        while incoming_remaining > Decimal::ZERO {
            if order.side == Side::Bid {
                if let Some(mut entry) = self.asks.first_entry() {
                    let queue = entry.get_mut();
                    if let Some(mut resting) = queue.pop_front() {
                        // Has resting order to fill
                        let fill_qty = min(resting.remaining, incoming_remaining);
                        incoming_remaining -= fill_qty;
                        resting.remaining -= fill_qty;
                        let new_trade = Trade {
                            maker_order_id: resting.id,
                            taker_order_id: order.id,
                            price: resting.price.unwrap(),
                            quantity: fill_qty,
                            symbol: order.symbol.clone(),
                            sequence_id: order.sequence_id,
                        };

                        trades.push(new_trade);

                        if resting.remaining > Decimal::ZERO {
                            queue.push_front(resting);
                        } else {
                            self.index.remove(&resting.id);
                            if queue.is_empty() {
                                entry.remove();
                            }
                        }
                    }
                } else {
                    break;
                }
            } else if order.side == Side::Ask {
                if let Some(mut entry) = self.bids.first_entry() {
                    let queue = entry.get_mut();
                    if let Some(mut resting) = queue.pop_front() {
                        // Has resting order to fill
                        let fill_qty = min(resting.remaining, incoming_remaining);
                        incoming_remaining -= fill_qty;
                        resting.remaining -= fill_qty;
                        let new_trade = Trade {
                            maker_order_id: resting.id,
                            taker_order_id: order.id,
                            price: resting.price.unwrap(),
                            quantity: fill_qty,
                            symbol: order.symbol.clone(),
                            sequence_id: order.sequence_id,
                        };

                        trades.push(new_trade);

                        if resting.remaining > Decimal::ZERO {
                            queue.push_front(resting);
                        } else {
                            self.index.remove(&resting.id);
                            if queue.is_empty() {
                                entry.remove();
                            }
                        }
                    }
                } else {
                    break;
                }
            }
        }

        order.remaining = incoming_remaining;
        return MatchResult {
            trades,
            remaining: if incoming_remaining > Decimal::ZERO {
                Some(order)
            } else {
                None
            },
        };
    }

    /// Remove a resting order by ID. Returns the cancelled order, or None if
    /// the order was already filled or never existed.
    pub fn cancel_order(&mut self, id: OrderId) -> Option<Order> {
        if let Some((side, price)) = self.index.get(&id).copied() {
            let mut removed: Option<Order> = None;
            let mut empty_queue = false;
            if side == Side::Ask {
                if let Some(queue) = self.asks.get_mut(&price) {
                    if let Some(pos) = queue.iter().position(|o| o.id == id) {
                        removed = queue.remove(pos);

                        if queue.is_empty() {
                            empty_queue = true;
                        }
                    }
                }
                if empty_queue {
                    self.asks.remove(&price);
                }
                self.index.remove(&id);
                removed
            } else {
                if let Some(queue) = self.bids.get_mut(&Reverse(price)) {
                    if let Some(pos) = queue.iter().position(|o| o.id == id) {
                        removed = queue.remove(pos);

                        if queue.is_empty() {
                            empty_queue = true;
                        }
                    }
                }
                if empty_queue {
                    self.bids.remove(&Reverse(price));
                }
                self.index.remove(&id);
                removed
            }
        } else {
            None
        }
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

        Depth {
            bids: bids,
            asks: asks,
        }
    }
}

pub struct Depth {
    pub bids: Vec<(Price, crate::order::Quantity)>,
    pub asks: Vec<(Price, crate::order::Quantity)>,
}
