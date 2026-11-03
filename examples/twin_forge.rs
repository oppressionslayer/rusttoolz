//! Hunt `k·2^n ± 1` twins: `cargo run --release --example twin_forge -- 256 200000` (`n k_to`, or `n k_from k_to`)
use rusttoolz::store::{Record, Store};
use rusttoolz::twin::{estimate, hunt};

fn main() {
    let a: Vec<u64> = std::env::args().skip(1).filter_map(|s| s.parse().ok()).collect();
    // `n`, or `n k_to`, or `n k_from k_to`
    let n = a.first().copied().unwrap_or(256) as u32;
    let (lo, hi) = match a.len() { 0 | 1 => (1, 200_000), 2 => (1, a[1]), _ => (a[1], a[2]) };
    let (_, per_plus, per_twin) = estimate(n);
    println!("n = {n}: searching k * 2^{n} +/- 1; about one k in {per_plus:.0} gives an upper prime, one in {per_twin:.0} a twin");
    let t = std::time::Instant::now();
    let hits = hunt(n, lo, hi, 100_000);
    println!("k in [{lo}, {hi}): {} proven upper primes, {} proven twin pairs, {:.2?}", hits.iter().filter(|h| h.upper_proven).count(), hits.iter().filter(|h| h.upper_proven && h.lower_proven).count(), t.elapsed());
    let mut store = Store::open(Store::default_path()).unwrap_or_else(|e| panic!("{e}"));
    let mut new = 0;
    for h in hits.iter().filter(|h| h.upper_proven) {
        if let Some(rec) = Record::forge(h) {
            new += store.record(rec).unwrap_or_else(|e| panic!("{e}")) as usize;
        }
        if h.lower_proven {
            println!("  twin k = {} n = {}: {} digits, +1 Pocklington {}, -1 Lucas {}", h.k, h.n, h.digits, h.upper_proven, h.lower_proven);
            println!("    lower = {}", h.lower);
            println!("    upper = {}", h.upper);
        } else {
            println!("  prime k = {} n = {}: {} digits, +1 Pocklington {}", h.k, h.n, h.digits, h.upper_proven);
            println!("    N = {}", h.upper);
        }
    }
    println!("store: {} new in {} ({} records)", new, store.path().display(), store.len());
}
