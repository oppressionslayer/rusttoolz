//! **One modular exponentiation, two engines.** Every `base^exp mod n` in the ladder and the
//! forge comes through here. By default it is `num-bigint`'s own modpow, pure Rust, which is
//! what compiles to wasm. With the `gmp` feature it is GMP's `powm` through the `rug` crate,
//! five to ten times faster past a few thousand bits, for native builds that want the speed.
//! The conversion each way is a copy of the limbs, nothing next to the exponentiation.

use num_bigint::BigUint;

#[cfg(not(feature = "gmp"))]
pub fn modpow(base: &BigUint, exp: &BigUint, n: &BigUint) -> BigUint {
    base.modpow(exp, n)
}

#[cfg(feature = "gmp")]
pub fn modpow(base: &BigUint, exp: &BigUint, n: &BigUint) -> BigUint {
    use rug::integer::Order;
    use rug::Integer;
    let to = |b: &BigUint| Integer::from_digits(&b.to_u64_digits(), Order::Lsf);
    let r = to(base).pow_mod(&to(exp), &to(n)).expect("a nonnegative exponent always has a power");
    BigUint::from_slice(&r.to_digits::<u32>(Order::Lsf))
}

/// Which engine this build uses.
pub const ENGINE: &str = if cfg!(feature = "gmp") { "gmp" } else { "num-bigint" };
