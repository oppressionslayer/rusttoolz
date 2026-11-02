use rusttoolz::twin::{bad_residues, estimate, factor_small, hunt, prove_pair, sieve_k};

#[test]
fn the_82_digit_pair_is_proven_both_ways() {
    let t = prove_pair(19902, 256);
    assert_eq!(t.digits, 82);
    assert_eq!(factor_small(19902), vec![2, 3, 31, 107]);
    assert!(t.upper_proven && t.lower_probable && t.lower_proven);
    assert_eq!(&t.upper - &t.lower, num_bigint::BigUint::from(2u32));
    assert_eq!(t.upper.to_string(), "2304494160001066921319909743642906742095779234815578505513284836925487106094006273");
}

#[test]
fn the_33_digit_pair_from_the_how_to() {
    let t = prove_pair(7737, 96);
    assert_eq!(t.digits, 33);
    assert!(t.upper_proven && t.lower_proven);
    assert_eq!(t.lower.to_string(), "612988293372863179961249543749631");
    // a k that is not a twin: one side fails
    let u = prove_pair(7736, 96);
    assert!(!(u.upper_proven && u.lower_proven));
}

#[test]
fn the_sieve_keeps_every_twin_and_the_hunt_finds_them() {
    let n = 32;
    let bad = bad_residues(n, 1000);
    let alive = sieve_k(1, 20_000, &bad);
    // every k the sieve drops has a small factor on one side
    let base = 1u128 << n;
    for k in 1..20_000u64 {
        let dropped = !alive.contains(&k);
        let killed = bad.iter().any(|&(p, _, _)| ((k as u128 * base + 1) % p as u128 == 0) || ((k as u128 * base - 1) % p as u128 == 0));
        assert_eq!(dropped, killed, "k = {k}");
    }
    let hits = hunt(n, 1, 20_000, 1000);
    assert!(hits.iter().all(|t| t.upper_proven));
    let twins: Vec<u64> = hits.iter().filter(|t| t.lower_proven).map(|t| t.k).collect();
    assert!(!twins.is_empty(), "some k·2^32 ± 1 twin below 20,000");
    // cross-check the twins against a direct test
    for t in &hits {
        let up = u128::try_from(&t.upper).unwrap();
        assert!(rusttoolz::is_prime_u64(up as u64) || up > u64::MAX as u128);
    }
    let (digits, per_plus, per_twin) = estimate(256);
    assert_eq!(digits, 78);
    assert!(per_plus > 80.0 && per_twin > per_plus);
}

#[test]
fn the_hunt_streams_packets_and_resumes_from_its_checkpoint() {
    use rusttoolz::twin::{hunt_stream, Hunt};
    let dir = std::env::temp_dir().join(format!("rusttoolz-forge-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (out, cp) = (dir.join("twins_64.jsonl"), dir.join("checkpoint.json"));
    let mut h = Hunt::new(64, 1, 20_000);
    h.span = 10_000;
    h.sieve_bound = 1000;
    h.out = Some(out.clone());
    h.checkpoint = Some(cp.clone());
    let first = hunt_stream(&h).unwrap();
    assert_eq!(std::fs::read_to_string(&cp).unwrap(), "{\"next_k\": 20000}");
    let lines: Vec<String> = std::fs::read_to_string(&out).unwrap().lines().map(String::from).collect();
    assert!(lines.iter().all(|l| l.starts_with("{\"k\": ") && l.contains("\"upper_proven_pocklington\": true")));
    assert_eq!(lines.iter().filter(|l| l.contains("\"lower_proven_lucas\": true")).count(), first.len());
    // resume: the checkpoint says 20000, so the next call starts there and finds new k only
    h.k_stop = 30_000;
    let second = hunt_stream(&h).unwrap();
    assert!(second.iter().all(|t| t.k >= 20_000));
    assert_eq!(std::fs::read_to_string(&cp).unwrap(), "{\"next_k\": 30000}");
    let _ = std::fs::remove_dir_all(dir);
}
