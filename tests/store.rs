//! The prime store: facts written once, read back to skip and to resume.
use rusttoolz::prove::{build_up, mersenne_seed, verify_certificate};
use rusttoolz::store::{Record, Store};

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("rusttoolz-store-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_file(&dir);
    dir
}

#[test]
fn the_known_mersennes_are_facts_and_are_seeded_once() {
    let path = scratch("mersennes");
    let mut st = Store::open(&path).unwrap();
    assert_eq!(st.seed_known_mersennes().unwrap(), 52);
    assert_eq!(st.seed_known_mersennes().unwrap(), 0);
    assert_eq!(st.mersenne(127), Some(true));
    assert_eq!(st.mersenne(11), None, "no verdict until one is recorded");
    assert!(st.record(Record::mersenne(11, false, 0.0)).unwrap());
    assert!(!st.record(Record::mersenne(11, false, 0.0)).unwrap());
    assert_eq!(st.mersenne(11), Some(false));
    // the file reads back to the same store
    let again = Store::open(&path).unwrap();
    assert_eq!(again.len(), 53);
    assert_eq!(again.mersennes().len(), 53);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_stored_ladder_resumes_from_its_top_rung_and_still_verifies() {
    let path = scratch("ladder");
    let mut st = Store::open(&path).unwrap();
    let chain = build_up(mersenne_seed(31).unwrap(), 256);
    assert_eq!(st.record_ladder("M31", &chain).unwrap(), chain.len());
    assert_eq!(st.record_ladder("M31", &chain).unwrap(), 0);
    let back = Store::open(&path).unwrap();
    let stored = back.ladder("M31");
    assert_eq!(stored, chain, "every rung, its q and its witness come back");
    assert!(verify_certificate(&stored, None));
    // climb on from the top rung: the joined chain is one certificate
    let more = build_up(stored.last().unwrap().clone(), 600);
    let mut joined = stored.clone();
    joined.extend(more.into_iter().skip(1));
    assert!(joined.last().unwrap().n.bits() >= 600);
    assert!(verify_certificate(&joined, None));
    // a rung under another base is another ladder
    assert!(back.ladder("M127").is_empty());
    assert!(back.tallest(256).unwrap().1 == chain.last().unwrap().n);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_broken_line_is_an_error_with_its_number() {
    let path = scratch("broken");
    std::fs::write(&path, "{\"kind\": \"mersenne\", \"p\": 7, \"bits\": 7, \"prime\": true}\nnot json\n").unwrap();
    let err = match Store::open(&path) {
        Ok(_) => panic!("a broken line opened"),
        Err(e) => e,
    };
    assert!(err.contains("line 2"), "{err}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_known_mersenne_seeds_and_verifies_without_lucas_lehmer() {
    use rusttoolz::prove::{known_mersenne, known_mersenne_seed};
    assert!(known_mersenne(4423) && known_mersenne(82_589_933) && !known_mersenne(11) && !known_mersenne(4421));
    let seed = known_mersenne_seed(2203).unwrap();
    let t = std::time::Instant::now();
    let chain = build_up(seed, 2400);
    assert!(verify_certificate(&chain, None), "a seed on M2203 is a fact");
    assert!(t.elapsed().as_secs() < 60);
    assert!(known_mersenne_seed(4421).is_none());
}

#[test]
fn a_recipe_rebuilds_the_climb_exactly_and_is_a_few_bytes() {
    use rusttoolz::prove::{rebuild, recipe};
    let chain = build_up(mersenne_seed(127).unwrap(), 1024);
    let offsets = recipe(&chain, 1024).unwrap();
    assert_eq!(offsets.len(), chain.len() - 1);
    let again = rebuild(mersenne_seed(127).unwrap(), 1024, &offsets).unwrap();
    assert_eq!(again, chain, "the offsets name every rung");
    assert!(verify_certificate(&again, None));
    // a ladder resumed toward a second target is not one climb, and says so instead of panicking
    let mut resumed = chain.clone();
    resumed.extend(build_up(chain.last().unwrap().clone(), 1500).into_iter().skip(1));
    assert!(recipe(&resumed, 1500).is_none());
    // an offset that misses a rung is refused, not guessed
    let mut wrong = offsets.clone();
    wrong[0] += 1;
    assert!(rebuild(mersenne_seed(127).unwrap(), 1024, &wrong).is_none() || rebuild(mersenne_seed(127).unwrap(), 1024, &wrong).unwrap() != chain);
    // and it stores as one short line
    let path = scratch("recipe");
    let mut st = Store::open(&path).unwrap();
    let witnesses: Vec<u32> = chain.iter().skip(1).map(|r| r.a).collect();
    let line = Record::recipe("M127", 1024, &offsets, &witnesses);
    assert!(line.to_json().len() < 200, "{}", line.to_json());
    assert!(st.record(line).unwrap());
    let (o, w) = Store::open(&path).unwrap().recipe_for("M127", 1024).unwrap();
    assert_eq!((o, w), (offsets, witnesses));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn gpts_python_offsets_for_m127_to_4096_are_checked_against_the_rust_start() {
    // GPT's recipe from the Python ladder: offsets 76, 544, 62, 124, 1724, 1139, witnesses all 2.
    // Whether they name the Rust rungs says whether both ladders start their search at the same
    // place; the test records the answer either way rather than assuming it.
    use rusttoolz::prove::rebuild;
    let py = [76, 544, 62, 124, 1724, 1139];
    let rust = build_up(mersenne_seed(127).unwrap(), 4096);
    let from_py = rebuild(mersenne_seed(127).unwrap(), 4096, &py);
    match from_py {
        Some(chain) => {
            assert!(verify_certificate(&chain, None));
            println!("python offsets rebuild a proven 4096-bit ladder; same rungs as the Rust climb: {}", chain == rust);
            println!("rust offsets: {:?}", rusttoolz::prove::recipe(&rust, 4096).unwrap());
            for (a, b) in chain.iter().zip(rust.iter()) {
                println!("  python rung {} bits witness {}  |  rust rung {} bits witness {}  same: {}", a.n.bits(), a.a, b.n.bits(), b.a, a == b);
            }
        }
        None => println!("python offsets do not land on Rust rungs: the two ladders start their search at different places"),
    }
}

#[test]
fn the_smallest_mersennes_climb_too() {
    use rusttoolz::prove::known_mersenne_seed;
    for p in [2u32, 3, 5, 7, 13, 17, 19, 31] {
        let chain = build_up(known_mersenne_seed(p).unwrap(), 256);
        assert!(chain.last().unwrap().n.bits() >= 256, "M{p}");
        assert!(verify_certificate(&chain, None), "M{p}");
    }
}

#[test]
fn a_known_mersenne_seed_is_stored_as_its_exponent_and_reads_back_whole() {
    use rusttoolz::prove::known_mersenne_seed;
    let path = scratch("exponent");
    let mut st = Store::open(&path).unwrap();
    let seed = known_mersenne_seed(44497).unwrap();
    let line = Record::rung("M44497", &seed);
    assert!(line.to_json().len() < 120, "{}", line.to_json());
    assert!(st.record(line).unwrap());
    assert_eq!(Store::open(&path).unwrap().ladder("M44497"), vec![seed]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_chain_seeded_on_a_stored_rung_verifies_against_the_store_and_not_without_it() {
    use rusttoolz::prove::Rung;
    use num_traits::Zero;
    let path = scratch("holds");
    let mut st = Store::open(&path).unwrap();
    let ours = build_up(mersenne_seed(31).unwrap(), 200);
    st.record_ladder("M31", &ours).unwrap();
    let top = ours.last().unwrap().n.clone();
    assert!(st.holds(&top));
    assert!(!st.holds(&(top.clone() + 2u32)));
    // another language's climb, seeded on our top rung, with that rung as its seed row
    let theirs = build_up(Rung { n: top.clone(), q: num_bigint::BigUint::zero(), a: 0 }, 400);
    assert!(!verify_certificate(&theirs, None), "its seed is not a Mersenne, so the plain verifier refuses it");
    assert!(st.verify(&theirs), "the store holds the seed, so the chain stands");
    assert!(!Store::open(scratch("none")).unwrap().verify(&theirs), "an empty store does not");
    // a witness one up may certify just as well (3 is a witness when 2 is); a shifted N cannot
    let mut bad = theirs.clone();
    bad[1].n += 2u32;
    assert!(!st.verify(&bad));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_descent_reads_the_whole_ladder_off_the_top_prime() {
    use rusttoolz::prove::descend;
    // 2032 bits from M127 is four doubling rungs; a sized last rung is what the descent cannot read
    let chain = build_up(mersenne_seed(127).unwrap(), 2032);
    let top = chain.last().unwrap().n.clone();
    let down = descend(&top, 64, 8192);
    assert_eq!(down, chain, "every rung, q and witness, from the top number alone");
    assert!(verify_certificate(&down, None));
    // a prime that is not a rung stops with a non-Mersenne bottom, and the verifier says so
    let stranger = &top + 2u32 * 3u32; // some other number
    let d = descend(&stranger, 64, 8192);
    assert!(!verify_certificate(&d, None));
}

#[test]
fn a_seed_spelled_out_and_the_same_seed_by_exponent_are_one_fact() {
    let path = scratch("twoforms");
    std::fs::write(&path, format!("{{\"kind\": \"rung\", \"base\": \"M127\", \"bits\": 127, \"n\": \"{}\", \"proof\": \"seed\", \"witness\": 0}}\n", (num_bigint::BigUint::from(1u32) << 127u32) - 1u32)).unwrap();
    let mut st = Store::open(&path).unwrap();
    let again = Record::rung("M127", &mersenne_seed(127).unwrap());
    assert!(!again.fields.contains_key("n"), "the current form names it by exponent");
    assert!(!st.record(again).unwrap(), "and it is the fact the store already holds");
    assert_eq!(st.len(), 1);
    let _ = std::fs::remove_file(&path);
}
