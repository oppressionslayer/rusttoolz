//! Hunt `k·2^n ± 1` twins: `cargo run --release --example twin_forge -- 256 1 200000`
use rusttoolz::twin::{estimate, hunt};

fn main() {
    let a: Vec<u64> = std::env::args().skip(1).filter_map(|s| s.parse().ok()).collect();
    let (n, lo, hi) = (a.first().copied().unwrap_or(256) as u32, a.get(1).copied().unwrap_or(1), a.get(2).copied().unwrap_or(200_000));
    let (digits, per_plus, per_twin) = estimate(n);
    println!("n = {n}: {digits}-digit numbers; about one k in {per_plus:.0} gives a +1 prime, one in {per_twin:.0} a twin");
    let t = std::time::Instant::now();
    let hits = hunt(n, lo, hi, 100_000);
    println!("k in [{lo}, {hi}): {} +1 primes proven, {} twins, {:.2?}", hits.len(), hits.iter().filter(|h| h.lower_proven).count(), t.elapsed());
    for h in hits.iter().filter(|h| h.lower_proven) {
        println!("  twin k = {} n = {}: {} digits, +1 Pocklington {}, -1 Lucas {}", h.k, h.n, h.digits, h.upper_proven, h.lower_proven);
    }
}
