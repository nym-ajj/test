//! CSV input/output for transactions and accounts.

use std::io::{Read, Write};

use anyhow::{Context, Result, anyhow};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::types::{Account, Amount, ClientId, Transaction, TransactionType, TxId};

/// Raw CSV schema for transaction input.
#[derive(Debug, Deserialize)]
struct CsvTransaction {
    #[serde(rename = "type")]
    tx_type: String,
    client: u16,
    tx: u32,
    amount: Option<Decimal>,
}

/// CSV schema for account output.
#[derive(Debug, Serialize)]
struct CsvAccount {
    client: u16,
    available: String,
    held: String,
    total: String,
    locked: bool,
}

impl From<&Account> for CsvAccount {
    fn from(account: &Account) -> Self {
        Self {
            client: account.client_id.0,
            available: format!("{}", account.available),
            held: format!("{}", account.held),
            total: format!("{}", account.total()),
            locked: account.locked,
        }
    }
}

/// Parses a transaction type string into a `TransactionType`.
fn parse_tx_type(s: &str) -> Result<TransactionType> {
    match s.trim().to_lowercase().as_str() {
        "deposit" => Ok(TransactionType::Deposit),
        "withdrawal" => Ok(TransactionType::Withdrawal),
        "dispute" => Ok(TransactionType::Dispute),
        "resolve" => Ok(TransactionType::Resolve),
        "chargeback" => Ok(TransactionType::Chargeback),
        other => Err(anyhow!("unknown transaction type: {}", other)),
    }
}

/// Validates and converts a raw CSV transaction to a domain Transaction.
fn validate_transaction(csv_tx: CsvTransaction) -> Result<Transaction> {
    let tx_type = parse_tx_type(&csv_tx.tx_type)?;
    let client_id = ClientId(csv_tx.client);
    let tx_id = TxId(csv_tx.tx);

    let amount = match tx_type {
        TransactionType::Deposit | TransactionType::Withdrawal => {
            let decimal = csv_tx
                .amount
                .ok_or_else(|| anyhow!("deposit/withdrawal requires amount"))?;

            let amount = Amount::new(decimal)
                .ok_or_else(|| anyhow!("amount has more than 4 decimal places"))?;

            if amount.is_negative() {
                return Err(anyhow!("negative amount"));
            }

            Some(amount)
        }
        // NOTE: Dispute/resolve/chargeback reference existing tx_ids, so the
        // amount field is semantically meaningless and ignored.
        TransactionType::Dispute | TransactionType::Resolve | TransactionType::Chargeback => None,
    };

    Ok(Transaction {
        tx_type,
        client_id,
        tx_id,
        amount,
    })
}

/// Parses transactions from a CSV reader.
///
/// Returns an iterator over parsed transactions. Each item is a `Result` to
/// allow the caller to handle parse errors gracefully.
pub fn parse_transactions(
    reader: impl Read,
) -> impl Iterator<Item = Result<Transaction, anyhow::Error>> {
    let csv_reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_reader(reader);

    csv_reader
        .into_deserialize::<CsvTransaction>()
        .map(|result| {
            result
                .context("failed to parse CSV row")
                .and_then(validate_transaction)
        })
}

/// Writes accounts to CSV format.
///
/// Always writes the header row, even if there are no accounts.
///
/// # Errors
///
/// Returns an error if writing to the output fails.
pub fn write_accounts<'a>(
    writer: impl Write,
    accounts: impl Iterator<Item = &'a Account>,
) -> Result<()> {
    let mut csv_writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(writer);

    csv_writer
        .write_record(["client", "available", "held", "total", "locked"])
        .context("failed to write header")?;

    for account in accounts {
        let csv_account = CsvAccount::from(account);
        csv_writer
            .serialize(&csv_account)
            .context("failed to write account")?;
    }

    csv_writer.flush().context("failed to flush CSV writer")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    #[test]
    fn parse_deposit_transaction() {
        let csv = "type,client,tx,amount\ndeposit,1,1,100.0";
        let mut txs: Vec<_> = parse_transactions(csv.as_bytes())
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(txs.len(), 1);
        let tx = txs.pop().unwrap();
        assert_eq!(tx.tx_type, TransactionType::Deposit);
        assert_eq!(tx.client_id, ClientId(1));
        assert_eq!(tx.tx_id, TxId(1));
        assert_eq!(tx.amount.unwrap().as_decimal(), dec!(100.0));
    }

    #[test]
    fn parse_dispute_ignores_amount() {
        let csv = "type,client,tx,amount\ndispute,1,1,999.0";
        let txs: Vec<_> = parse_transactions(csv.as_bytes())
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(txs.len(), 1);
        assert!(txs[0].amount.is_none());
    }

    #[test]
    fn rejects_negative_amount() {
        let csv = "type,client,tx,amount\ndeposit,1,1,-100.0";
        let result: Result<Vec<_>, _> = parse_transactions(csv.as_bytes()).collect();
        assert!(result.is_err());
    }

    #[test]
    fn rejects_too_many_decimals() {
        let csv = "type,client,tx,amount\ndeposit,1,1,1.00001";
        let result: Result<Vec<_>, _> = parse_transactions(csv.as_bytes()).collect();
        assert!(result.is_err());
    }

    #[test]
    fn write_accounts_formats_correctly() {
        let account = Account {
            client_id: ClientId(1),
            available: Amount::new(dec!(75.0)).unwrap(),
            held: Amount::new(dec!(0.0)).unwrap(),
            locked: false,
        };

        let mut output = Vec::new();
        write_accounts(&mut output, std::iter::once(&account)).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("client,available,held,total,locked"));
        assert!(output_str.contains("1,75.0000,0.0000,75.0000,false"));
    }
}
