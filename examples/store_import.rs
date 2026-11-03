//! **Bring a ladder from another language into the store.** Reads a certificate as the Python
//! package or the browser writes it — the `[N, q, a]` / `[N, k, a]` lists, or Python's `repr` of
//! `build_up`'s dicts (`{'N': …, 'q': …, 'a': …}` and `{'N': …, 'proof': 'base'}`) — keeps the
//! rows that chain (each rung's `q` the row before it), verifies the chain against the store,
//! and records every rung: `cargo run --release --example store_import -- chain.txt [base]`.
//! The seed may be a Mersenne prime or any prime the store already holds, so a Python climb
//! seeded on one of our rungs is accepted and its rungs become ours.
use num_bigint::BigUint;
use num_traits::Zero;
use rusttoolz::cert::{from_json, sniff};
use rusttoolz::prove::Rung;
use rusttoolz::store::Store;

/// Python's dict repr: every `{...}` row, `N` required, `q` and `a` when present.
fn dict_rows(text: &str) -> Vec<Rung> {
    let field = |row: &str, key: &str| -> Option<String> {
        let at = row.find(&format!("'{key}'"))?;
        let rest = &row[at + key.len() + 2..];
        let rest = rest.trim_start_matches(|c: char| c == ':' || c.is_whitespace());
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        (!digits.is_empty()).then_some(digits)
    };
    let mut out = Vec::new();
    for row in text.split('{').skip(1) {
        let row = row.split('}').next().unwrap_or("");
        let Some(n) = field(row, "N").and_then(|s| s.parse::<BigUint>().ok()) else { continue };
        let q = field(row, "q").and_then(|s| s.parse::<BigUint>().ok());
        let a = field(row, "a").and_then(|s| s.parse::<u32>().ok());
        match (q, a) {
            (Some(q), Some(a)) if !row.contains("'base'") => out.push(Rung { n, q, a }),
            _ => out.push(Rung { n, q: BigUint::zero(), a: 0 }),
        }
    }
    out
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let Some(path) = a.first() else {
        eprintln!("store_import <certificate file> [base]");
        std::process::exit(2);
    };
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("{path}: {e}\nSave the Python chain first, e.g. in IPython: open('{path}', 'w').write(repr(chain))");
        std::process::exit(2);
    });
    let rows = match sniff(&text) {
        Some(conv) => from_json(&text, conv).unwrap_or_else(|e| panic!("{e}")),
        None => dict_rows(&text),
    };
    let store = Store::open(Store::default_path()).unwrap_or_else(|e| panic!("{e}"));
    // **the chain is the longest run of rows that chain**, ending at the top: walk back from the
    // last row through its q, and the row that no longer chains is the seed
    let mut chain: Vec<Rung> = Vec::new();
    for row in rows.iter().rev() {
        match chain.last() {
            None => chain.push(row.clone()),
            Some(top) if !top.is_seed() && top.q == row.n => chain.push(row.clone()),
            _ => break,
        }
    }
    chain.reverse();
    if let Some(first) = chain.first_mut() {
        // the lowest row is the seed whatever its own bookkeeping said: its proof is Lucas–Lehmer,
        // the known list, or the store holding it
        first.q = BigUint::zero();
        first.a = 0;
    }
    let dropped = rows.len() - chain.len();
    println!("{} rows read, {} chain from a {}-bit seed to {} bits{}", rows.len(), chain.len(), chain.first().map(|r| r.n.bits()).unwrap_or(0), chain.last().map(|r| r.n.bits()).unwrap_or(0), if dropped > 0 { format!(" ({dropped} unchained rows left out)") } else { String::new() });
    let seed = &chain[0].n;
    let how = if rusttoolz::prove::verify_certificate(&chain[..1], None) { "a Mersenne prime" } else if store.holds(seed) { "a prime the store holds" } else { "unknown: not a Mersenne and not in the store" };
    println!("seed: {how}");
    if !store.verify(&chain) {
        println!("the chain does not verify; nothing recorded");
        std::process::exit(1);
    }
    let base = a.get(1).cloned().unwrap_or_else(|| format!("import-{}", seed.bits()));
    let mut store = store;
    let new = store.record_ladder(&base, &chain).unwrap_or_else(|e| panic!("{e}"));
    for r in &chain {
        println!("  {:>7} bits  witness {}", r.n.bits(), r.a);
    }
    println!("verified; {} new rung(s) under base {base} in {} ({} records)", new, store.path().display(), store.len());
}
