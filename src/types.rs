//! Core domain types for the payments engine.

use std::{
    fmt,
    ops::{Add, Sub},
};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Maximum number of decimal places allowed for amounts.
const AMOUNT_SCALE: u32 = 4;

/// A monetary amount with exactly 4 decimal places precision.
///
/// Uses `rust_decimal::Decimal` internally to avoid floating-point errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Amount(Decimal);

impl Amount {
    /// Creates a new Amount from a Decimal.
    ///
    /// Returns `None` if the decimal has more than 4 decimal places.
    pub fn new(value: Decimal) -> Option<Self> {
        if value.scale() > AMOUNT_SCALE {
            return None;
        }
        Some(Self(value))
    }

    /// Returns true if this amount is negative.
    pub fn is_negative(&self) -> bool {
        self.0.is_sign_negative() && !self.0.is_zero()
    }

    /// Returns true if this amount is zero.
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// Returns the underlying Decimal value.
    pub fn as_decimal(&self) -> Decimal {
        self.0
    }
}

impl Add for Amount {
    type Output = Option<Amount>;

    fn add(self, rhs: Self) -> Self::Output {
        self.0.checked_add(rhs.0).map(Amount)
    }
}

impl Sub for Amount {
    type Output = Option<Amount>;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0.checked_sub(rhs.0).map(Amount)
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4}", self.0)
    }
}

/// A unique client identifier.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct ClientId(pub u16);

/// A unique transaction identifier.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct TxId(pub u32);

/// The type of transaction being processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

/// A transaction to be processed by the engine.
#[derive(Debug, Clone)]
pub struct Transaction {
    pub tx_type: TransactionType,
    pub client_id: ClientId,
    pub tx_id: TxId,
    /// Amount is required for deposit/withdrawal, empty for
    /// dispute/resolve/chargeback.
    pub amount: Option<Amount>,
}

/// A client account with available and held balances.
#[derive(Debug, Clone)]
pub struct Account {
    pub client_id: ClientId,
    pub available: Amount,
    pub held: Amount,
    pub locked: bool,
}

impl Account {
    /// Creates a new account for the given client with zero balances.
    pub fn new(client_id: ClientId) -> Self {
        Self {
            client_id,
            available: Amount::default(),
            held: Amount::default(),
            locked: false,
        }
    }

    /// Returns the total balance (available + held).
    pub fn total(&self) -> Amount {
        // NOTE: Cannot overflow for valid account states since both components
        // are non-negative and bounded by deposit limits.
        (self.available + self.held).unwrap_or_default()
    }
}

impl Default for Account {
    fn default() -> Self {
        Self {
            client_id: ClientId(0),
            available: Amount::default(),
            held: Amount::default(),
            locked: false,
        }
    }
}

/// The dispute state of a deposit transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisputeState {
    /// Transaction has not been disputed.
    #[default]
    None,
    /// Transaction is currently under dispute.
    Disputed,
    /// Dispute has been resolved (can be re-disputed).
    Resolved,
    /// Chargeback completed (terminal state).
    ChargedBack,
}

/// Record of a deposit transaction for dispute lookup.
#[derive(Debug, Clone)]
pub struct DepositRecord {
    pub client_id: ClientId,
    pub amount: Amount,
    pub dispute_state: DisputeState,
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    #[test]
    fn amount_display_formats_four_decimals() {
        let amt = Amount::new(dec!(1.0)).unwrap();
        assert_eq!(format!("{}", amt), "1.0000");

        let amt2 = Amount::new(dec!(123.4567)).unwrap();
        assert_eq!(format!("{}", amt2), "123.4567");
    }

    #[test]
    fn amount_rejects_more_than_four_decimals() {
        assert!(Amount::new(dec!(1.00001)).is_none());
    }

    #[test]
    fn amount_addition_works() {
        let a = Amount::new(dec!(1.0)).unwrap();
        let b = Amount::new(dec!(2.5)).unwrap();
        let sum = (a + b).unwrap();
        assert_eq!(sum.as_decimal(), dec!(3.5));
    }

    #[test]
    fn amount_subtraction_works() {
        let a = Amount::new(dec!(5.0)).unwrap();
        let b = Amount::new(dec!(2.5)).unwrap();
        let diff = (a - b).unwrap();
        assert_eq!(diff.as_decimal(), dec!(2.5));
    }

    #[test]
    fn amount_is_negative() {
        let positive = Amount::new(dec!(1.0)).unwrap();
        let negative = Amount::new(dec!(-1.0)).unwrap();
        let zero = Amount::new(dec!(0)).unwrap();

        assert!(!positive.is_negative());
        assert!(negative.is_negative());
        assert!(!zero.is_negative());
    }

    #[test]
    fn account_total() {
        let mut account = Account::new(ClientId(1));
        account.available = Amount::new(dec!(100.0)).unwrap();
        account.held = Amount::new(dec!(50.0)).unwrap();
        assert_eq!(account.total().as_decimal(), dec!(150.0));
    }
}
