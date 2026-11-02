//! **Rails, lanes, gates and folds.** Free logic settles most numbers with no primality test:
//! a wheel of small-prime lanes, the horizon below which surviving the wheel *is* being prime,
//! the square lane, the Fermat near-square walk, the fold-wheel that folds a far factor pair
//! close with a multiplier, and the rho walk as the one heuristic. `verdict` stacks them and
//! spends a real test only on the thin band of primes past the fold horizon.

/// The default wheel.
pub const LANES: [u64; 6] = [2, 3, 5, 7, 11, 13];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cost {
    /// A mod-class, square, near-square or fold certainty.
    Free,
    /// The rho walk found a factor.
    Cheap,
    /// Survived every walk; one real test confirmed a prime.
    Test,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    pub prime: bool,
    pub how: String,
    pub cost: Cost,
}

fn isqrt(n: u64) -> u64 {
    let mut r = (n as f64).sqrt() as u64;
    while r * r > n {
        r -= 1;
    }
    while (r + 1) * (r + 1) <= n {
        r += 1;
    }
    r
}

/// A perfect square is composite on sight.
pub fn is_square(n: u64) -> bool {
    let r = isqrt(n);
    r * r == n
}

/// **The every-time lanes** for `x`: residues and fold directions, pure functions of the low bits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaneRead {
    pub x: u64,
    pub mod8: u64,
    pub mod6: u64,
    pub mod4: u64,
    pub mod5: u64,
    /// `6k-1`, `6k+1` or `off`.
    pub lane: &'static str,
    pub fold: u64,
    pub fold_dn: bool,
    pub v1: u64,
    pub v1_eq: bool,
    pub v1_dn: bool,
}

pub fn lane_read(x: u64) -> LaneRead {
    let lane = match x % 6 {
        5 => "6k-1",
        1 => "6k+1",
        _ => "off",
    };
    LaneRead {
        x,
        mod8: x % 8,
        mod6: x % 6,
        mod4: x % 4,
        mod5: x % 5,
        lane,
        fold: (x + 1) ^ 5,
        fold_dn: ((x + 1) ^ 5) < x,
        v1: (x + 3) ^ 5,
        v1_eq: ((x + 3) ^ 5) == x,
        v1_dn: ((x + 3) ^ 5) < x,
    }
}

/// The horizon of a wheel: the square of the next prime past its top lane.
pub fn horizon(lanes: &[u64]) -> u64 {
    let mut m = lanes.iter().max().copied().unwrap_or(1) + 1;
    while !crate::is_prime_u64(m) {
        m += 1;
    }
    m * m
}

/// **The lane verdict**: a lane divides it (composite, free); survives below the horizon
/// (prime, free); survives past it (`None`, a slip-through semiprime could reach here).
pub fn lane_verdict(x: u64, lanes: &[u64]) -> Option<Verdict> {
    if x < 2 {
        return Some(Verdict { prime: false, how: "less than 2".into(), cost: Cost::Free });
    }
    if lanes.contains(&x) {
        return Some(Verdict { prime: true, how: "is a lane itself".into(), cost: Cost::Free });
    }
    for &w in lanes {
        if x % w == 0 {
            return Some(Verdict { prime: false, how: format!("{w} divides it"), cost: Cost::Free });
        }
    }
    let h = horizon(lanes);
    if x < h {
        return Some(Verdict { prime: true, how: format!("survived every lane below the horizon {h}"), cost: Cost::Free });
    }
    None
}

/// **The near-square walk**: `n = a² − b²`, `a` up from `⌈√n⌉`, `steps` tries. Close pairs land fast.
pub fn fermat_walk(n: u64, steps: u32) -> Option<(u64, u64)> {
    if n % 2 == 0 {
        return Some((2, n / 2));
    }
    let mut a = isqrt(n);
    if a * a < n {
        a += 1;
    }
    for _ in 0..steps {
        let b2 = a * a - n;
        if is_square(b2) {
            let b = isqrt(b2);
            return if a - b > 1 { Some((a - b, a + b)) } else { None };
        }
        a += 1;
    }
    None
}

/// **Pollard's rho**: the far pairs, a cheap walk with no bounded guarantee.
pub fn rho_walk(n: u64, steps: u32) -> Option<(u64, u64)> {
    fn gcd(mut a: u64, mut b: u64) -> u64 {
        while b != 0 {
            let t = a % b;
            a = b;
            b = t;
        }
        a
    }
    if n < 4 {
        return None;
    }
    if n % 2 == 0 {
        return Some((2, n / 2));
    }
    for c in [1u64, 2, 3, 5, 7, 11] {
        let (mut x, mut y, mut d, mut i) = (2u64, 2u64, 1u64, 0u32);
        let f = |v: u64| (crate::mulmod(v, v, n) + c) % n;
        while d == 1 && i < steps {
            x = f(x);
            y = f(f(y));
            d = gcd(x.abs_diff(y), n);
            i += 1;
        }
        if 1 < d && d < n {
            return Some((d, n / d));
        }
    }
    None
}

const KS_SHARP: [u64; 22] = [1, 4, 8, 16, 32, 64, 128, 256, 512, 3, 5, 7, 9, 11, 12, 13, 15, 17, 19, 20, 21, 24];

