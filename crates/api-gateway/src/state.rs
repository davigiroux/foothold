use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use matching_engine::{OrderBook, Symbol};
use risk_engine::RiskEngine;
use sequencer::Sequencer;

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
}

impl AppState {
    pub fn new() -> Self {
        Self {
            risk: Arc::new(Mutex::new(RiskEngine::new())),
            sequencer: Arc::new(Mutex::new(Sequencer::new())),
            books: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
