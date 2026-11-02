//! **rusttoolz**: Lars Rocha's sitoolz, the parts with real compute, in Rust.
//!
//! - [`net`]: the prime net. Every prime to a limit stored as half-gaps, proving any number to
//!   the square of the largest by elimination, factoring it, generating any window, counting.
//! - [`prove`]: the Pocklington ladder. `N = 2·R·q + 1` off a proven `q`, one witness per rung;
//!   `build_up` rolls a seed to any size and `verify_certificate` replays the climb.
//! - [`twin`]: the twin forge. `k·2^n ± 1`, the `+1` side proven by Pocklington, the `−1` side
//!   by Lucas, a residue sieve on `k`, and a parallel hunt.
//! - [`xortri`]: the XOR difference triangle and its closed form.
//! - [`lanes`]: rails, lanes, gates and folds: the free-logic verdict stack that settles 88% of a
//!   range with no primality test.
//! - [`gaps`]: the gap forge: the distances between primes, twins, records, and how a number sieves them.
//! - [`mont`]: Montgomery multiplication on `u64` limbs, the modpow under the ladder and the forge.
//! - [`numtoolz`]: the Mersenne exponents, cycle8, the RT sieve and its pure-prime cap, the linear sieve.
//! - [`buildsystem`]: the XOR build system and the structured index.
//! - [`edgelaw`]: rule 30's light-cone edge by lookup, verified to 2^49.
//! - [`city`]: Lars's addressed city, the law behind the Spindexin district builder and the engine's `wlaw::city`.
//! - [`motion`]: the one tick law of speed the site and the engine drive by.
//! - [`contact`]: first contact along a straight segment, in exact integers.
//! - [`cert`]: certificates as text in the Python and JavaScript conventions, so all three verify each other.
//!
//! Pure Rust by default, the wasm path: big integers are `num-bigint`, everything within the net's
//! reach is `u64`. Native builds may turn on GMP with `--features gmp` ([`modpow`]).

pub mod buildsystem;
pub mod cert;
pub mod city;
pub mod contact;
pub mod convoy;
pub mod zipper;
pub mod district;
pub mod edgelaw;
pub mod gaps;
pub mod lanes;
pub mod modpow;
pub mod mont;
pub mod motion;
pub mod net;
pub mod numtoolz;
pub mod prove;
pub mod twin;
pub mod xortri;

/// `a·b mod m` without overflow.
#[inline]
pub fn mulmod(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128 * b as u128) % m as u128) as u64
}

/// `a^e mod m`.
pub fn powmod(mut a: u64, mut e: u64, m: u64) -> u64 {
    let mut r = 1u64;
    a %= m;
    while e > 0 {
        if e & 1 == 1 {
            r = mulmod(r, a, m);
        }
        a = mulmod(a, a, m);
        e >>= 1;
    }
    r
}

/// Deterministic Miller–Rabin, exact for every `u64`.
pub fn is_prime_u64(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    for p in [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        if n % p == 0 {
            return n == p;
        }
    }
    let d = (n - 1) >> (n - 1).trailing_zeros();
    let s = (n - 1).trailing_zeros();
    'witness: for a in [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        let mut x = powmod(a, d, n);
        if x == 1 || x == n - 1 {
            continue;
        }
        for _ in 1..s {
            x = mulmod(x, x, n);
            if x == n - 1 {
                continue 'witness;
            }
        }
        return false;
    }
    true
}

/// A plain sieve: every prime to `limit`, inclusive.
pub fn sieve(limit: u64) -> Vec<u64> {
    if limit < 2 {
        return Vec::new();
    }
    let n = limit as usize + 1;
    let mut s = vec![true; n];
    s[0] = false;
    s[1] = false;
    let mut i = 2;
    while i * i < n {
        if s[i] {
            let mut j = i * i;
            while j < n {
                s[j] = false;
                j += i;
            }
        }
        i += 1;
    }
    (0..n).filter(|&i| s[i]).map(|i| i as u64).collect()
}
