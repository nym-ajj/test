//! Payments engine library.
//!
//! Processes financial transactions (deposits, withdrawals, disputes) and
//! maintains client account states.

pub mod csv_io;
pub mod engine;
pub mod types;

use std::io::{BufReader, Read};

pub use engine::Engine;
use tracing::{info, warn};
pub use types::{Account, Amount, ClientId, Transaction, TransactionType, TxId};

/// Processes transactions from a reader and returns sorted account states.
///
/// Malformed rows and transaction errors are logged via `tracing::warn` but
/// do not stop processing.
pub fn process_transactions<R: Read>(reader: R) -> Vec<Account> {
    let mut engine = Engine::new();
    let mut total = 0u64;
    let mut skipped = 0u64;

    for result in csv_io::parse_transactions(BufReader::new(reader)) {
        total += 1;
        match result {
            Ok(tx) => {
                if let Err(e) = engine.process(tx) {
                    skipped += 1;
                    warn!("skipping transaction: {}", e);
                }
            }
            Err(e) => {
                skipped += 1;
                warn!("skipping malformed row: {}", e);
            }
        }
    }

    let mut accounts: Vec<_> = engine.accounts().cloned().collect();
    accounts.sort_by_key(|a| a.client_id.0);

    info!(
        "processed {} transactions, {} skipped, {} accounts",
        total,
        skipped,
        accounts.len()
    );

    accounts
}
