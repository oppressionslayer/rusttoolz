//! Rule 30's edge by lookup, checked against measurement: `cargo run --release --example edge`
use rusttoolz::edgelaw::{edge, level, measure, verify, VERIFIED_TO};

fn main() {
    for g in [255u64, 767, 1279, 1791, 2047, 16_777_215, 1_000_000_000_000] {
        println!("gen {g:>16}  v2 {:>2}  edge run {:?}", level(g), edge(g));
    }
    println!("measured at 1791: {}", measure(1791, 512));
    println!("lookup vs measurement to 8191: {} mismatches", verify(8191, 512).len());
    println!("the table covers every generation to {VERIFIED_TO}");
}
