use num_bigint::BigUint;
use rusttoolz::gaps::{wheel_horizon, GapForge};

#[test]
fn the_forge_reads_the_distances_off_the_sieve() {
    let f = GapForge::new(1_000_000);
    assert_eq!(f.primes.len(), 78498);
    assert_eq!(&f.gaps()[..10], &[1, 2, 2, 4, 2, 4, 2, 4, 6, 2]);
    assert_eq!(&f.evens()[..5], &[2, 2, 4, 2, 4]);
    assert_eq!(f.twins().iter().take_while(|&&p| p <= 100).copied().collect::<Vec<_>>(), vec![3, 5, 11, 17, 29, 41, 59, 71]);
    assert_eq!(f.twins().len(), 8169);
    assert_eq!(f.jumping_champion(), 6);
    let c = f.gap_counts();
    assert_eq!(c[&2], 8169);
    assert_eq!(f.where_gap(6).len() as u64, c[&6]);
    assert_eq!(f.records().iter().take(4).copied().collect::<Vec<_>>(), vec![(2, 3, 1), (3, 5, 2), (7, 11, 4), (23, 29, 6)]);
    assert_eq!(f.records().last().unwrap().2, 114, "the largest gap below a million");
    assert!(f.sexy().iter().take_while(|&&p| p <= 50).copied().eq([5, 7, 11, 13, 17, 23, 31, 37, 41, 47]));
    assert_eq!(wheel_horizon(&[2, 3, 5, 7, 11, 13]), 289);
    // the session's gap-LCM number catches every gap it holds a factor of
    let m: BigUint = "1437533509890978910608944514130150588800".parse().unwrap();
    let cov = f.coverage(&m, 5);
    assert_eq!(cov.total, 78497);
    assert!(cov.catch_rate > 0.99, "{}", cov.catch_rate);
    let small = f.coverage(&BigUint::from(12u32), 3);
    assert_eq!(small.top_survivors[0].0, 10, "10 is the commonest gap 12 cannot catch");
}
