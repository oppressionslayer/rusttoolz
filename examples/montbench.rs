//! Montgomery modpow against num-bigint's, at the sizes the ladder works at.
use num_bigint::BigUint;
use num_traits::One;
use rusttoolz::mont::Mont;

fn main() {
    for bits in [512u64, 1024, 2048, 4096, 8192] {
        let n = (BigUint::one() << bits) - BigUint::from(1234567u32); // odd
        let n = &n | BigUint::one();
        let e = &n - BigUint::one();
        let b = BigUint::from(2u32);
        let t = std::time::Instant::now();
        let a = b.modpow(&e, &n);
        let t1 = t.elapsed();
        let t = std::time::Instant::now();
        let m = Mont::new(&n).unwrap();
        let c = m.powmod(&b, &e);
        let t2 = t.elapsed();
        println!("{bits:>5} bits  num-bigint {t1:>10.2?}  montgomery {t2:>10.2?}  agree {}  speedup {:.1}x", a == c, t1.as_secs_f64() / t2.as_secs_f64());
    }
}