/// **The fold-wheel, sharpened by the mod-4 fork**: multiply `n` by `k` so a far pair folds
/// close, then the near-square walk lands it. `k ≡ 2 (mod 4)` can never land and is skipped;
/// for odd `k·n` only one parity of `a` can work, so `a` steps by two. `(p, q, k)` when it fires.
pub fn fold_sharp(n: u64, steps: u32) -> Option<(u64, u64, u64)> {
    fn gcd(mut a: u64, mut b: u64) -> u64 {
        while b != 0 {
            let t = a % b;
            a = b;
            b = t;
        }
        a
    }
    if n % 2 == 0 {
        return Some((2, n / 2, 1));
    }
    for &k in KS_SHARP.iter() {
        if k % 4 == 2 {
            continue;
        }
        let m = (n as u128) * (k as u128);
        if m > u64::MAX as u128 {
            continue;
        }
        let m = m as u64;
        let mut a = isqrt(m);
        if a * a < m {
            a += 1;
        }
        let step = match m % 4 {
            1 => {
                if a % 2 == 0 {
                    a += 1;
                }
                2
            }
            3 => {
                if a % 2 == 1 {
                    a += 1;
                }
                2
            }
            _ => 1,
        };
        for _ in 0..steps {
            let a2 = (a as u128) * (a as u128);
            if a2 >= m as u128 {
                let b2 = a2 - m as u128;
                if b2 <= u64::MAX as u128 {
                    let b2 = b2 as u64;
                    let r = isqrt(b2);
                    if r * r == b2 {
                        for big in [a - r, a + r] {
                            let g = gcd(big, n);
                            if 1 < g && g < n {
                                return Some((g, n / g, k));
                            }
                        }
                    }
                }
            }
            a += step;
        }
    }
    None
}

/// **The extended free horizon**: the first wheel-coprime composite the square, Fermat and fold
/// lanes all miss. Below it, a wheel survivor the fold lanes miss is prime, free and certain.
/// For the default wheel and eight Fermat steps it is 11441 = 17 × 673.
pub fn fold_horizon(lanes: &[u64], fermat_steps: u32) -> u64 {
    let mut n = horizon(lanes);
    loop {
        if lanes.iter().all(|&l| n % l != 0) && !crate::is_prime_u64(n) && !is_square(n) && fermat_walk(n, fermat_steps).is_none() && fold_sharp(n, 6).is_none() {
            return n;
        }
        n += 1;
    }
}

/// The deterministic part of the stack: lanes, square, Fermat, fold, the fold horizon.
pub fn free_verdict(x: u64, lanes: &[u64], fermat_steps: u32) -> Option<Verdict> {
    if let Some(v) = lane_verdict(x, lanes) {
        return Some(v);
    }
    if is_square(x) {
        return Some(Verdict { prime: false, how: format!("perfect square ({}^2)", isqrt(x)), cost: Cost::Free });
    }
    if let Some((p, q)) = fermat_walk(x, fermat_steps) {
        return Some(Verdict { prime: false, how: format!("near-square walk: {p}x{q}"), cost: Cost::Free });
    }
    if let Some((p, q, k)) = fold_sharp(x, 6) {
        return Some(Verdict { prime: false, how: format!("fold-wheel k={k}: {p}x{q}"), cost: Cost::Free });
    }
    let h = fold_horizon(lanes, fermat_steps);
    if x < h {
        return Some(Verdict { prime: true, how: format!("certified prime: below the fold-wheel horizon {h}, and no deterministic lane factors it"), cost: Cost::Free });
    }
    None
}

/// **The full stacked verdict**: free lanes, the walks, rho, and one real test last.
pub fn verdict(x: u64, lanes: &[u64], fermat_steps: u32) -> Verdict {
    if let Some(v) = free_verdict(x, lanes, fermat_steps) {
        return v;
    }
    if let Some((p, q)) = rho_walk(x, 20_000) {
        return Verdict { prime: false, how: format!("rho {p}×{q}"), cost: Cost::Cheap };
    }
    Verdict { prime: crate::is_prime_u64(x), how: "confirmed by test".into(), cost: Cost::Test }
}

/// The accounting for a range: how many settle by the wheel, below the horizon, by the walks,
/// by rho, and by a real test.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Dashboard {
    pub wheel: u64,
    pub below_horizon: u64,
    pub walk: u64,
    pub rho: u64,
    pub test: u64,
}

impl Dashboard {
    pub fn total(&self) -> u64 {
        self.wheel + self.below_horizon + self.walk + self.rho + self.test
    }
    /// The share settled by deterministic logic, in percent.
    pub fn deterministic_pct(&self) -> f64 {
        100.0 * (self.wheel + self.below_horizon + self.walk) as f64 / self.total().max(1) as f64
    }
}

pub fn dashboard(lo: u64, hi: u64, lanes: &[u64]) -> Dashboard {
    let h = horizon(lanes);
    let mut d = Dashboard::default();
    for x in lo.max(2)..hi {
        if crate::is_prime_u64(x) {
            if x < h {
                d.below_horizon += 1;
            } else {
                d.test += 1;
            }
        } else if lanes.iter().all(|&l| x % l != 0) {
            if is_square(x) || fermat_walk(x, 8).is_some() || fold_sharp(x, 6).is_some() {
                d.walk += 1;
            } else {
                d.rho += 1;
            }
        } else {
            d.wheel += 1;
        }
    }
    d
}
