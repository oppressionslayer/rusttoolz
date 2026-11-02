use rusttoolz::net::{prime_count, Net};

#[test]
fn the_net_decodes_to_the_sieve_and_proves_by_elimination() {
    let net = Net::build(100_000);
    let primes = rusttoolz::sieve(100_000);
    assert_eq!(net.primes().collect::<Vec<_>>(), primes);
    assert_eq!(net.count, 9592);
    assert_eq!(net.largest, 99991);
    assert_eq!(net.reach, 99991u64 * 99991);
    assert_eq!(net.nth(1), Some(2));
    assert_eq!(net.nth(100), Some(541));
    for k in [1u64, 2, 3, 1023, 1024, 1025, 1026, 2049, 9591, 9592] {
        assert_eq!(net.nth(k), Some(primes[k as usize - 1]), "nth({k}) across the marks");
    }
    assert_eq!(net.nth(9593), None);
    for x in [2u64, 3, 4, 5, 8161, 8162, 8167, 8168, 50_000, 99_991, 100_000] {
        assert_eq!(net.count_upto(x), primes.iter().filter(|&&p| p <= x).count() as u64, "count_upto({x})");
    }
    assert_eq!(net.first(5), vec![2, 3, 5, 7, 11]);
    // the largest prime at or under the reach, by the u64 test, then by elimination
    let mut p = net.reach;
    while !rusttoolz::is_prime_u64(p) {
        p -= 1;
    }
    assert_eq!(net.is_prime(p), Ok(true));
    assert_eq!(net.factor(net.reach), Ok(Some(99991)), "the reach itself is largest squared");
    assert_eq!(net.factorize(360), Ok(vec![(2, 3), (3, 2), (5, 1)]));
    assert_eq!(net.factorize(1_009_732_533).unwrap().iter().map(|&(p, e)| p.pow(e)).product::<u64>(), 1_009_732_533);
    assert!(net.is_prime(net.reach + 1).is_err());
    for n in 2..5000u64 {
        assert_eq!(net.is_prime(n).unwrap(), rusttoolz::is_prime_u64(n), "{n}");
    }
}

#[test]
fn windows_counts_and_twins() {
    let net = Net::build(100_000);
    let w = net.primes_between(1_000_000_000, 1_000_000_200).unwrap();
    assert_eq!(w, vec![1_000_000_007, 1_000_000_009, 1_000_000_021, 1_000_000_033, 1_000_000_087, 1_000_000_093, 1_000_000_097, 1_000_000_103, 1_000_000_123, 1_000_000_181]);
    assert_eq!(net.twins(0, 100).unwrap(), vec![(3, 5), (5, 7), (11, 13), (17, 19), (29, 31), (41, 43), (59, 61), (71, 73)]);
    assert_eq!(net.pi(1000), 168);
    assert_eq!(net.pi(100_000), 9592);
    assert_eq!(prime_count(1_000_000), 78498);
    assert_eq!(prime_count(10_000_000), 664579);
    assert_eq!(net.pi(10_000_000), 664579, "past the net the count is exact too");
}

#[test]
fn the_net_round_trips_its_file_and_reads_the_python_one() {
    let net = Net::build(50_000);
    let path = std::env::temp_dir().join(format!("rusttoolz-net-{}.hg", std::process::id()));
    net.save(&path).unwrap();
    let back = Net::load(&path).unwrap();
    assert_eq!(back.halfgaps, net.halfgaps);
    assert_eq!((back.largest, back.count, back.reach), (net.largest, net.count, net.reach));
    let _ = std::fs::remove_file(path);
    // the Python package's 15 MB net, when it is beside this crate
    let gaps = std::path::Path::new("../sitoolz-lib/src/sitoolz/primebase/data/base_gaps.npy");
    let meta = std::path::Path::new("../sitoolz-lib/src/sitoolz/primebase/data/meta.json");
    if gaps.exists() && meta.exists() {
        let big = Net::from_npy(gaps, meta).unwrap();
        assert_eq!(big.largest, 268_499_993);
        assert_eq!(big.count, 14_634_183);
        assert_eq!(big.reach, 72_092_246_241_000_049);
        assert_eq!(big.nth(1_000_000), Some(15_485_863));
        assert_eq!(big.is_prime(9_999_999_999_999_917), Ok(true));
        assert_eq!(big.factor(72_092_246_240_999_997), Ok(Some(3)));
        let w = big.primes_between(1_000_000_000_000, 1_000_000_000_100).unwrap();
        assert_eq!(w, vec![1_000_000_000_039, 1_000_000_000_061, 1_000_000_000_063, 1_000_000_000_091]);
    }
}
