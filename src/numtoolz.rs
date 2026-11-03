//! **The `numtoolz` half of sitoolz**: the Mersenne exponents and what hangs off them, the
//! cycle8 corner identity, the RT sieve of doubled primes and the cap below which its survivors
//! are pure primes, the linear sieve that marks every composite once, and the prime deltas.

use num_bigint::BigUint;
use num_traits::One;

/// The 52 known Mersenne prime exponents, in order.
pub const MERSENNE_EXPONENTS: [u64; 52] = [
    2, 3, 5, 7, 13, 17, 19, 31, 61, 89, 107, 127, 521, 607, 1279, 2203, 2281, 3217, 4253, 4423, 9689,
    9941, 11213, 19937, 21701, 23209, 44497, 86243, 110503, 132049, 216091, 756839, 859433, 1257787,
    1398269, 2976221, 3021377, 6972593, 13466917, 20996011, 24036583, 25964951, 30402457, 32582657,
    37156667, 42643801, 43112609, 57885161, 74207281, 77232917, 82589933, 136279841,
];

/// `2^p − 1` for the first `count` exponents.
pub fn mersenne_primes(count: usize) -> Vec<BigUint> {
    MERSENNE_EXPONENTS.iter().take(count).map(|&p| (BigUint::one() << p) - BigUint::one()).collect()
}

/// **The cycle8 corner identity**: at index `2^(p−3) − 1` the cycle `8·N` reads `2^p − 8`, and
/// XOR 7 lifts it to `2^p − 1`. The same numbers as `mersenne_primes`, reached through cycle8.
pub fn mersenne_via_cycle8(count: usize) -> Vec<(u64, BigUint)> {
    MERSENNE_EXPONENTS
        .iter()
        .take(count)
        .map(|&p| {
            let m = if p < 3 { (BigUint::one() << p) - BigUint::one() } else { (BigUint::from(8u32) * ((BigUint::one() << (p - 3)) - BigUint::one())) ^ BigUint::from(7u32) };
            (p, m)
        })
        .collect()
}

/// The even perfect numbers, `2^(p−1)·(2^p − 1)`, one per Mersenne prime (Euclid–Euler).
pub fn perfect_numbers(count: usize) -> Vec<(u64, BigUint)> {
    MERSENNE_EXPONENTS.iter().take(count).map(|&p| (p, (BigUint::one() << (p - 1)) * ((BigUint::one() << p) - BigUint::one()))).collect()
}

/// The cycle: `0, step, 2·step, ...`, `side²` of them, so it reshapes into a square.
pub fn cycle8(side: u64, step: u64) -> Vec<u64> {
    (0..side * side).map(|i| i * step).collect()
}

/// **RT**: twice every prime to 127, the session's array.
pub const RT: [u64; 31] = [4, 6, 10, 14, 22, 26, 34, 38, 46, 58, 62, 74, 82, 86, 94, 106, 118, 122, 134, 142, 146, 158, 166, 178, 194, 202, 206, 214, 218, 226, 254];

/// `2·p` for the first `k` primes: RT dialled to any size.
pub fn extend_rt(k: usize) -> Vec<u64> {
    let mut out = Vec::with_capacity(k);
    let mut n = 2u64;
    while out.len() < k {
        if crate::is_prime_u64(n) {
            out.push(2 * n);
        }
        n += 1;
    }
    out
}

/// Each `n` of `nums` striking `n, 2n, 3n, ...` below `cap` (at or below with `inclusive`).
pub fn self_sieve(nums: &[u64], cap: u64, inclusive: bool) -> Vec<(u64, Vec<u64>)> {
    nums.iter()
        .map(|&n| {
            let mut v = Vec::new();
            let mut m = n;
            while m < cap || (inclusive && m == cap) {
                v.push(m);
                m += n;
            }
            (n, v)
        })
        .collect()
}

/// **The survivors**: the evens below `cap` no number of `nums` ever strikes. Only 2 survives
/// small caps; the real survivors start at 262 = 2·131, the first even whose half dodges every
/// prime to 127.
pub fn rt_survivors(nums: &[u64], cap: u64) -> Vec<u64> {
    (2..cap).step_by(2).filter(|&e| nums.iter().all(|&n| e % n != 0)).collect()
}

/// Below this cap every survivor of `2·{primes to P}` is `2·prime`: `2·next_prime(P)²`. For RT it
/// is 34,322, the first composite survivor the session found.
pub fn pure_prime_cap(base: &[u64]) -> u64 {
    let p = base.iter().max().copied().unwrap_or(2) / 2;
    let mut q = p + 1;
    while !crate::is_prime_u64(q) {
        q += 1;
    }
    2 * q * q
}

/// **The linear sieve**: every composite marked exactly once, by its smallest prime factor. The
/// primes to `n` and, beside them, `spf[x]` for every `x` to `n`.
pub fn linear_sieve(n: u64) -> (Vec<u64>, Vec<u64>) {
    let n = n as usize;
    let mut spf = vec![0u64; n + 1];
    let mut primes = Vec::new();
    for i in 2..=n {
        if spf[i] == 0 {
            spf[i] = i as u64;
            primes.push(i as u64);
        }
        for &p in &primes {
            let ip = i * p as usize;
            if p > spf[i] || ip > n {
                break;
            }
            spf[ip] = p;
        }
    }
    (primes, spf)
}

/// `delta[k] = primes[k] ^ primes[k+1]`.
pub fn primes_to_deltas(primes: &[u64]) -> Vec<u64> {
    primes.windows(2).map(|w| w[0] ^ w[1]).collect()
}

