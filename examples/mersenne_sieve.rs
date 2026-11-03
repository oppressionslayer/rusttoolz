//! **Fail the Mersennes fast.** Every prime exponent in `[lo, hi)` is tried for a small factor
//! `2kp + 1`, `k ≤ k_max`; a factor is a proven composite and goes into the store as one, and
//! the exponents with none are the survivors, the only ones a Lucas–Lehmer would be spent on.
//! `cargo run --release --example mersenne_sieve -- 136279841 136400000 1000000` (`lo hi k_max`).
//! Prints the heuristic guess of where the next one lies, for a sense of scale, not a prediction.
use rusttoolz::numtoolz::{estimate_next, mersenne_factor};
use rusttoolz::store::{Record, Store};
use rayon::prelude::*;

fn main() {
    let a: Vec<u64> = std::env::args().skip(1).filter_map(|s| s.parse().ok()).collect();
    let lo = a.first().copied().unwrap_or(136_279_842);
    let hi = a.get(1).copied().unwrap_or(lo + 100_000);
    let k_max = a.get(2).copied().unwrap_or(1_000_000);
    let (fitted, next, heuristic) = estimate_next();
    println!("the 52 known exponents grow by {fitted:.4} a step (the heuristic says {heuristic:.4}); the next is expected near p = {next:.0}, with a wide spread");
    let mut store = Store::open(Store::default_path()).unwrap_or_else(|e| panic!("{e}"));
    store.seed_known_mersennes().unwrap_or_else(|e| panic!("{e}"));
    let known: std::collections::HashSet<u64> = store.mersennes().iter().map(|(p, _)| *p as u64).collect();
    let exps: Vec<u64> = (lo..hi).filter(|&p| p % 2 == 1 && rusttoolz::is_prime_u64(p) && !known.contains(&p)).collect();
    println!("{} prime exponents in [{lo}, {hi}) without a verdict; trying k to {k_max}", exps.len());
    let t = std::time::Instant::now();
    let verdicts: Vec<(u64, Option<u64>)> = exps.par_iter().map(|&p| (p, mersenne_factor(p, k_max))).collect();
    let failed = verdicts.iter().filter(|v| v.1.is_some()).count();
    println!("{} failed quickly, {} survive, {:.2?}", failed, verdicts.len() - failed, t.elapsed());
    let mut new = 0;
    for (p, q) in &verdicts {
        if let Some(q) = q {
            let rec = Record::new("mersenne").int("p", *p).int("bits", *p).flag("prime", false).str("proof", "factor").int("factor", *q);
            new += store.record(rec).unwrap_or_else(|e| panic!("{e}")) as usize;
        }
    }
    println!("store: {new} composites recorded with their factor ({} records)", store.len());
    // **small survivors are settled here and now.** Lucas–Lehmer at p < 50,000 is milliseconds to
    // seconds; above that a single test is days, and the list is the answer
    const SETTLE: u64 = 50_000;
    let (small, large): (Vec<u64>, Vec<u64>) = verdicts.iter().filter(|v| v.1.is_none()).map(|v| v.0).partition(|&p| p < SETTLE);
    if !small.is_empty() {
        let t = std::time::Instant::now();
        let tested: Vec<(u64, bool)> = small.par_iter().map(|&p| (p, rusttoolz::prove::lucas_lehmer(p as u32))).collect();
        let mut primes = Vec::new();
        for &(p, prime) in &tested {
            store.record(Record::mersenne(p as u32, prime, 0.0)).unwrap_or_else(|e| panic!("{e}"));
            if prime {
                primes.push(p.to_string());
            }
        }
        println!("{} survivors below {SETTLE} settled by Lucas–Lehmer in {:.2?}: {} prime{}", tested.len(), t.elapsed(), primes.len(), if primes.is_empty() { String::new() } else { format!(" ({})", primes.join(" ")) });
    }
    if !large.is_empty() {
        let s: Vec<String> = large.iter().map(|p| p.to_string()).collect();
        println!("survivors above {SETTLE}, each a full Lucas–Lehmer away from a verdict: {}", if s.len() > 40 { format!("{} … ({} in all)", s[..40].join(" "), s.len()) } else { s.join(" ") });
    }
}
