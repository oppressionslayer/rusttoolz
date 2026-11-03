//! **The Pocklington ladder.** From a proven prime `q`, `N = 2·R·q + 1` with `q > √N` is proven
//! by one witness `a`: `a^(N−1) ≡ 1 (mod N)` and `gcd(a^((N−1)/q) − 1, N) = 1`. Roll again with
//! `q = N`. Every rung's `N − 1` is factored by construction, so the certificate is the climb.

use num_bigint::BigUint;
use crate::modpow::modpow;
use num_integer::Integer;
use num_traits::{One, Zero};
use rayon::prelude::*;

pub const WITNESSES: [u32; 12] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];

/// One rung: `n` proven from `q` by witness `a`; a seed carries `q = 0`, `a = 0`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rung {
    pub n: BigUint,
    pub q: BigUint,
    pub a: u32,
}

impl Rung {
    pub fn is_seed(&self) -> bool {
        self.q.is_zero()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Proven(u32),
    Composite,
    Inconclusive,
}

/// A strong probable-prime screen with the twelve witnesses: exact below `3.18·10^23` (the
/// primes to 37; a thirteenth base, 41, would reach `3.3·10^24`), a gate against Fermat
/// pseudoprimes above.
pub fn probable_prime(n: &BigUint) -> bool {
    let two = BigUint::from(2u32);
    if *n < two {
        return false;
    }
    for &p in WITNESSES.iter() {
        let p = BigUint::from(p);
        if (n % &p).is_zero() {
            return *n == p;
        }
    }
    let nm1 = n - BigUint::one();
    let s = nm1.trailing_zeros().unwrap_or(0);
    let d = &nm1 >> s;
    'w: for &a in WITNESSES.iter() {
        let mut x = modpow(&BigUint::from(a), &d, n);
        if x.is_one() || x == nm1 {
            continue;
        }
        for _ in 1..s {
            x = (&x * &x) % n;
            if x == nm1 {
                continue 'w;
            }
        }
        return false;
    }
    true
}

/// **One Pocklington step with `q` supplied.** `q | N − 1`, `q > √N`, then a witness. Gated by
/// the probable-prime screen so a composite `q` cannot smuggle a pseudoprime through.
///
/// **The proof is only as good as `q`.** The theorem needs `q` prime; this step does not check
/// that, the ladder does, by handing in the rung it has already proven. A `q` built from the
/// shape of `N` alone — `2^(p−1) − 1` for a Mersenne, say, which is divisible by 3 — proves
/// nothing even when the witness arithmetic comes out 1.
pub fn pocklington(n: &BigUint, q: &BigUint) -> Verdict {
    if *n < BigUint::from(4u32) {
        return if *n < BigUint::from(2u32) { Verdict::Composite } else { Verdict::Proven(0) };
    }
    let nm1 = n - BigUint::one();
    if !(&nm1 % q).is_zero() || q * q <= *n {
        return Verdict::Inconclusive;
    }
    if !probable_prime(n) {
        return Verdict::Composite;
    }
    let e = &nm1 / q;
    for &a in WITNESSES.iter() {
        let ab = BigUint::from(a) % n;
        if !modpow(&ab, &nm1, n).is_one() {
            return Verdict::Composite;
        }
        let g = (modpow(&ab, &e, n) + n - BigUint::one()) % n;
        if g.gcd(n).is_one() {
            return Verdict::Proven(a);
        }
    }
    Verdict::Inconclusive
}

/// The witness test alone, with `q` and `a` supplied: one modpow and one gcd.
pub fn gcd_certify(n: &BigUint, q: &BigUint, a: u32) -> bool {
    let nm1 = n - BigUint::one();
    if q * q <= *n || !(&nm1 % q).is_zero() {
        return false;
    }
    let ab = BigUint::from(a) % n;
    if !modpow(&ab, &nm1, n).is_one() {
        return false;
    }
    let g = (modpow(&ab, &(&nm1 / q), n) + n - BigUint::one()) % n;
    g.gcd(n).is_one()
}

