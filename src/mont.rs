//! **Montgomery multiplication on `u64` limbs**: the classic CIOS loop with `u128` intermediates
//! for an odd modulus, and a windowed modpow on top of it. Pure Rust. Measured against
//! `num-bigint`'s own modpow it comes out even from 512 to 8,192 bits, because `num-bigint`
//! already reduces in Montgomery form; the ladder and the forge keep `num-bigint`'s, and this
//! module is the place a Karatsuba-in-Montgomery multiply would go to pull ahead.

use num_bigint::BigUint;
use num_traits::{One, Zero};

/// An odd modulus prepared for Montgomery arithmetic.
#[derive(Clone, Debug)]
pub struct Mont {
    pub n: Vec<u64>,
    /// `−n^(−1) mod 2^64`.
    pub n0inv: u64,
    /// `R² mod n` with `R = 2^(64·limbs)`, for conversion into Montgomery form.
    pub r2: Vec<u64>,
    pub modulus: BigUint,
}

fn limbs(x: &BigUint, len: usize) -> Vec<u64> {
    let mut v = x.to_u64_digits();
    v.resize(len, 0);
    v
}

fn from_limbs(v: &[u64]) -> BigUint {
    let mut bytes = Vec::with_capacity(v.len() * 8);
    for d in v {
        bytes.extend_from_slice(&d.to_le_bytes());
    }
    BigUint::from_bytes_le(&bytes)
}

impl Mont {
    /// `None` for an even or zero modulus.
    pub fn new(modulus: &BigUint) -> Option<Mont> {
        if modulus.is_zero() || !modulus.bit(0) {
            return None;
        }
        let n = modulus.to_u64_digits();
        let k = n.len();
        // n0inv = -n^-1 mod 2^64 by Newton's iteration
        let n0 = n[0];
        let mut inv: u64 = 1;
        for _ in 0..6 {
            inv = inv.wrapping_mul(2u64.wrapping_sub(n0.wrapping_mul(inv)));
        }
        let n0inv = inv.wrapping_neg();
        let r = BigUint::one() << (64 * k as u64);
        let r2 = limbs(&((&r * &r) % modulus), k);
        Some(Mont { n, n0inv, r2, modulus: modulus.clone() })
    }

    /// `a·b·R^(−1) mod n`, CIOS.
    pub fn mul(&self, a: &[u64], b: &[u64]) -> Vec<u64> {
        let k = self.n.len();
        let mut t = vec![0u64; k + 2];
        for &bi in b.iter().take(k) {
            let bi = bi as u128;
            let mut carry: u128 = 0;
            for j in 0..k {
                let s = t[j] as u128 + a[j] as u128 * bi + carry;
                t[j] = s as u64;
                carry = s >> 64;
            }
            let s = t[k] as u128 + carry;
            t[k] = s as u64;
            t[k + 1] = (s >> 64) as u64;
            let m = (t[0].wrapping_mul(self.n0inv)) as u128;
            let s = t[0] as u128 + m * self.n[0] as u128;
            let mut carry = s >> 64;
            for j in 1..k {
                let s = t[j] as u128 + m * self.n[j] as u128 + carry;
                t[j - 1] = s as u64;
                carry = s >> 64;
            }
            let s = t[k] as u128 + carry;
            t[k - 1] = s as u64;
            t[k] = t[k + 1] + (s >> 64) as u64;
            t[k + 1] = 0;
        }
        let mut out = t[..k].to_vec();
        // one conditional subtraction
        let ge = t[k] != 0 || {
            let mut ge = true;
            for j in (0..k).rev() {
                if out[j] != self.n[j] {
                    ge = out[j] > self.n[j];
                    break;
                }
            }
            ge
        };
        if ge {
            let mut borrow: u128 = 0;
            for (o, &nj) in out.iter_mut().zip(&self.n) {
                let s = *o as u128 + (1u128 << 64) - nj as u128 - borrow;
                *o = s as u64;
                borrow = 1 - (s >> 64);
            }
        }
        out
    }

    /// `x` into Montgomery form: `x·R mod n`.
    pub fn to_mont(&self, x: &BigUint) -> Vec<u64> {
        let k = self.n.len();
        let xr = limbs(&(x % &self.modulus), k);
        self.mul(&xr, &self.r2)
    }

    /// Out of Montgomery form.
    pub fn from_mont(&self, a: &[u64]) -> BigUint {
        let k = self.n.len();
        let mut one = vec![0u64; k];
        one[0] = 1;
        from_limbs(&self.mul(a, &one))
    }

    /// **`base^exp mod n`**, square-and-multiply in Montgomery form with a 4-bit window.
    pub fn powmod(&self, base: &BigUint, exp: &BigUint) -> BigUint {
        let k = self.n.len();
        let b = self.to_mont(base);
        // table of b^0..b^15
        let mut table: Vec<Vec<u64>> = Vec::with_capacity(16);
        let mut one = vec![0u64; k];
        one[0] = 1;
        table.push(self.mul(&one, &self.r2)); // R mod n: the Montgomery one
        for i in 1..16 {
            let t = self.mul(&table[i - 1], &b);
            table.push(t);
        }
        let bits = exp.bits();
        if bits == 0 {
            return BigUint::one() % &self.modulus;
        }
        let mut acc = table[0].clone();
        let mut i = bits.div_ceil(4) * 4;
        let mut started = false;
        while i >= 4 {
            i -= 4;
            let mut w = 0usize;
            for j in 0..4 {
                if exp.bit(i + j) {
                    w |= 1 << j;
                }
            }
            if started {
                for _ in 0..4 {
                    acc = self.mul(&acc, &acc);
                }
            }
            if w != 0 {
                acc = if started { self.mul(&acc, &table[w]) } else { table[w].clone() };
                started = true;
            } else if started {
                // nothing to multiply
            }
        }
        self.from_mont(&acc)
    }
}

/// `base^exp mod n` through Montgomery form when `n` is odd, `num-bigint`'s modpow otherwise.
pub fn powmod(base: &BigUint, exp: &BigUint, n: &BigUint) -> BigUint {
    match Mont::new(n) {
        Some(m) if n.bits() >= 256 => m.powmod(base, exp),
        _ => base.modpow(exp, n),
    }
}
