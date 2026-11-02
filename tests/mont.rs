use num_bigint::BigUint;
use num_traits::One;
use rusttoolz::mont::{powmod, Mont};

#[test]
fn montgomery_agrees_with_num_bigint_at_every_size() {
    let mut seed = 0x9E3779B97F4A7C15u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for bits in [64u64, 65, 127, 128, 200, 256, 511, 512, 1000, 2048] {
        for _ in 0..3 {
            let mut n = BigUint::from(next()) << (bits.saturating_sub(64));
            n += BigUint::from(next());
            n |= BigUint::one();
            let n = (&n % (BigUint::one() << bits)) | (BigUint::one() << (bits - 1)) | BigUint::one();
            let m = Mont::new(&n).unwrap();
            for _ in 0..3 {
                let b = BigUint::from(next()) % &n;
                let e = BigUint::from(next());
                assert_eq!(m.powmod(&b, &e), b.modpow(&e, &n), "{bits} bits");
                assert_eq!(powmod(&b, &e, &n), b.modpow(&e, &n));
                // a product out of Montgomery form is the plain product
                let a2 = m.to_mont(&b);
                let c = BigUint::from(next()) % &n;
                let c2 = m.to_mont(&c);
                assert_eq!(m.from_mont(&m.mul(&a2, &c2)), (&b * &c) % &n);
            }
            assert_eq!(m.powmod(&BigUint::from(2u32), &BigUint::from(0u32)), BigUint::one());
        }
    }
    assert!(Mont::new(&BigUint::from(10u32)).is_none(), "even modulus refused");
}
