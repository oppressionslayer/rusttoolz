//! One proven prime, fast: `cargo run --release --features gmp --example proth -- 12919`
//! finds the first `k` with `k·2^n + 1` prime and proves it with Pocklington witnesses, since
//! `N − 1 = k·2^n` is already factored. Optional second and third arguments bound `k`.
use rusttoolz::twin::proth_first;
use std::time::Instant;

fn main() {
    let a: Vec<u64> = std::env::args().skip(1).filter_map(|s| s.parse().ok()).collect();
    // `n`, or `n k_to`, or `n k_from k_to`
    let n = a.first().copied().unwrap_or(4096) as u32;
    let (lo, hi) = match a.len() { 0 | 1 => (1, 1_000_000), 2 => (1, a[1]), _ => (a[1], a[2]) };
    let t = Instant::now();
    match proth_first(n, lo, hi, 100_000) {
        Some(p) => {
            let s = p.value.to_string();
            println!("k = {}  n = {}  {} digits  proven in {:.2?}  ({} engine)", p.k, p.n, p.digits, t.elapsed(), rusttoolz::modpow::ENGINE);
            println!("N = {}...{}", &s[..30.min(s.len())], &s[s.len().saturating_sub(30)..]);
            println!("certificate: {{\"k\":{},\"n\":{},\"factors\":{:?}}}", p.k, p.n, p.factors);
        }
        None => println!("no k in [{lo}, {hi}) gives a prime at n = {n}"),
    }
}
