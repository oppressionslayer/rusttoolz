use num_bigint::BigUint;
use num_traits::One;
use rusttoolz::net::Net;
use rusttoolz::prove::{build_up, build_up_small, lucas_lehmer, mersenne_seed, net_seed, pocklington, verify_certificate, Rung, Verdict};

#[test]
fn lucas_lehmer_knows_the_mersenne_primes() {
    let known = [2u32, 3, 5, 7, 13, 17, 19, 31, 61, 89, 107, 127];
    for p in 2..130u32 {
        assert_eq!(lucas_lehmer(p), known.contains(&p), "p = {p}");
    }
}

#[test]
fn one_rung_proves_and_a_pseudoprime_is_refused() {
    let q = BigUint::from(2u32).pow(31) - BigUint::one(); // M31
    let n = BigUint::from(2u32) * BigUint::from(3u32) * &q + BigUint::one(); // R = 3
    match pocklington(&n, &q) {
        Verdict::Proven(_) | Verdict::Composite => {}
        v => panic!("{v:?}"),
    }
    // 341 = 11·31 beats Fermat base 2; the gate refuses it
    assert_eq!(pocklington(&BigUint::from(341u32), &BigUint::from(85u32)), Verdict::Composite);
    // q must exceed the square root
    assert_eq!(pocklington(&BigUint::from(23u32), &BigUint::from(2u32)), Verdict::Inconclusive);
}

#[test]
fn the_ladder_climbs_from_a_mersenne_seed_and_replays() {
    let chain = build_up(mersenne_seed(31).unwrap(), 512);
    assert!(chain.len() >= 5 && chain.len() <= 7, "about doubling per rung: {} rungs", chain.len() - 1);
    assert!(chain.last().unwrap().n.bits() >= 512 && chain.last().unwrap().n.bits() <= 520, "lands on the target");
    let short = build_up_small(mersenne_seed(31).unwrap(), 128);
    assert!(short.len() > 8, "smallest R climbs in many short rungs");
    assert!(verify_certificate(&short, None));
    assert!(verify_certificate(&chain, None));
    // every rung is 2·R·q + 1 off the one below
    for w in chain.windows(2) {
        let (lo, hi) = (&w[0].n, &w[1].n);
        assert_eq!(&w[1].q, lo);
        assert!(((hi - BigUint::one()) % (BigUint::from(2u32) * lo)) == BigUint::from(0u32));
    }
    // tamper with a witness and the replay fails
    let mut bad = chain.clone();
    bad[1].a = 1;
    assert!(!verify_certificate(&bad, None));
    let mut bad = chain.clone();
    bad[2].q += BigUint::one();
    assert!(!verify_certificate(&bad, None));
}

#[test]
fn a_short_last_rung_never_runs_out_of_r() {
    // targets just above a rung's size leave only a few R below the start: the walk turns upward
    for bits in [250u64, 256, 260, 300, 400] {
        let chain = build_up(mersenne_seed(31).unwrap(), bits);
        assert!(chain.last().unwrap().n.bits() >= bits, "{bits}");
        assert!(verify_certificate(&chain, None));
    }
}

#[test]
fn a_net_seed_climbs_too() {
    let net = Net::build(10_000);
    let chain = build_up(net_seed(&net, 7919).unwrap(), 200);
    assert!(verify_certificate(&chain, Some(&net)));
    assert!(!verify_certificate(&chain, None), "a net seed needs the net to replay");
    let fake = vec![Rung { n: BigUint::from(7917u32), q: BigUint::from(0u32), a: 0 }];
    assert!(!verify_certificate(&fake, Some(&net)), "7917 = 3·7·13·29 is no seed");
}
