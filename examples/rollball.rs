//! Roll M127 up the Pocklington ladder: `cargo run --release --example rollball -- 4096`
//!
//! Every rung goes into the prime store (`primes.jsonl`, or `RUSTTOOLZ_STORE`), and a climb that
//! is already there starts from its top rung instead of from M127.
use rusttoolz::prove::{build_up, mersenne_seed, verify_certificate};
use rusttoolz::store::Store;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() > 1 {
        eprintln!("rollball takes one number, the target bits: `rollball -- 12000`. For another Mersenne use `rollballmersennes -- <p_max> <bits>`; for k*2^n+-1 use `twin_forge -- <n> <k_from> <k_to>`.");
        std::process::exit(2);
    }
    let bits: u64 = args.first().and_then(|s| s.parse().ok()).unwrap_or(2048);
    let mut store = Store::open(Store::default_path()).unwrap_or_else(|e| panic!("{e}"));
    let t = std::time::Instant::now();
    let stored = store.ladder("M127");
    // **a size the store already reaches is a lookup**: the ladder up to the first rung of at
    // least `bits`, nothing climbed, nothing printed twice
    if let Some(at) = stored.iter().position(|r| r.n.bits() >= bits) {
        let top = stored.last().unwrap();
        println!("the store holds M127 to {} bits; {} bits is its rung {} (ask for more than {} to climb on)", top.n.bits(), stored[at].n.bits(), at, top.n.bits());
    }
    let stored: Vec<_> = match stored.iter().position(|r| r.n.bits() >= bits) {
        Some(at) => stored[..=at].to_vec(),
        None => stored,
    };
    let seed = stored.last().cloned().unwrap_or_else(|| mersenne_seed(127).expect("M127"));
    let from = seed.n.bits();
    let mut chain = stored.clone();
    let climb = build_up(seed, bits);
    chain.extend(climb.into_iter().skip(if chain.is_empty() { 0 } else { 1 }));
    let top = chain.last().unwrap();
    println!("M127 -> {} bits ({} digits) in {} rungs, {:.2?}{}", top.n.bits(), top.n.to_string().len(), chain.len() - 1, t.elapsed(), if stored.is_empty() { String::new() } else { format!(" (resumed from {from} bits in the store)") });
    for r in &chain {
        println!("  {:>7} bits  witness {}", r.n.bits(), r.a);
    }
    let t = std::time::Instant::now();
    println!("verify: {} in {:.2?}", verify_certificate(&chain, None), t.elapsed());
    let new = store.record_ladder("M127", &chain).unwrap_or_else(|e| panic!("{e}"));
    println!("store: {} new rung(s) in {} ({} records)", new, store.path().display(), store.len());
    // a recipe names one climb to one target; a ladder resumed through several targets has none
    if let Some(offsets) = rusttoolz::prove::recipe(&chain, bits).filter(|o| !o.is_empty()) {
        let witnesses: Vec<u32> = chain.iter().skip(1).map(|r| r.a).collect();
        store.record(rusttoolz::store::Record::recipe("M127", bits, &offsets, &witnesses)).unwrap_or_else(|e| panic!("{e}"));
        println!("recipe: M127 -> {bits} bits, offsets {offsets:?}, witnesses {witnesses:?}");
    }
    println!("\nN = {}", top.n);
}
