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