/// **Lucas–Lehmer**: `2^p − 1` is prime iff `s_{p−2} ≡ 0`, `s_0 = 4`, `s = s² − 2`.
pub fn lucas_lehmer(p: u32) -> bool {
    if p == 2 {
        return true;
    }
    if p < 2 || !crate::is_prime_u64(p as u64) {
        return false;
    }
    let m = (BigUint::one() << p) - BigUint::one();
    let mut s = BigUint::from(4u32);
    for _ in 0..p - 2 {
        s = (&s * &s + &m - BigUint::from(2u32)) % &m;
    }
    s.is_zero()
}

/// The Mersenne prime `2^p − 1` as a seed rung, when Lucas–Lehmer proves it.
pub fn mersenne_seed(p: u32) -> Option<Rung> {
    lucas_lehmer(p).then(|| Rung { n: (BigUint::one() << p) - BigUint::one(), q: BigUint::zero(), a: 0 })
}

/// A seed rung for a prime the net proves.
pub fn net_seed(net: &crate::net::Net, p: u64) -> Option<Rung> {
    matches!(net.is_prime(p), Ok(true)).then(|| Rung { n: BigUint::from(p), q: BigUint::zero(), a: 0 })
}

/// **Roll a seed up to at least `bits`.** Each rung takes `R` just under `q/2`, the largest the
/// theorem allows, so the bit length about doubles per rung and the top is reached in a handful
/// of rungs: M127 to 4096 bits is five. The last rung is sized to land on `bits` rather than
/// overshoot it. `R` is sieved against the small primes and each candidate screened with Fermat
/// base 2 before the twelve-witness gate, so a rung costs a few hundred modpows.
pub fn build_up(seed: Rung, bits: u64) -> Vec<Rung> {
    let small = crate::sieve(20_000);
    let mut chain = vec![seed];
    while chain.last().unwrap().n.bits() < bits {
        let q = chain.last().unwrap().n.clone();
        let two_q = BigUint::from(2u32) * &q;
        let barred: Vec<(u64, u64)> = small
            .iter()
            .skip(1)
            .filter_map(|&p| {
                let t = u64::try_from(&two_q % p).unwrap();
                if t == 0 {
                    return None;
                }
                let inv = crate::powmod(t, p - 2, p);
                Some((p, (p - inv) % p))
            })
            .collect();
        let (cap, start) = rung_start(&q, bits);
        // **candidates in parallel**: a batch of R below the start, every core on its own Fermat
        // screen, and the largest R that proves is the rung (the batch is small, so the bit
        // length stays what the start set)
        let batch: usize = (rayon::current_num_threads() * 8).max(8);
        // batches walk down from the start; when the few below it are spent, they walk up
        let mut top = start.clone();
        let mut up = false;
        loop {
            let cands: Vec<BigUint> = (0..batch as u64)
                .map(|d| if up { &top + BigUint::from(d) } else if top > BigUint::from(d) { &top - BigUint::from(d) } else { BigUint::zero() })
                .filter(|r| !r.is_zero() && *r <= cap)
                .filter(|r| {
                    // a rung that is itself a small prime (2·3·1 + 1 = 7) is barred by its own
                    // residue; below the sieve bound the candidate is tested, not sieved
                    if &two_q * r + BigUint::one() <= BigUint::from(20_000u32) {
                        return true;
                    }
                    let r_small: u64 = u64::try_from(r % 20_000u64).unwrap();
                    !barred.iter().any(|&(p, b)| r_small % p == b)
                })
                .collect();
            let found: Option<Rung> = cands
                .par_iter()
                .filter_map(|r| {
                    let n = &two_q * r + BigUint::one();
                    if !modpow(&BigUint::from(2u32), &(&n - BigUint::one()), &n).is_one() {
                        return None;
                    }
                    match pocklington(&n, &q) {
                        Verdict::Proven(a) => Some(Rung { n, q: q.clone(), a }),
                        _ => None,
                    }
                })
                .max_by(|x, y| x.n.cmp(&y.n));
            if let Some(rung) = found {
                chain.push(rung);
                break;
            }
            if up {
                top += BigUint::from(batch as u64);
                // every R to the cap has been tried: say so rather than spin
                assert!(top <= cap, "no rung above q = {q} lands on {bits} bits: every R to (q-1)/2 was tried");
            } else if top > BigUint::from(batch as u64) {
                top -= BigUint::from(batch as u64);
            } else {
                up = true;
                top = &start + BigUint::one();
            }
        }
    }
    chain
}

