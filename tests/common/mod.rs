//! Shared test utilities.

/// Process input CSV using the library and return output CSV as string.
#[allow(dead_code)]
pub fn process_via_library(input: &str) -> String {
    let accounts = payments::process_transactions(input.as_bytes());
    let mut output = Vec::new();
    payments::csv_io::write_accounts(&mut output, accounts.iter())
        .expect("failed to write accounts");
    String::from_utf8(output).expect("invalid utf8")
}

/// Process CSV output and return lines sorted by client ID.
pub fn sorted_output(csv: &str) -> String {
    let mut lines: Vec<&str> = csv.lines().collect();
    if lines.is_empty() {
        return String::new();
    }
    let header = lines.remove(0);
    lines.sort_by(|a, b| {
        let a_client: u16 = a.split(',').next().unwrap_or("0").parse().unwrap_or(0);
        let b_client: u16 = b.split(',').next().unwrap_or("0").parse().unwrap_or(0);
        a_client.cmp(&b_client)
    });
    let mut result = vec![header];
    result.extend(lines);
    result.join("\n") + "\n"
}
