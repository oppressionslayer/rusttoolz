//! The free-logic verdict stack on a range: `cargo run --release --example verdict -- 2 10000`
use rusttoolz::lanes::{dashboard, verdict, LANES};

fn main() {
    let a: Vec<u64> = std::env::args().skip(1).filter_map(|s| s.parse().ok()).collect();
    let (lo, hi) = (a.first().copied().unwrap_or(2), a.get(1).copied().unwrap_or(10_000));
    for x in [91u64, 323, 1717, 2533, 4999, 11441, 11443] {
        let v = verdict(x, &LANES, 8);
        println!("{x:>6}  {}  [{:?}]  {}", if v.prime { "prime    " } else { "composite" }, v.cost, v.how);
    }
    let d = dashboard(lo, hi, &LANES);
    println!("\n[{lo}, {hi}): wheel {} · below horizon {} · square/fermat/fold {} · rho {} · test {}   {:.1}% deterministic", d.wheel, d.below_horizon, d.walk, d.rho, d.test, d.deterministic_pct());
}
