//! **Roll the Mersennes.** Every known Mersenne prime up to `p_max` is a certified fact in the
//! store; each one is rolled up the Pocklington ladder to `bits`, and every rung is written down
//! with its bits and its witness: `cargo run --release --features gmp --example rollballmersennes -- 4423 8192`
//! (`p_max bits`). Rerun it and it reads instead of proving; raise `bits` and it climbs on from
//! each ladder's top.
use rusttoolz::numtoolz::MERSENNE_EXPONENTS;
use rusttoolz::prove::{build_up, known_mersenne_seed, verify_certificate};
use rusttoolz::store::Store;

fn main() {
    let a: Vec<u64> = std::env::args().skip(1).filter_map(|s| s.parse().ok()).collect();
    let p_max = a.first().copied().unwrap_or(4423);
    let bits = a.get(1).copied().unwrap_or(8192);
    let mut store = Store::open(Store::default_path()).unwrap_or_else(|e| panic!("{e}"));
    let seeded = store.seed_known_mersennes().unwrap_or_else(|e| panic!("{e}"));
    println!("store {}: {} records, {} known Mersennes seeded as facts", store.path().display(), store.len(), seeded);
    for &p in MERSENNE_EXPONENTS.iter().filter(|&&p| p <= p_max) {
        let base = format!("M{p}");
        let t = std::time::Instant::now();
        let stored = store.ladder(&base);
        let seed = stored.last().cloned().unwrap_or_else(|| known_mersenne_seed(p as u32).expect("a known Mersenne"));
        let had = stored.len();
        let mut chain = stored;
        let climb = build_up(seed, bits);
        chain.extend(climb.into_iter().skip(if chain.is_empty() { 0 } else { 1 }));
        let new = store.record_ladder(&base, &chain).unwrap_or_else(|e| panic!("{e}"));
        let top = chain.last().unwrap();
        let rungs: Vec<String> = chain.iter().map(|r| format!("{}/{}", r.n.bits(), r.a)).collect();
        println!("{base:>7} -> {} bits in {} rungs, {} new, {:.2?}  [{}]", top.n.bits(), chain.len() - 1, new, t.elapsed(), rungs.join(" "));
        if had == 0 || new > 0 {
            // the seed is a known Mersenne, so verify replays only the witnesses
            let t = std::time::Instant::now();
            println!("         verify: {} in {:.2?}", verify_certificate(&chain, None), t.elapsed());
        }
    }
    let ms: Vec<String> = store.mersennes().iter().map(|(p, ok)| format!("{p}{}", if *ok { "" } else { "x" })).collect();
    println!("mersennes in the store: {}", ms.join(" "));
}
