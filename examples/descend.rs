//! **The certificate from the top prime alone**: `cargo run --release --features gmp --example descend -- <N or a file holding it>`.
//! Takes square roots down, reads each rung's `q` off the number above it, proves every step by Pocklington,
//! and stops at a Mersenne. Prints the ladder and whether it verifies.
use num_bigint::BigUint;
use rusttoolz::prove::{descend, verify_certificate};

fn main() {
    let arg = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("descend <N or file>");
        std::process::exit(2);
    });
    let text = std::fs::read_to_string(&arg).unwrap_or(arg);
    let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
    let top: BigUint = digits.parse().unwrap_or_else(|_| {
        eprintln!("give a decimal number or a file holding one");
        std::process::exit(2);
    });
    let t = std::time::Instant::now();
    let chain = descend(&top, 64, 8_192);
    println!("{} bits down to {} bits in {} rungs, {:.2?}", top.bits(), chain[0].n.bits(), chain.len() - 1, t.elapsed());
    for r in &chain {
        println!("  {:>7} bits  witness {}", r.n.bits(), r.a);
    }
    let t = std::time::Instant::now();
    let ok = verify_certificate(&chain, None);
    println!("verify: {ok} in {:.2?}{}", t.elapsed(), if ok { "" } else { "  (the bottom is not a Mersenne: a rung sized to land on a bit count is not the square of the one below; the store holds it)" });
}
