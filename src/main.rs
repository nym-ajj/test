//! Payments engine CLI - thin wrapper around the payments library.

use std::{env, fs::File, io, process::ExitCode};

use anyhow::{Context, Result};
use tracing::{error, warn};

fn run() -> Result<ExitCode> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .with_writer(io::stderr)
        .init();

    let args: Vec<String> = env::args().collect();

    let file_path = if let Some(path) = args.get(1) {
        path
    } else {
        error!("missing file argument");
        payments::csv_io::write_accounts(io::stdout(), std::iter::empty())?;
        return Ok(ExitCode::SUCCESS);
    };

    if args.len() > 2 {
        warn!("ignoring extra arguments: {:?}", &args[2..]);
    }

    let file =
        File::open(file_path).with_context(|| format!("failed to open file: {file_path}"))?;

    let accounts = payments::process_transactions(file);
    payments::csv_io::write_accounts(io::stdout(), accounts.iter())?;

    Ok(ExitCode::SUCCESS)
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            error!("fatal error: {}", e);
            ExitCode::FAILURE
        }
    }
}
