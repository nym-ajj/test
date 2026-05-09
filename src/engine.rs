//! Transaction processing engine.

use std::collections::{HashMap, HashSet};

use crate::types::{
    Account, Amount, ClientId, DepositRecord, DisputeState, Transaction, TransactionType, TxId,
};

/// Errors that can occur during transaction processing.
///
/// These are informational; the engine continues processing on error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    /// Transaction ID has already been used.
    DuplicateTxId(TxId),
    /// Account is locked and cannot process transactions.
    AccountLocked(ClientId),
    /// Insufficient available funds for withdrawal.
    InsufficientFunds {
        client_id: ClientId,
        requested: Amount,
        available: Amount,
    },
    /// Referenced transaction does not exist.
    TxNotFound(TxId),
    /// Referenced transaction belongs to a different client.
    ClientMismatch {
        tx_id: TxId,
        expected: ClientId,
        actual: ClientId,
    },
    /// Transaction is not in a disputable state.
    NotDisputable { tx_id: TxId, state: DisputeState },
    /// Transaction is not currently disputed.
    NotDisputed { tx_id: TxId, state: DisputeState },
    /// Missing amount for deposit/withdrawal.
    MissingAmount(TxId),
    /// Arithmetic overflow during balance update.
    Overflow { client_id: ClientId },
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateTxId(tx_id) => write!(f, "duplicate transaction ID: {}", tx_id.0),
            Self::AccountLocked(client_id) => write!(f, "account {} is locked", client_id.0),
            Self::InsufficientFunds {
                client_id,
                requested,
                available,
            } => {
                write!(
                    f,
                    "client {} has insufficient funds: requested {}, available {}",
                    client_id.0, requested, available
                )
            }
            Self::TxNotFound(tx_id) => write!(f, "transaction {} not found", tx_id.0),
            Self::ClientMismatch {
                tx_id,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "transaction {} belongs to client {}, not {}",
                    tx_id.0, actual.0, expected.0
                )
            }
            Self::NotDisputable { tx_id, state } => {
                write!(
                    f,
                    "transaction {} is not disputable (state: {:?})",
                    tx_id.0, state
                )
            }
            Self::NotDisputed { tx_id, state } => {
                write!(
                    f,
                    "transaction {} is not disputed (state: {:?})",
                    tx_id.0, state
                )
            }
            Self::MissingAmount(tx_id) => {
                write!(f, "transaction {} is missing required amount", tx_id.0)
            }
            Self::Overflow { client_id } => {
                write!(f, "arithmetic overflow for client {}", client_id.0)
            }
        }
    }
}

impl std::error::Error for EngineError {}

/// The transaction processing engine.
///
/// Maintains account state and processes transactions according to the
/// payments engine specification.
pub struct Engine {
    /// Client accounts indexed by client ID.
    accounts: HashMap<ClientId, Account>,
    /// Set of all transaction IDs seen (for uniqueness check).
    tx_ids: HashSet<TxId>,
    /// Deposit records for dispute lookup (only deposits are stored).
    deposits: HashMap<TxId, DepositRecord>,
}