/// **Where a rung's search starts.** The cap is the largest `R` the theorem allows, `(q − 1)/2`;
/// the start is `2^want` when that lands on `bits`, else the cap. The climb walks down from the
/// start in batches and keeps the largest `R` that proves, so the rung is a function of `q` and
/// `bits` alone and its **offset** `start − R` names it in a few digits.
pub fn rung_start(q: &BigUint, bits: u64) -> (BigUint, BigUint) {
    let cap: BigUint = (q - BigUint::one()) >> 1u32;
    let want = bits.saturating_sub(q.bits() + 1);
    let start: BigUint = if want < cap.bits() { BigUint::one() << want } else { cap.clone() };
    (cap, start)
}

/// **The recipe of a climb**: for each rung above the seed, `start − R` (negative when the walk
/// went up), so the chain is the seed, the target, and one small integer per rung. `None` when
/// the chain was not one climb to `bits`: a ladder resumed toward several targets has rungs
/// whose starts were set by other targets, and their offsets are not small numbers.
pub fn recipe(chain: &[Rung], bits: u64) -> Option<Vec<i64>> {
    chain
        .windows(2)
        .map(|w| {
            let (q, n) = (&w[0].n, &w[1].n);
            let r = (n - BigUint::one()) / (BigUint::from(2u32) * q);
            let (_, start) = rung_start(q, bits);
            let (s, r) = (num_bigint::BigInt::from(start), num_bigint::BigInt::from(r));
            i64::try_from(s - r).ok()
        })
        .collect()
}

/// **A climb back from its recipe**, no search: each rung's `R` is `start − offset`, and the
/// Pocklington step is run once to find the witness, so the result is proven and not merely
/// recalled. `None` when an offset does not land on a rung.
pub fn rebuild(seed: Rung, bits: u64, offsets: &[i64]) -> Option<Vec<Rung>> {
    let mut chain = vec![seed];
    for &off in offsets {
        let q = chain.last().unwrap().n.clone();
        let (cap, start) = rung_start(&q, bits);
        let r = num_bigint::BigInt::from(start) - num_bigint::BigInt::from(off);
        let r = BigUint::try_from(r).ok()?;
        if r.is_zero() || r > cap {
            return None;
        }
        let n = BigUint::from(2u32) * &q * &r + BigUint::one();
        match pocklington(&n, &q) {
            Verdict::Proven(a) => chain.push(Rung { n, q, a }),
            _ => return None,
        }
    }
    Some(chain)
}

/// **Lars's descent: the certificate from the top prime alone.** A doubling rung takes
/// `R = (q − 1)/2 − offset` with the offset a few thousand at most, so
/// `N = 2·q·R + 1 = q² − q·(2·offset + 1) + 1`: **the rung below is the square root of the rung
/// above**, `q = isqrt(N) + offset + 1` to within the offset. So from the top prime alone: take
/// the integer square root, walk a few thousand steps up from it, and the value that divides
/// `(N − 1)/2` and is prime is `q`; the Pocklington step proves `N` from it; then take the
/// square root of `q`, down to a Mersenne. Nothing is stored and no primality is searched for:
/// every `q` is read off the number above it, and the chain comes back seed first for
/// `verify_certificate` to replay. Measured on the M127 ladder: every doubling rung's `q` sits
/// within 4,000 of `isqrt(N)`.
///
/// What it cannot read: a rung **sized to land on a bit count** (`rollball -- 11000` from 10,000
/// bits) has `R` near `2^want` instead, nowhere near `q/2`, so no root recovers it. The descent
/// stops there with a non-Mersenne bottom, `verify_certificate` says so, and the store holds
/// those rungs. `window` is how far above the root to look; 8,192 covers every offset seen.
pub fn descend(top: &BigUint, floor_bits: u64, window: u64) -> Vec<Rung> {
    let mut rungs: Vec<Rung> = Vec::new();
    let mut n = top.clone();
    loop {
        if mersenne_big(&n) || n.bits() <= floor_bits {
            rungs.push(Rung { n, q: BigUint::zero(), a: 0 });
            break;
        }
        let half = (&n - BigUint::one()) >> 1u32;
        let root = n.sqrt();
        let mut found: Option<Rung> = None;
        for d in 0..=window {
            let q = &root + BigUint::from(d);
            if (&half % &q).is_zero() && probable_prime(&q) {
                if let Verdict::Proven(a) = pocklington(&n, &q) {
                    found = Some(Rung { n: n.clone(), q, a });
                    break;
                }
            }
        }
        match found {
            Some(r) => {
                n = r.q.clone();
                rungs.push(r);
            }
            None => {
                rungs.push(Rung { n, q: BigUint::zero(), a: 0 });
                break;
            }
        }
    }
    rungs.reverse();
    rungs
}

