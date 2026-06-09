#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;
    use uuid::Uuid;

    use crate::{RiskEngine, RiskError, Side};

    fn user() -> uuid::Uuid {
        Uuid::new_v4()
    }

    // --- add_wallet / add_position ---

    #[test]
    fn add_position_unknown_user_returns_error() {
        let mut engine = RiskEngine::new();
        let result = engine.add_position(user(), "AAPL".to_string(), dec!(10));
        assert!(matches!(result, Err(RiskError::UserNotFound)));
    }

    #[test]
    fn add_position_known_user_succeeds() {
        let mut engine = RiskEngine::new();
        let id = user();
        engine.add_wallet(id, dec!(1000));
        assert!(engine.add_position(id, "AAPL".to_string(), dec!(10)).is_ok());
    }

    // --- check_and_lock: Buy ---

    #[test]
    fn buy_lock_succeeds_with_sufficient_funds() {
        let mut engine = RiskEngine::new();
        let id = user();
        engine.add_wallet(id, dec!(1000));
        assert!(engine.check_and_lock(id, "AAPL", Side::Buy, dec!(100), dec!(5)).is_ok());
    }

    #[test]
    fn buy_lock_moves_cash_to_locked() {
        let mut engine = RiskEngine::new();
        let id = user();
        engine.add_wallet(id, dec!(1000));
        engine.check_and_lock(id, "AAPL", Side::Buy, dec!(100), dec!(5)).unwrap();
        let wallet = engine.wallet(id).unwrap();
        assert_eq!(wallet.cash_available, dec!(500));
        assert_eq!(wallet.cash_locked, dec!(500));
    }

    #[test]
    fn buy_lock_fails_with_insufficient_funds() {
        let mut engine = RiskEngine::new();
        let id = user();
        engine.add_wallet(id, dec!(100));
        let result = engine.check_and_lock(id, "AAPL", Side::Buy, dec!(100), dec!(5));
        assert!(matches!(result, Err(RiskError::InsufficientFunds { .. })));
    }

    #[test]
    fn buy_lock_unknown_user_returns_error() {
        let mut engine = RiskEngine::new();
        let result = engine.check_and_lock(user(), "AAPL", Side::Buy, dec!(100), dec!(1));
        assert!(matches!(result, Err(RiskError::UserNotFound)));
    }

    // --- check_and_lock: Sell ---

    #[test]
    fn sell_lock_succeeds_with_sufficient_stock() {
        let mut engine = RiskEngine::new();
        let id = user();
        engine.add_wallet(id, dec!(0));
        engine.add_position(id, "AAPL".to_string(), dec!(10)).unwrap();
        assert!(engine.check_and_lock(id, "AAPL", Side::Sell, dec!(100), dec!(5)).is_ok());
    }

    #[test]
    fn sell_lock_moves_stock_to_locked() {
        let mut engine = RiskEngine::new();
        let id = user();
        engine.add_wallet(id, dec!(0));
        engine.add_position(id, "AAPL".to_string(), dec!(10)).unwrap();
        engine.check_and_lock(id, "AAPL", Side::Sell, dec!(100), dec!(5)).unwrap();
        let wallet = engine.wallet(id).unwrap();
        let pos = wallet.positions.get("AAPL").unwrap();
        assert_eq!(pos.available, dec!(5));
        assert_eq!(pos.locked, dec!(5));
    }

    #[test]
    fn sell_lock_fails_with_insufficient_stock() {
        let mut engine = RiskEngine::new();
        let id = user();
        engine.add_wallet(id, dec!(0));
        engine.add_position(id, "AAPL".to_string(), dec!(3)).unwrap();
        let result = engine.check_and_lock(id, "AAPL", Side::Sell, dec!(100), dec!(5));
        assert!(matches!(result, Err(RiskError::InsufficientStock { .. })));
    }

    #[test]
    fn sell_lock_fails_with_no_position() {
        let mut engine = RiskEngine::new();
        let id = user();
        engine.add_wallet(id, dec!(1000));
        let result = engine.check_and_lock(id, "AAPL", Side::Sell, dec!(100), dec!(1));
        assert!(matches!(result, Err(RiskError::NoPosition { .. })));
    }

    // --- release: Buy ---

    #[test]
    fn buy_release_restores_cash() {
        let mut engine = RiskEngine::new();
        let id = user();
        engine.add_wallet(id, dec!(1000));
        engine.check_and_lock(id, "AAPL", Side::Buy, dec!(100), dec!(5)).unwrap();
        engine.release(id, "AAPL", Side::Buy, dec!(100), dec!(5)).unwrap();
        let wallet = engine.wallet(id).unwrap();
        assert_eq!(wallet.cash_available, dec!(1000));
        assert_eq!(wallet.cash_locked, dec!(0));
    }

    #[test]
    fn buy_release_fails_if_nothing_locked() {
        let mut engine = RiskEngine::new();
        let id = user();
        engine.add_wallet(id, dec!(1000));
        let result = engine.release(id, "AAPL", Side::Buy, dec!(100), dec!(5));
        assert!(matches!(result, Err(RiskError::InsufficientFunds { .. })));
    }

    // --- release: Sell ---

    #[test]
    fn sell_release_restores_stock() {
        let mut engine = RiskEngine::new();
        let id = user();
        engine.add_wallet(id, dec!(0));
        engine.add_position(id, "AAPL".to_string(), dec!(10)).unwrap();
        engine.check_and_lock(id, "AAPL", Side::Sell, dec!(100), dec!(5)).unwrap();
        engine.release(id, "AAPL", Side::Sell, dec!(100), dec!(5)).unwrap();
        let wallet = engine.wallet(id).unwrap();
        let pos = wallet.positions.get("AAPL").unwrap();
        assert_eq!(pos.available, dec!(10));
        assert_eq!(pos.locked, dec!(0));
    }

    #[test]
    fn sell_release_fails_if_nothing_locked() {
        let mut engine = RiskEngine::new();
        let id = user();
        engine.add_wallet(id, dec!(0));
        engine.add_position(id, "AAPL".to_string(), dec!(10)).unwrap();
        let result = engine.release(id, "AAPL", Side::Sell, dec!(100), dec!(5));
        assert!(matches!(result, Err(RiskError::InsufficientStock { .. })));
    }
}