impl Engine {
    /// Creates a new engine with no accounts.
    #[must_use]
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            tx_ids: HashSet::new(),
            deposits: HashMap::new(),
        }
    }

    /// Processes a single transaction.
    ///
    /// # Errors
    ///
    /// Returns an error describing why the transaction was skipped.
    /// The engine state is unchanged when an error is returned.
    pub fn process(&mut self, tx: Transaction) -> Result<(), EngineError> {
        // NOTE: Deposit/withdrawal consume a new tx_id (uniqueness enforced),
        // while dispute/resolve/chargeback reference existing tx_ids.
        match tx.tx_type {
            TransactionType::Deposit | TransactionType::Withdrawal => {
                if self.tx_ids.contains(&tx.tx_id) {
                    return Err(EngineError::DuplicateTxId(tx.tx_id));
                }
            }
            TransactionType::Dispute | TransactionType::Resolve | TransactionType::Chargeback => {}
        }

        match tx.tx_type {
            TransactionType::Deposit => self.handle_deposit(tx),
            TransactionType::Withdrawal => self.handle_withdrawal(tx),
            TransactionType::Dispute => self.handle_dispute(tx),
            TransactionType::Resolve => self.handle_resolve(tx),
            TransactionType::Chargeback => self.handle_chargeback(tx),
        }
    }

    /// Handles a deposit transaction.
    fn handle_deposit(&mut self, tx: Transaction) -> Result<(), EngineError> {
        let amount = tx.amount.ok_or(EngineError::MissingAmount(tx.tx_id))?;

        let is_locked = self.accounts.get(&tx.client_id).is_some_and(|a| a.locked);
        if is_locked {
            return Err(EngineError::AccountLocked(tx.client_id));
        }

        // Check overflow before mutating any state
        let current_available = self
            .accounts
            .get(&tx.client_id)
            .map(|a| a.available)
            .unwrap_or_default();
        let new_available = (current_available + amount).ok_or(EngineError::Overflow {
            client_id: tx.client_id,
        })?;

        self.tx_ids.insert(tx.tx_id);

        self.deposits.insert(
            tx.tx_id,
            DepositRecord {
                client_id: tx.client_id,
                amount,
                dispute_state: DisputeState::None,
            },
        );

        let account = self.get_or_create_account(tx.client_id);
        account.available = new_available;

        Ok(())
    }

    /// Handles a withdrawal transaction.
    fn handle_withdrawal(&mut self, tx: Transaction) -> Result<(), EngineError> {
        let amount = tx.amount.ok_or(EngineError::MissingAmount(tx.tx_id))?;

        // NOTE: Pre-check lock and balance before mutable borrow. The account
        // may not exist yet, in which case we treat it as unlocked with zero
        // balance (withdrawal will fail for insufficient funds).
        let (is_locked, current_available) = self
            .accounts
            .get(&tx.client_id)
            .map_or((false, Amount::default()), |a| (a.locked, a.available));

        if is_locked {
            return Err(EngineError::AccountLocked(tx.client_id));
        }

        if current_available.as_decimal() < amount.as_decimal() {
            return Err(EngineError::InsufficientFunds {
                client_id: tx.client_id,
                requested: amount,
                available: current_available,
            });
        }

        // Check overflow before mutating state
        let new_available = (current_available - amount).ok_or(EngineError::Overflow {
            client_id: tx.client_id,
        })?;

        self.tx_ids.insert(tx.tx_id);

        let account = self.get_or_create_account(tx.client_id);
        account.available = new_available;

        // NOTE: Withdrawals are not recorded for dispute lookup. Per spec, only
        // deposits are disputable since they represent funds entering the system.

        Ok(())
    }

    /// Handles a dispute transaction.
    fn handle_dispute(&mut self, tx: Transaction) -> Result<(), EngineError> {
        let deposit = self
            .deposits
            .get(&tx.tx_id)
            .ok_or(EngineError::TxNotFound(tx.tx_id))?;

        if deposit.client_id != tx.client_id {
            return Err(EngineError::ClientMismatch {
                tx_id: tx.tx_id,
                expected: tx.client_id,
                actual: deposit.client_id,
            });
        }

        // NOTE: Can dispute if None (never disputed) or Resolved (re-dispute).
        // Cannot dispute if already Disputed or ChargedBack.
        match deposit.dispute_state {
            DisputeState::None | DisputeState::Resolved => {}
            state => {
                return Err(EngineError::NotDisputable {
                    tx_id: tx.tx_id,
                    state,
                });
            }
        }

        let amount = deposit.amount;

        // Pre-check locked and compute new balances before mutating
        let (is_locked, current_available, current_held) = self
            .accounts
            .get(&tx.client_id)
            .map_or((false, Amount::default(), Amount::default()), |a| {
                (a.locked, a.available, a.held)
            });

        if is_locked {
            return Err(EngineError::AccountLocked(tx.client_id));
        }

        // NOTE: Available MAY go negative here. Per spec, this represents a
        // fraud scenario where funds were withdrawn before the dispute.
        let new_available = (current_available - amount).ok_or(EngineError::Overflow {
            client_id: tx.client_id,
        })?;
        let new_held = (current_held + amount).ok_or(EngineError::Overflow {
            client_id: tx.client_id,
        })?;

        let account = self.get_or_create_account(tx.client_id);
        account.available = new_available;
        account.held = new_held;

        if let Some(deposit) = self.deposits.get_mut(&tx.tx_id) {
            deposit.dispute_state = DisputeState::Disputed;
        }

        Ok(())
    }

    /// Handles a resolve transaction.
    fn handle_resolve(&mut self, tx: Transaction) -> Result<(), EngineError> {
        let deposit = self
            .deposits
            .get(&tx.tx_id)
            .ok_or(EngineError::TxNotFound(tx.tx_id))?;

        if deposit.client_id != tx.client_id {
            return Err(EngineError::ClientMismatch {
                tx_id: tx.tx_id,
                expected: tx.client_id,
                actual: deposit.client_id,
            });
        }

        if deposit.dispute_state != DisputeState::Disputed {
            return Err(EngineError::NotDisputed {
                tx_id: tx.tx_id,
                state: deposit.dispute_state,
            });
        }

        let amount = deposit.amount;

        // Pre-compute new balances before mutating
        let (current_available, current_held) = self
            .accounts
            .get(&tx.client_id)
            .map_or((Amount::default(), Amount::default()), |a| {
                (a.available, a.held)
            });

        let new_available = (current_available + amount).ok_or(EngineError::Overflow {
            client_id: tx.client_id,
        })?;
        let new_held = (current_held - amount).ok_or(EngineError::Overflow {
            client_id: tx.client_id,
        })?;

        let account = self.get_or_create_account(tx.client_id);
        account.available = new_available;
        account.held = new_held;

        if let Some(deposit) = self.deposits.get_mut(&tx.tx_id) {
            deposit.dispute_state = DisputeState::Resolved;
        }

        Ok(())
    }

    /// Handles a chargeback transaction.
    fn handle_chargeback(&mut self, tx: Transaction) -> Result<(), EngineError> {
        let deposit = self
            .deposits
            .get(&tx.tx_id)
            .ok_or(EngineError::TxNotFound(tx.tx_id))?;

        if deposit.client_id != tx.client_id {
            return Err(EngineError::ClientMismatch {
                tx_id: tx.tx_id,
                expected: tx.client_id,
                actual: deposit.client_id,
            });
        }

        if deposit.dispute_state != DisputeState::Disputed {
            return Err(EngineError::NotDisputed {
                tx_id: tx.tx_id,
                state: deposit.dispute_state,
            });
        }

        let amount = deposit.amount;

        // Pre-compute new held before mutating
        let current_held = self
            .accounts
            .get(&tx.client_id)
            .map(|a| a.held)
            .unwrap_or_default();

        // NOTE: Funds are removed from held (they leave the system entirely).
        // The account is then locked to prevent further transactions.
        let new_held = (current_held - amount).ok_or(EngineError::Overflow {
            client_id: tx.client_id,
        })?;

        let account = self.get_or_create_account(tx.client_id);
        account.held = new_held;
        account.locked = true;

        if let Some(deposit) = self.deposits.get_mut(&tx.tx_id) {
            deposit.dispute_state = DisputeState::ChargedBack;
        }

        Ok(())
    }

    /// Gets or creates an account for the given client.
    fn get_or_create_account(&mut self, client_id: ClientId) -> &mut Account {
        self.accounts
            .entry(client_id)
            .or_insert_with(|| Account::new(client_id))
    }

    /// Returns an iterator over all accounts.
    pub fn accounts(&self) -> impl Iterator<Item = &Account> {
        self.accounts.values()
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    fn make_deposit(client: u16, tx: u32, amount: &str) -> Transaction {
        Transaction {
            tx_type: TransactionType::Deposit,
            client_id: ClientId(client),
            tx_id: TxId(tx),
            amount: Some(Amount::new(amount.parse().unwrap()).unwrap()),
        }
    }

    fn make_withdrawal(client: u16, tx: u32, amount: &str) -> Transaction {
        Transaction {
            tx_type: TransactionType::Withdrawal,
            client_id: ClientId(client),
            tx_id: TxId(tx),
            amount: Some(Amount::new(amount.parse().unwrap()).unwrap()),
        }
    }

    fn make_dispute(client: u16, tx: u32) -> Transaction {
        Transaction {
            tx_type: TransactionType::Dispute,
            client_id: ClientId(client),
            tx_id: TxId(tx),
            amount: None,
        }
    }

    fn make_resolve(client: u16, tx: u32) -> Transaction {
        Transaction {
            tx_type: TransactionType::Resolve,
            client_id: ClientId(client),
            tx_id: TxId(tx),
            amount: None,
        }
    }

    fn make_chargeback(client: u16, tx: u32) -> Transaction {
        Transaction {
            tx_type: TransactionType::Chargeback,
            client_id: ClientId(client),
            tx_id: TxId(tx),
            amount: None,
        }
    }

    #[test]
    fn deposit_increases_available() {
        let mut engine = Engine::new();
        engine.process(make_deposit(1, 1, "100.0")).unwrap();

        let account = engine.accounts.get(&ClientId(1)).unwrap();
        assert_eq!(account.available.as_decimal(), dec!(100.0));
        assert_eq!(account.held.as_decimal(), dec!(0));
    }

    #[test]
    fn withdrawal_decreases_available() {
        let mut engine = Engine::new();
        engine.process(make_deposit(1, 1, "100.0")).unwrap();
        engine.process(make_withdrawal(1, 2, "25.0")).unwrap();

        let account = engine.accounts.get(&ClientId(1)).unwrap();
        assert_eq!(account.available.as_decimal(), dec!(75.0));
    }

    #[test]
    fn withdrawal_insufficient_funds_fails() {
        let mut engine = Engine::new();
        engine.process(make_deposit(1, 1, "50.0")).unwrap();

        let result = engine.process(make_withdrawal(1, 2, "100.0"));
        assert!(matches!(result, Err(EngineError::InsufficientFunds { .. })));

        // Balance unchanged
        let account = engine.accounts.get(&ClientId(1)).unwrap();
        assert_eq!(account.available.as_decimal(), dec!(50.0));
    }

    #[test]
    fn duplicate_tx_id_rejected() {
        let mut engine = Engine::new();
        engine.process(make_deposit(1, 1, "100.0")).unwrap();

        let result = engine.process(make_deposit(1, 1, "50.0"));
        assert!(matches!(result, Err(EngineError::DuplicateTxId(_))));
    }

    #[test]
    fn dispute_moves_funds_to_held() {
        let mut engine = Engine::new();
        engine.process(make_deposit(1, 1, "100.0")).unwrap();
        engine.process(make_dispute(1, 1)).unwrap();

        let account = engine.accounts.get(&ClientId(1)).unwrap();
        assert_eq!(account.available.as_decimal(), dec!(0));
        assert_eq!(account.held.as_decimal(), dec!(100.0));
        assert_eq!(account.total().as_decimal(), dec!(100.0));
    }

    #[test]
    fn resolve_releases_held_funds() {
        let mut engine = Engine::new();
        engine.process(make_deposit(1, 1, "100.0")).unwrap();
        engine.process(make_dispute(1, 1)).unwrap();
        engine.process(make_resolve(1, 1)).unwrap();

        let account = engine.accounts.get(&ClientId(1)).unwrap();
        assert_eq!(account.available.as_decimal(), dec!(100.0));
        assert_eq!(account.held.as_decimal(), dec!(0));
    }

    #[test]
    fn chargeback_removes_funds_and_locks() {
        let mut engine = Engine::new();
        engine.process(make_deposit(1, 1, "100.0")).unwrap();
        engine.process(make_dispute(1, 1)).unwrap();
        engine.process(make_chargeback(1, 1)).unwrap();

        let account = engine.accounts.get(&ClientId(1)).unwrap();
        assert_eq!(account.available.as_decimal(), dec!(0));
        assert_eq!(account.held.as_decimal(), dec!(0));
        assert_eq!(account.total().as_decimal(), dec!(0));
        assert!(account.locked);
    }

    #[test]
    fn locked_account_rejects_transactions() {
        let mut engine = Engine::new();
        engine.process(make_deposit(1, 1, "100.0")).unwrap();
        engine.process(make_dispute(1, 1)).unwrap();
        engine.process(make_chargeback(1, 1)).unwrap();

        let result = engine.process(make_deposit(1, 2, "50.0"));
        assert!(matches!(result, Err(EngineError::AccountLocked(_))));
    }
}
