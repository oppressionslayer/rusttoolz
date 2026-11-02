//! **The twin forge.** `k·2^n ± 1`: the `+1` side is a Proth number, `N − 1 = k·2^n` is factored
//! by construction, one Pocklington witness per prime factor proves it. The `−1` side has
//! `N + 1 = k·2^n`, and the Lucas test with a discriminant of Jacobi symbol −1 proves it. A
//! residue sieve on `k` kills most candidates before any test, and the hunt runs the survivors
//! in parallel.

use num_bigint::BigUint;
use crate::modpow::modpow;
use num_integer::Integer;
use num_traits::{One, Zero};
use rayon::prelude::*;

const WIT: [u32; 15] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47];

/// The distinct prime factors of `k·2^n`: 2 and the odd primes of `k`.
pub fn factor_small(mut k: u64) -> Vec<u64> {
    let mut f = vec![2u64];
    while k % 2 == 0 && k > 0 {
        k /= 2;
    }
    let mut d = 3;
    while d * d <= k {
        if k % d == 0 {
            f.push(d);
            while k % d == 0 {
                k /= d;
            }
        }
        d += 2;
    }
    if k > 1 && k != 2 {
        f.push(k);
    }
    f.sort_unstable();
    f.dedup();
    f
}

/// **The `+1` side.** `N = k·2^n + 1`; a witness `a` per prime `p | N − 1` with `a^(N−1) ≡ 1` and
/// `gcd(a^((N−1)/p) − 1, N) = 1`. With `2^n > √N` the factored part suffices.
pub fn proth_proven(n_val: &BigUint, fps: &[u64]) -> bool {
    let nm1 = n_val - BigUint::one();
    for &p in fps {
        let e = &nm1 / BigUint::from(p);
        let mut ok = false;
        for &a in WIT.iter() {
            let ab = BigUint::from(a);
            if !modpow(&ab, &nm1, n_val).is_one() {
                return false;
            }
            let g = (modpow(&ab, &e, n_val) + n_val - BigUint::one()) % n_val;
            if g.gcd(n_val).is_one() {
                ok = true;
                break;
            }
        }
        if !ok {
            return false;
        }
    }
    true
}

fn jacobi(a: &BigUint, m: &BigUint) -> i32 {
    let mut a = a % m;
    let mut m = m.clone();
    let mut r = 1i32;
    let (three, four, five, eight) = (BigUint::from(3u32), BigUint::from(4u32), BigUint::from(5u32), BigUint::from(8u32));
    while !a.is_zero() {
        while a.is_even() {
            a >>= 1;
            let m8 = &m % &eight;
            if m8 == three || m8 == five {
                r = -r;
            }
        }
        std::mem::swap(&mut a, &mut m);
        if &a % &four == three && &m % &four == three {
            r = -r;
        }
        a %= &m;
    }
    if m.is_one() { r } else { 0 }
}

/// `U_m, V_m` of the Lucas sequence `(P, Q)` modulo `mo`, by the binary chain.
pub fn lucas_uv(m: &BigUint, p: i64, q: i64, mo: &BigUint) -> (BigUint, BigUint) {
    let to = |v: i64| -> BigUint {
        if v >= 0 { BigUint::from(v as u64) % mo } else { (mo - (BigUint::from((-v) as u64) % mo)) % mo }
    };
    let (pb, qb) = (to(p), to(q));
    let d = to(p * p - 4 * q);
    let inv2 = modpow(&BigUint::from(2u32), &(mo - BigUint::from(2u32)), mo); // mo is odd and prime-tested
    let (mut u, mut v, mut qk) = (BigUint::zero(), BigUint::from(2u32) % mo, BigUint::one());
    let bits = m.bits();
    for i in (0..bits).rev() {
        let (nu, nv) = ((&u * &v) % mo, ((&v * &v) + mo * BigUint::from(2u32) - BigUint::from(2u32) * &qk) % mo);
        u = nu;
        v = nv;
        qk = (&qk * &qk) % mo;
        if m.bit(i) {
            let (nu, nv) = (((&pb * &u + &v) * &inv2) % mo, ((&d * &u + &pb * &v) * &inv2) % mo);
            u = nu;
            v = nv;
            qk = (&qk * &qb) % mo;
        }
    }
    (u, v)
}

