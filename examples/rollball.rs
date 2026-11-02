//! Roll M127 up the Pocklington ladder: `cargo run --release --example rollball -- 4096`
use rusttoolz::prove::{build_up, mersenne_seed, verify_certificate};

fn main() {
    let bits: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(2048);
    let t = std::time::Instant::now();
    let chain = build_up(mersenne_seed(127).expect("M127"), bits);
    let top = chain.last().unwrap();
    println!("M127 -> {} bits ({} digits) in {} rungs, {:.2?}", top.n.bits(), top.n.to_string().len(), chain.len() - 1, t.elapsed());
    for r in &chain {
        println!("  {:>7} bits  witness {}", r.n.bits(), r.a);
    }
    let t = std::time::Instant::now();
    println!("verify: {} in {:.2?}", verify_certificate(&chain, None), t.elapsed());
}
