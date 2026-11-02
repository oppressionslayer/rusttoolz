use num_bigint::BigUint;
use rusttoolz::numtoolz::*;

#[test]
fn the_mersenne_facts_and_the_cycle8_corner() {
    assert_eq!(mersenne_primes(5).iter().map(|m| m.to_string()).collect::<Vec<_>>(), ["3", "7", "31", "127", "8191"]);
    assert_eq!(mersenne_via_cycle8(5).iter().map(|(_, v)| v.to_string()).collect::<Vec<_>>(), ["3", "7", "31", "127", "8191"]);
    assert_eq!(mersenne_via_cycle8(12).iter().map(|(_, v)| v.clone()).collect::<Vec<_>>(), mersenne_primes(12));
    assert_eq!(perfect_numbers(4).iter().map(|(_, n)| n.to_string()).collect::<Vec<_>>(), ["6", "28", "496", "8128"]);
    // every listed exponent is prime and, to 127, Lucas–Lehmer says so
    for &p in MERSENNE_EXPONENTS.iter() {
        assert!(rusttoolz::is_prime_u64(p));
        if p <= 127 {
            assert!(rusttoolz::prove::lucas_lehmer(p as u32));
        }
    }
    assert_eq!(MERSENNE_EXPONENTS.len(), 52);
    assert_eq!(cycle8(32, 8).len(), 1024);
    assert_eq!(cycle8(32, 8)[1023], 8184);
    assert_eq!(digital_root(&BigUint::from(8128u32)), 1);
    assert_eq!(digital_root(&BigUint::from(28u32)), 1, "every even perfect number past 6 has digital root 1");
}

#[test]
fn the_rt_sieve_and_its_pure_prime_cap() {
    assert_eq!(extend_rt(5), vec![4, 6, 10, 14, 22]);
    assert_eq!(extend_rt(31), RT.to_vec());
    let s = self_sieve(&RT, 32, false);
    assert_eq!(s[0], (4, vec![4, 8, 12, 16, 20, 24, 28]));
    assert_eq!(s[1], (6, vec![6, 12, 18, 24, 30]));
    assert_eq!(rt_survivors(&RT, 64), vec![2]);
    assert_eq!(rt_survivors(&RT, 280), vec![2, 262, 274, 278]);
    assert_eq!(pure_prime_cap(&RT), 34322);
    assert_eq!(pure_prime_cap(&extend_rt(100)), 598418);
    // below the cap every survivor past 2 is twice a prime; the first composite half is at the cap
    for e in rt_survivors(&RT, 34322).into_iter().skip(1) {
        assert!(rusttoolz::is_prime_u64(e / 2), "{e}");
    }
    assert!(!rusttoolz::is_prime_u64(34322 / 2), "17161 = 131²");
    let (primes, spf) = linear_sieve(20);
    assert_eq!(primes, vec![2, 3, 5, 7, 11, 13, 17, 19]);
    assert_eq!(spf[12], 2);
    assert_eq!(spf[15], 3);
    let (p2, _) = linear_sieve(100_000);
    assert_eq!(p2, rusttoolz::sieve(100_000));
    assert_eq!(primes_to_deltas(&[2, 3, 5, 7, 11]), vec![1, 6, 2, 12]);
    assert_eq!(deltas_to_primes(&[1, 6, 2, 12], 2), vec![2, 3, 5, 7, 11]);
}