/// The primes back from their XOR deltas and the seed.
pub fn deltas_to_primes(deltas: &[u64], seed: u64) -> Vec<u64> {
    let mut out = vec![seed];
    for &d in deltas {
        let next = out.last().unwrap() ^ d;
        out.push(next);
    }
    out
}

/// The digital root of a decimal integer.
pub fn digital_root(n: &BigUint) -> u32 {
    use num_traits::Zero;
    if n.is_zero() {
        return 0;
    }
    let r = u32::try_from(n % BigUint::from(9u32)).unwrap();
    if r == 0 { 9 } else { r }
}

/// **Lars's all-ones step.** `2^a − 1` is `2^b − 1` shifted up by `a − b` with the gap filled
/// with ones: `(n << gap) ^ ((1 << gap) − 1)`, and the two pieces share no set bit so XOR, OR
/// and addition agree. `extend_mersenne(M89, 18) = M107`, `extend_mersenne(M107, 20) = M127`.
/// It holds for every gap, prime exponent or not; which gaps land on a prime is the Mersenne
/// sequence itself.
pub fn extend_mersenne(n: &BigUint, gap: u64) -> BigUint {
    (n << gap) ^ ((BigUint::one() << gap) - BigUint::one())
}

/// The differences between successive known exponents: the Mersenne sequence as the gaps that
/// `extend_mersenne` walks from a seed. `M2` and `gaps` rebuild all 52.
pub fn mersenne_gaps() -> Vec<u64> {
    MERSENNE_EXPONENTS.windows(2).map(|w| w[1] - w[0]).collect()
}

/// **The quick fail.** Every factor of `2^p − 1` (p an odd prime) is `q = 2kp + 1` with
/// `q ≡ ±1 (mod 8)`, so the only candidates are those, and each is one `powmod`: `2^p mod q`
/// is 1 or it is not. The first factor found up to `k_max`, or `None` when none is: a Mersenne
/// number with no such factor still has to face Lucas–Lehmer; one with a factor never does.
/// `q` stays below 2^63, so `k_max` is capped for large `p`.
pub fn mersenne_factor(p: u64, k_max: u64) -> Option<u64> {
    if p < 3 || !crate::is_prime_u64(p) {
        return None;
    }
    let k_cap = ((1u64 << 63) - 1) / (2 * p);
    for k in 1..=k_max.min(k_cap) {
        let q = 2 * k * p + 1;
        // for a small p the candidate can reach 2^p − 1 itself, which divides itself
        if p < 64 && q * q > (1u64 << p) - 1 {
            return None;
        }
        let r = q & 7;
        if r != 1 && r != 7 {
            continue;
        }
        // a composite q never beats its own prime factors to the front: skip the obvious
        if q % 3 == 0 || q % 5 == 0 || q % 7 == 0 || q % 11 == 0 || q % 13 == 0 {
            continue;
        }
        if crate::powmod(2, p, q) == 1 {
            return Some(q);
        }
    }
    None
}

/// **Where the next one is expected**, by the Lenstra–Pomerance–Wagstaff heuristic: exponents
/// grow by about `2^(1/e^γ) ≈ 1.4757` on average. Returns the ratio fitted to the known
/// exponents (the geometric mean of successive ratios), the expected next exponent from the
/// largest known, and the heuristic's own constant for comparison. A guess of odds, not of a prime.
pub fn estimate_next() -> (f64, f64, f64) {
    let e = &MERSENNE_EXPONENTS;
    let logs: f64 = e.windows(2).map(|w| (w[1] as f64 / w[0] as f64).ln()).sum();
    let fitted = (logs / (e.len() - 1) as f64).exp();
    let gamma = 0.577_215_664_901_532_9_f64;
    let heuristic = 2f64.powf(1.0 / gamma.exp());
    (fitted, *e.last().unwrap() as f64 * fitted, heuristic)
}

#[cfg(test)]
mod mersenne_tests {
    use super::*;

    #[test]
    fn the_quick_fail_finds_the_classic_factors_and_spares_the_primes() {
        assert_eq!(mersenne_factor(11, 10), Some(23)); // 2047 = 23 · 89, k = 1
        assert_eq!(mersenne_factor(23, 10), Some(47)); // k = 1
        assert_eq!(mersenne_factor(29, 10), Some(233)); // k = 4
        assert_eq!(mersenne_factor(37, 10), Some(223)); // k = 3
        assert_eq!(mersenne_factor(67, 1000), None, "M67's factors are too big for k ≤ 1000");
        for &p in MERSENNE_EXPONENTS.iter().take(20) {
            assert_eq!(mersenne_factor(p, 10_000), None, "M{p} is prime");
        }
        assert_eq!(mersenne_factor(15, 10), None, "not a prime exponent");
    }

    #[test]
    fn the_all_ones_step_walks_the_whole_list_from_m2() {
        let m = |p: u64| (BigUint::one() << p) - BigUint::one();
        assert_eq!(extend_mersenne(&m(89), 18), m(107));
        assert_eq!(extend_mersenne(&m(107), 20), m(127));
        assert_eq!(extend_mersenne(&m(7), 4), m(11), "every gap, prime or not: 2047");
        let mut n = m(2);
        for g in mersenne_gaps().into_iter().take(20) {
            n = extend_mersenne(&n, g);
        }
        assert_eq!(n, m(MERSENNE_EXPONENTS[20]));
    }

    #[test]
    fn the_estimate_is_near_the_heuristic_constant() {
        let (fitted, next, heuristic) = estimate_next();
        assert!((fitted - heuristic).abs() < 0.12, "{fitted} vs {heuristic}");
        assert!(next > 136_279_841.0 && next < 250_000_000.0);
    }
}
