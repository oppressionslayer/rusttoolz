//! Roll a chain and print it as a certificate: `cargo run --release --example cert_json -- 512 slip`
use rusttoolz::cert::{to_json, Convention};
use rusttoolz::prove::{build_up, mersenne_seed};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let bits: u64 = a.first().and_then(|s| s.parse().ok()).unwrap_or(512);
    let conv = if a.get(1).map(|s| s == "slip").unwrap_or(false) { Convention::Slip } else { Convention::Numtools };
    let chain = build_up(mersenne_seed(31).unwrap(), bits);
    println!("{}", to_json(&chain, conv));
}
