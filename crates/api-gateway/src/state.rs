use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use matching_engine::{OrderBook, Symbol};
use risk_engine::RiskEngine;
use rust_decimal::Decimal;
use sequencer::Sequencer;
use uuid::Uuid;

/// Shared application state, cloned into every request handler.
///
/// Each field is wrapped in `Arc<Mutex<_>>`:
///   - `Arc`   = shared ownership across threads (handlers run concurrently)
///   - `Mutex` = exclusive access when mutating (one writer at a time)
///
/// `#[derive(Clone)]` clones the Arcs (cheap pointer bumps), not the inner data.
#[derive(Clone)]
pub struct AppState {
    pub risk: Arc<Mutex<RiskEngine>>,
    pub sequencer: Arc<Mutex<Sequencer>>,
    pub books: Arc<Mutex<HashMap<Symbol, OrderBook>>>,
    pub registry: Arc<Mutex<HashMap<Uuid, OpenOrder>>>,
}

pub struct OpenOrder {
    pub user_id: Uuid,
    pub symbol: String,
    pub side: risk_engine::Side,
    pub price: Decimal,
    pub quantity: Decimal,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            risk: Arc::new(Mutex::new(RiskEngine::new())),
            sequencer: Arc::new(Mutex::new(Sequencer::new())),
            books: Arc::new(Mutex::new(HashMap::new())),
            registry: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
