use std::collections::HashMap;

use rust_decimal::Decimal;
use thiserror::Error;
use uuid::Uuid;

pub type UserId = Uuid;
pub type Symbol = String;

#[derive(Debug, Error)]
pub enum RiskError {
    #[error("user not found")]
    UserNotFound,
    #[error("insufficient funds: need {needed}, have {available}")]
    InsufficientFunds { needed: Decimal, available: Decimal },
    #[error("insufficient stock for {symbol}: need {needed}, have {available}")]
    InsufficientStock {
        symbol: String,
        needed: Decimal,
        available: Decimal,
    },
    #[error("no position in {symbol}")]
    NoPosition { symbol: String },
}

/// Per-symbol stock position split into free and locked quantities.
pub struct Position {
    pub available: Decimal,
    pub locked: Decimal,
}

/// A user's full wallet: cash + per-symbol stock positions.
pub struct Wallet {
    pub cash_available: Decimal,
    pub cash_locked: Decimal,
    pub positions: HashMap<Symbol, Position>,
}

pub struct RiskEngine {
    wallets: HashMap<UserId, Wallet>,
}

impl RiskEngine {
    pub fn new() -> Self {
        Self {
            wallets: HashMap::new(),
        }
    }

    pub fn wallet(&self, user_id: UserId) -> Option<&Wallet> {
        self.wallets.get(&user_id)
    }

    /// Register a wallet for a user with an opening cash balance.
    pub fn add_wallet(&mut self, user_id: UserId, cash: Decimal) {
        self.wallets.insert(
            user_id,
            Wallet {
                cash_available: cash,
                cash_locked: Decimal::ZERO,
                positions: HashMap::new(),
            },
        );
    }

    /// Credit a stock position to a user's wallet (e.g. on deposit).
    pub fn add_position(
        &mut self,
        user_id: UserId,
        symbol: Symbol,
        qty: Decimal,
    ) -> Result<(), RiskError> {
        let wallet = self
            .wallets
            .get_mut(&user_id)
            .ok_or(RiskError::UserNotFound)?;

        wallet.positions.entry(symbol).or_insert(Position {
            available: qty,
            locked: Decimal::ZERO,
        });

        Ok(())
    }

    /// Check the order is fundable and atomically lock the required balance.
    /// Buy  → locks price * quantity cash.
    /// Sell → locks quantity stock for that symbol.
    pub fn check_and_lock(
        &mut self,
        user_id: UserId,
        symbol: &str,
        side: Side,
        price: Decimal,
        quantity: Decimal,
    ) -> Result<(), RiskError> {
        let wallet = self
            .wallets
            .get_mut(&user_id)
            .ok_or(RiskError::UserNotFound)?;

        match side {
            Side::Buy => {
                let needed = price * quantity;

                if wallet.cash_available < needed {
                    return Err(RiskError::InsufficientFunds {
                        needed,
                        available: wallet.cash_available,
                    });
                }

                wallet.cash_available -= needed;
                wallet.cash_locked += needed;

                Ok(())
            }
            Side::Sell => {
                let position = wallet
                    .positions
                    .get_mut(symbol)
                    .ok_or(RiskError::NoPosition {
                        symbol: symbol.to_string(),
                    })?;

                if position.available < quantity {
                    return Err(RiskError::InsufficientStock {
                        symbol: symbol.to_string(),
                        needed: quantity,
                        available: position.available,
                    });
                }

                position.available -= quantity;
                position.locked += quantity;

                Ok(())
            }
        }
    }

    /// Release a previously locked amount (on cancel or fill).
    /// Must mirror the locking logic in check_and_lock exactly.
    pub fn release(
        &mut self,
        user_id: UserId,
        symbol: &str,
        side: Side,
        price: Decimal,
        quantity: Decimal,
    ) -> Result<(), RiskError> {
        let wallet = self
            .wallets
            .get_mut(&user_id)
            .ok_or(RiskError::UserNotFound)?;

        match side {
            Side::Buy => {
                let needed = price * quantity;

                if wallet.cash_locked < needed {
                    return Err(RiskError::InsufficientFunds {
                        needed,
                        available: wallet.cash_locked,
                    });
                }

                wallet.cash_available += needed;
                wallet.cash_locked -= needed;

                Ok(())
            }
            Side::Sell => {
                let position = wallet
                    .positions
                    .get_mut(symbol)
                    .ok_or(RiskError::NoPosition {
                        symbol: symbol.to_string(),
                    })?;

                if position.locked < quantity {
                    return Err(RiskError::InsufficientStock {
                        symbol: symbol.to_string(),
                        needed: quantity,
                        available: position.locked,
                    });
                }

                position.available += quantity;
                position.locked -= quantity;

                Ok(())
            }
        }
    }
}

/// Passed to check_and_lock / release so the engine knows which side to act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

#[cfg(test)]
mod tests;