/// The same climb with the **smallest** `R` at every rung: the Cunningham-like ladder, many
/// short rungs, each `N` as close above `2q` as a prime allows.
pub fn build_up_small(seed: Rung, bits: u64) -> Vec<Rung> {
    let mut chain = vec![seed];
    while chain.last().unwrap().n.bits() < bits {
        let q = chain.last().unwrap().n.clone();
        let two_q = BigUint::from(2u32) * &q;
        let mut r: u64 = 1;
        loop {
            let n = &two_q * r + BigUint::one();
            if modpow(&BigUint::from(2u32), &(&n - BigUint::one()), &n).is_one() {
                if let Verdict::Proven(a) = pocklington(&n, &q) {
                    chain.push(Rung { n, q, a });
                    break;
                }
            }
            r += 1;
        }
    }
    chain
}

/// **Replay a certificate from the seed up.** A seed rung must be a Mersenne prime by
/// Lucas–Lehmer or a prime within `net`'s reach; every other rung must chain from the one below
/// and pass its witness.
pub fn verify_certificate(chain: &[Rung], net: Option<&crate::net::Net>) -> bool {
    let mut prev: Option<&BigUint> = None;
    for r in chain {
        if r.is_seed() {
            let ok = match u64::try_from(&r.n) {
                Ok(v) => net.map(|nt| matches!(nt.is_prime(v), Ok(true))).unwrap_or(false) || mersenne_u64(v),
                Err(_) => mersenne_big(&r.n),
            };
            if !ok {
                return false;
            }
        } else if prev != Some(&r.q) || !gcd_certify(&r.n, &r.q, r.a) {
            return false;
        }
        prev = Some(&r.n);
    }
    !chain.is_empty()
}

fn mersenne_u64(v: u64) -> bool {
    v > 1 && (v + 1).is_power_of_two() && (known_mersenne((v + 1).trailing_zeros()) || lucas_lehmer((v + 1).trailing_zeros()))
}

fn mersenne_big(n: &BigUint) -> bool {
    let p = n.bits();
    let m = (BigUint::one() << p) - BigUint::one();
    *n == m && p <= u32::MAX as u64 && (known_mersenne(p as u32) || lucas_lehmer(p as u32))
}

/// **The 52 Mersenne primes are certified facts**, not work: a seed `2^p − 1` with `p` in
/// [`crate::numtoolz::MERSENNE_EXPONENTS`] is accepted without replaying Lucas–Lehmer, which
/// for `p = 82,589,933` no laptop would finish. Any other exponent is still tested.
pub fn known_mersenne(p: u32) -> bool {
    crate::numtoolz::MERSENNE_EXPONENTS.contains(&(p as u64))
}

/// A seed rung on a known Mersenne prime, no Lucas–Lehmer: `None` unless `p` is one of the 52.
pub fn known_mersenne_seed(p: u32) -> Option<Rung> {
    known_mersenne(p).then(|| Rung { n: (BigUint::one() << p) - BigUint::one(), q: BigUint::zero(), a: 0 })
}

/// The odd part of `x`.
pub fn odd_core(x: &BigUint) -> BigUint {
    if x.is_zero() {
        return BigUint::zero();
    }
    x >> x.trailing_zeros().unwrap_or(0)
}
