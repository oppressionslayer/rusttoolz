use rusttoolz::lanes::*;

#[test]
fn the_verdicts_the_docs_print() {
    let v = verdict(91, &LANES, 8);
    assert!(!v.prime && v.how == "7 divides it" && v.cost == Cost::Free);
    let v = verdict(323, &LANES, 8);
    assert!(!v.prime && v.how == "near-square walk: 17x19" && v.cost == Cost::Free);
    let v = verdict(1717, &LANES, 8);
    assert!(!v.prime && v.how == "fold-wheel k=16: 17x101" && v.cost == Cost::Free);
    let v = verdict(4999, &LANES, 8);
    assert!(v.prime && v.how.starts_with("certified prime: below the fold-wheel horizon 11441") && v.cost == Cost::Free);
    let v = verdict(11441, &LANES, 8);
    assert!(!v.prime && v.how == "rho 17×673" && v.cost == Cost::Cheap);
    let v = verdict(11443, &LANES, 8);
    assert!(v.prime && v.how == "confirmed by test" && v.cost == Cost::Test);
    let v = verdict(97, &LANES, 8);
    assert!(v.prime && v.how == "survived every lane below the horizon 289");
    assert_eq!(horizon(&LANES), 289);
    assert_eq!(horizon(&[2, 3, 5, 7]), 121);
    assert_eq!(fold_horizon(&LANES, 8), 11441, "17 × 673");
    assert_eq!(fold_sharp(1717, 6), Some((17, 101, 16)));
    assert_eq!(fold_sharp(2533, 6), Some((17, 149, 32)));
    assert_eq!(fermat_walk(323, 8), Some((17, 19)));
    assert_eq!(rho_walk(1717, 20_000).map(|(p, q)| p.min(q)), Some(17));
    assert!(is_square(49) && !is_square(50));
    let r = lane_read(1717);
    assert_eq!((r.lane, r.mod4, r.mod6, r.mod8), ("6k+1", 1, 1, 5));
    // the verdict never lies: against the exact test on every number to 20,000
    for x in 2..20_000u64 {
        assert_eq!(verdict(x, &LANES, 8).prime, rusttoolz::is_prime_u64(x), "{x}");
    }
}

#[test]
fn the_dashboard_matches_the_cookbook() {
    let d = dashboard(2, 10_000, &LANES);
    assert_eq!((d.wheel, d.below_horizon, d.walk, d.rho, d.test), (8076, 61, 693, 0, 1168));
    assert!((d.deterministic_pct() - 88.3).abs() < 0.1);
}