/// **The `−1` side.** `N + 1 = k·2^n` factored; a Lucas sequence with `Jacobi(D, N) = −1` and
/// `U_{N+1} ≡ 0`, `gcd(U_{(N+1)/q}, N) = 1` for each prime `q | N + 1` proves `N`.
pub fn lucas_proven(n_val: &BigUint, fps: &[u64]) -> bool {
    if *n_val < BigUint::from(3u32) || n_val.is_even() || n_val.sqrt().pow(2) == *n_val {
        return false;
    }
    let np1 = n_val + BigUint::one();
    let mut ds: Vec<i64> = Vec::new();
    let mut d: i64 = 5;
    while ds.len() < 60 {
        let db = if d >= 0 { BigUint::from(d as u64) } else { n_val - (BigUint::from((-d) as u64) % n_val) };
        let j = jacobi(&db, n_val);
        if j == 0 && !(BigUint::from(d.unsigned_abs()) % n_val).is_zero() {
            return false;
        }
        if j == -1 {
            ds.push(d);
        }
        d = if d < 0 { -d + 2 } else { -d - 2 };
    }
    for &d in &ds {
        for p in (1..40).step_by(2) {
            let p: i64 = p;
            if (p * p - d) % 4 != 0 {
                continue;
            }
            let q = (p * p - d) / 4;
            let g = BigUint::from((2 * q.abs() * d.abs()) as u64).gcd(n_val);
            if !g.is_one() {
                continue;
            }
            if !lucas_uv(&np1, p, q, n_val).0.is_zero() {
                continue;
            }
            if fps.iter().all(|&f| lucas_uv(&(&np1 / BigUint::from(f)), p, q, n_val).0.gcd(n_val).is_one()) {
                return true;
            }
        }
    }
    false
}

/// What the forge proved about one `k`, `n`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Twin {
    pub k: u64,
    pub n: u32,
    pub digits: usize,
    pub upper: BigUint,
    pub lower: BigUint,
    pub upper_proven: bool,
    pub lower_probable: bool,
    pub lower_proven: bool,
}

/// **Prove a given pair** `k·2^n ± 1` both ways: Pocklington on the `+1`, Lucas on the `−1`.
pub fn prove_pair(k: u64, n: u32) -> Twin {
    let base = BigUint::one() << n;
    let mid = BigUint::from(k) * &base;
    let upper = &mid + BigUint::one();
    let lower = &mid - BigUint::one();
    let fps = factor_small(k);
    let upper_proven = proth_proven(&upper, &fps);
    let lower_probable = crate::prove::probable_prime(&lower);
    let lower_proven = lower_probable && lucas_proven(&lower, &fps);
    Twin { k, n, digits: upper.to_string().len(), upper, lower, upper_proven, lower_probable, lower_proven }
}

/// The residues of `k` modulo each odd prime to `bound` that kill one side: `k ≡ ±2^(−n) (mod p)`.
pub fn bad_residues(n: u32, bound: u64) -> Vec<(u64, u64, u64)> {
    crate::sieve(bound)
        .into_iter()
        .skip(1)
        .filter_map(|p| {
            let t = crate::powmod(2, n as u64, p);
            if t == 0 {
                return None;
            }
            let r = crate::powmod(t, p - 2, p);
            Some((p, r % p, (p - r) % p))
        })
        .collect()
}

/// The `k` in `[k_lo, k_hi)` no sieve prime kills on either side.
pub fn sieve_k(k_lo: u64, k_hi: u64, bad: &[(u64, u64, u64)]) -> Vec<u64> {
    let n = (k_hi - k_lo) as usize;
    let mut alive = vec![true; n];
    for &(p, r1, r2) in bad {
        for r in [r1, r2] {
            let mut j = (r + p - (k_lo % p)) % p;
            while (j as usize) < n {
                alive[j as usize] = false;
                j += p;
            }
        }
    }
    (0..n).filter(|&i| alive[i]).map(|i| k_lo + i as u64).collect()
}

/// How big the search is at `n`: the digit count and the expected `k` per `+1` prime and per twin.
pub fn estimate(n: u32) -> (usize, f64, f64) {
    let ln_n = n as f64 * std::f64::consts::LN_2;
    let digits = (n as f64 * std::f64::consts::LOG10_2) as usize + 1;
    (digits, ln_n / 2.0, ln_n * ln_n / (2.0 * 0.6601618))
}

