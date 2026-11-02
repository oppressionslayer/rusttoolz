//! **Rule 30's light-cone edge, by lookup.** The run of live cells at the leading edge depends
//! only on `v2(g + 1)`; the odd part of `g + 1` is irrelevant, so generations 255, 767, 1279 and
//! 1791 all give 24. Rungs 0..30 measured by evolving the edge alone, `E = E ^ ((E << 1) | (E << 2))`;
//! rungs 31..48 from OEIS A094604, which agrees on every overlapping rung. Verified single-valued
//! for every generation to 16,777,215; the table covers every generation to `2^48 − 1`.

/// `TABLE[v]` is the edge run for `v2(g + 1) = v`.
pub const TABLE: [u32; 49] = [
    1, 3, 4, 6, 7, 9, 15, 16, 24, 25, 27, 29, 34, 36, 37, 39, 41, 43, 48, 49, 51, 54, 55, 58, 60, 63,
    64, 66, 69, 70, 72, 74, 77, 79, 80, 82, 84, 86, 90, 91, 93, 100, 103, 104, 106, 108, 111, 112, 114,
];
pub const VMAX: u32 = 48;
/// The last generation the table covers.
pub const VERIFIED_TO: u64 = (1 << VMAX) - 1;

/// `v2(g + 1)`: which rung of the ladder generation `g` sits on.
pub fn level(g: u64) -> u32 {
    (g + 1).trailing_zeros()
}

/// The length of the edge run at generation `g`, or `None` past the table.
pub fn edge(g: u64) -> Option<u32> {
    TABLE.get(level(g) as usize).copied()
}

/// The edge read as a number: `2^run − 1`, a Mersenne number.
pub fn value(g: u64) -> Option<u128> {
    edge(g).map(|r| (1u128 << r) - 1)
}

/// The first generation on rung `v`; the others are `(2m + 1)·2^v − 1`.
pub fn first(v: u32) -> u64 {
    (1u64 << v) - 1
}

/// `count` generations sharing `g`'s rung, all with the identical edge run.
pub fn same_as(g: u64, count: u64) -> Vec<u64> {
    let v = level(g);
    (0..count).map(|m| (2 * m + 1) * (1u64 << v) - 1).collect()
}

pub fn covers(g: u64) -> bool {
    level(g) <= VMAX
}

/// **Measure the edge** by evolving it alone on a `w`-bit window: the check on the lookup.
pub fn measure(g: u64, w: u32) -> u32 {
    use num_bigint::BigUint;
    use num_traits::One;
    let mask = (BigUint::one() << w) - BigUint::one();
    let mut e = BigUint::one();
    for _ in 0..g {
        e = (&e ^ ((&e << 1u32) | (&e << 2u32))) & &mask;
    }
    e.trailing_ones() as u32
}

/// Check the lookup against measurement for every generation to `upto`; the mismatches.
pub fn verify(upto: u64, w: u32) -> Vec<u64> {
    use num_bigint::BigUint;
    use num_traits::One;
    let mask = (BigUint::one() << w) - BigUint::one();
    let mut e = BigUint::one();
    let mut bad = Vec::new();
    for g in 1..=upto {
        e = (&e ^ ((&e << 1u32) | (&e << 2u32))) & &mask;
        if let Some(want) = edge(g) {
            if e.trailing_ones() as u32 != want {
                bad.push(g);
            }
        }
    }
    bad
}
