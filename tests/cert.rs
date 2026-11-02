use rusttoolz::cert::{from_json, sniff, to_json, Convention};
use rusttoolz::prove::{build_up, mersenne_seed, verify_certificate};

#[test]
fn a_rust_chain_round_trips_through_both_text_conventions() {
    let chain = build_up(mersenne_seed(31).unwrap(), 300);
    for conv in [Convention::Numtools, Convention::Slip] {
        let text = to_json(&chain, conv);
        let back = from_json(&text, conv).unwrap();
        assert_eq!(back, chain, "{conv:?}");
        assert!(verify_certificate(&back, None));
        assert_eq!(sniff(&text), Some(conv));
    }
    // a tampered slip is caught at parse time: N must be 2kq + 1 off the rung below
    let text = to_json(&chain, Convention::Slip).replacen("\"", "\"9", 3);
    assert!(from_json(&text, Convention::Slip).is_err() || !verify_certificate(&from_json(&text, Convention::Slip).unwrap(), None));
}

#[test]
fn the_python_packages_4096_bit_chain_verifies_here() {
    // sitoolz ships a 4096-bit prime with its chain in numtools' convention, as a Python repr
    let path = std::path::Path::new("../sitoolz-lib/src/sitoolz/data/prime_4096_chain.json");
    if !path.exists() {
        return;
    }
    let text = std::fs::read_to_string(path).unwrap();
    let start = text.find("\"chain\": ").unwrap() + 9;
    let end = text.rfind("]]").unwrap() + 2;
    let chain_text = &text[start..end];
    assert_eq!(sniff(chain_text), Some(Convention::Numtools));
    let chain = from_json(chain_text, Convention::Numtools).unwrap();
    assert_eq!(chain.len(), 9);
    assert_eq!(chain.last().unwrap().n.bits(), 4096);
    // the seed 112543 is a prime a small net proves
    let net = rusttoolz::net::Net::build(1000);
    assert!(verify_certificate(&chain, Some(&net)), "every rung of the Python chain re-proves in Rust");
}