/// **The hunt**: sieve `[k_lo, k_hi)`, screen every survivor's `+1` side in parallel, prove the
/// hits, and prove the `−1` side where it also passes the screen. Returns every `+1` prime found.
pub fn hunt(n: u32, k_lo: u64, k_hi: u64, sieve_bound: u64) -> Vec<Twin> {
    let bad = bad_residues(n, sieve_bound);
    let survivors = sieve_k(k_lo.max(1), k_hi, &bad);
    let base = BigUint::one() << n;
    let mut hits: Vec<Twin> = survivors
        .par_iter()
        .filter_map(|&k| {
            let upper = BigUint::from(k) * &base + BigUint::one();
            if !crate::prove::probable_prime(&upper) {
                return None;
            }
            Some(prove_pair(k, n))
        })
        .collect();
    hits.sort_by_key(|t| t.k);
    hits
}

impl Twin {
    /// **The packet the Python forge writes**, one JSON object per line: `k, n, digits, upper,
    /// upper_proven_pocklington, lower, lower_status` and, when the −1 side was proven,
    /// `lower_proven_lucas`. `sitoolz.showcase.twins()` reads these back.
    pub fn packet(&self) -> String {
        let status = if self.lower_proven || self.lower_probable { "probable_prime" } else { "composite" };
        let mut s = format!(
            "{{\"k\": {}, \"n\": {}, \"digits\": {}, \"upper\": \"{}\", \"upper_proven_pocklington\": {}, \"lower\": \"{}\", \"lower_status\": \"{}\"",
            self.k, self.n, self.digits, self.upper, self.upper_proven, self.lower, status
        );
        if self.lower_probable {
            s.push_str(&format!(", \"lower_proven_lucas\": {}", self.lower_proven));
        }
        s.push('}');
        s
    }
}

/// **What a streaming hunt needs**: where `k` starts and stops, the span per round, the sieve
/// bound, how many fully proven pairs to stop at, and where the packets and the checkpoint go.
#[derive(Clone, Debug)]
pub struct Hunt {
    pub n: u32,
    pub k_start: u64,
    pub k_stop: u64,
    pub span: u64,
    pub sieve_bound: u64,
    pub want_twins: usize,
    pub out: Option<std::path::PathBuf>,
    pub checkpoint: Option<std::path::PathBuf>,
}

impl Hunt {
    pub fn new(n: u32, k_start: u64, k_stop: u64) -> Hunt {
        Hunt { n, k_start, k_stop, span: 2_000_000, sieve_bound: 100_000, want_twins: usize::MAX, out: None, checkpoint: None }
    }
}

/// **A hunt that streams and resumes.** Spans of `k` from `k_start`, each sieved and tested in
/// parallel; every `+1` prime is appended to `out` as a packet line and the next `k` is written
/// to `checkpoint`, so a stopped hunt picks up where it left off. Stops after `want_twins` fully
/// proven pairs or when `k` reaches `k_stop`. Returns the proven twins.
pub fn hunt_stream(h: &Hunt) -> std::io::Result<Vec<Twin>> {
    let (n, k_stop, span, sieve_bound, want_twins) = (h.n, h.k_stop, h.span, h.sieve_bound, h.want_twins);
    let (out, checkpoint) = (h.out.as_deref(), h.checkpoint.as_deref());
    let mut k = h.k_start;
    use std::io::Write;
    if let Some(cp) = checkpoint {
        if let Ok(t) = std::fs::read_to_string(cp) {
            if let Some(v) = t.split(':').nth(1).and_then(|x| x.trim().trim_end_matches('}').parse::<u64>().ok()) {
                k = v;
            }
        }
    }
    let mut twins = Vec::new();
    while k < k_stop && twins.len() < want_twins {
        let hi = (k + span).min(k_stop);
        let hits = hunt(n, k, hi, sieve_bound);
        if let Some(path) = out {
            let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
            for h in &hits {
                writeln!(f, "{}", h.packet())?;
            }
        }
        twins.extend(hits.into_iter().filter(|h| h.lower_proven));
        k = hi;
        if let Some(cp) = checkpoint {
            std::fs::write(cp, format!("{{\"next_k\": {k}}}"))?;
        }
    }
    Ok(twins)
}
