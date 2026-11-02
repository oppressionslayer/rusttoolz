//! **The gap forge.** Build the primes to `n` by elimination and read their gaps, the even
//! distances, straight off: twins, sexy pairs, gap counts, the jumping champion, the gap records,
//! and how well a number `M` sieves the distances (a gap is caught when it divides `M`).

use num_bigint::BigUint;
use num_traits::Zero;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct GapForge {
    pub n: u64,
    pub primes: Vec<u64>,
}

impl GapForge {
    pub fn new(n: u64) -> GapForge {
        GapForge { n, primes: crate::sieve(n) }
    }

    /// `gaps[i] = primes[i+1] − primes[i]`; `gaps[0] == 1`, every later gap is even.
    pub fn gaps(&self) -> Vec<u64> {
        self.primes.windows(2).map(|w| w[1] - w[0]).collect()
    }

    /// The strictly even distances: every gap after 2 → 3.
    pub fn evens(&self) -> Vec<u64> {
        self.gaps().into_iter().skip(1).collect()
    }

    /// The indices `i` where `primes[i] → primes[i+1]` is `g`.
    pub fn where_gap(&self, g: u64) -> Vec<usize> {
        self.primes.windows(2).enumerate().filter(|(_, w)| w[1] - w[0] == g).map(|(i, _)| i).collect()
    }

    /// The lower member of every twin pair.
    pub fn twins(&self) -> Vec<u64> {
        self.primes.windows(2).filter(|w| w[1] - w[0] == 2).map(|w| w[0]).collect()
    }

    /// The lower member of every sexy pair `(p, p + 6)`, both prime (not necessarily consecutive).
    pub fn sexy(&self) -> Vec<u64> {
        let set: std::collections::HashSet<u64> = self.primes.iter().copied().collect();
        self.primes.iter().copied().filter(|p| set.contains(&(p + 6))).collect()
    }

    /// `{gap: how many times}` over every distance to `n`.
    pub fn gap_counts(&self) -> BTreeMap<u64, u64> {
        let mut m = BTreeMap::new();
        for g in self.gaps() {
            *m.entry(g).or_insert(0) += 1;
        }
        m
    }

    /// The most common distance to `n`.
    pub fn jumping_champion(&self) -> u64 {
        self.gap_counts().into_iter().max_by_key(|&(g, c)| (c, std::cmp::Reverse(g))).map(|(g, _)| g).unwrap_or(0)
    }

    /// The gap records: `(prime, next, gap)` wherever a gap exceeds every gap before it.
    pub fn records(&self) -> Vec<(u64, u64, u64)> {
        let mut out = Vec::new();
        let mut best = 0;
        for w in self.primes.windows(2) {
            let g = w[1] - w[0];
            if g > best {
                best = g;
                out.push((w[0], w[1], g));
            }
        }
        out
    }

    /// **How well `M` sieves the distances**: the share of gaps that divide `M`, and the gaps it
    /// misses most, as `(gap, count)`.
    pub fn coverage(&self, m: &BigUint, top: usize) -> Coverage {
        let counts = self.gap_counts();
        let total: u64 = counts.values().sum();
        let mut caught = 0u64;
        let mut missed: Vec<(u64, u64)> = Vec::new();
        for (&g, &c) in &counts {
            if (m % BigUint::from(g)).is_zero() {
                caught += c;
            } else {
                missed.push((g, c));
            }
        }
        missed.sort_by_key(|&(g, c)| (std::cmp::Reverse(c), g));
        missed.truncate(top);
        Coverage { total, caught, catch_rate: caught as f64 / total.max(1) as f64, top_survivors: missed }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Coverage {
    pub total: u64,
    pub caught: u64,
    pub catch_rate: f64,
    pub top_survivors: Vec<(u64, u64)>,
}

/// A wheel of these primes builds every prime and gap exactly to `next_prime(max)²`.
pub fn wheel_horizon(wheel: &[u64]) -> u64 {
    crate::lanes::horizon(wheel)
}
