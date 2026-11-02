use rusttoolz::edgelaw::*;

#[test]
fn the_edge_is_a_lookup_and_the_lookup_matches_the_edge() {
    assert_eq!((0..16).map(|g| edge(g).unwrap()).collect::<Vec<_>>(), vec![1, 3, 1, 4, 1, 3, 1, 6, 1, 3, 1, 4, 1, 3, 1, 7]);
    for g in [255u64, 767, 1279, 1791] {
        assert_eq!(edge(g), Some(24), "v2(g+1) = 8 for every one");
    }
    assert_eq!(edge(2047), Some(29));
    assert_eq!(edge(16_777_215), Some(60));
    assert_eq!(level(1791), 8);
    assert_eq!(first(8), 255);
    assert_eq!(same_as(1791, 4), vec![255, 767, 1279, 1791]);
    assert_eq!(VERIFIED_TO, 281_474_976_710_655);
    assert!(covers(VERIFIED_TO) && !covers((1u64 << 49) - 1));
    assert_eq!(value(7), Some(63), "rung 3 runs six ones");
    // measured against the law for every generation to 4095
    assert!(verify(4095, 512).is_empty());
    assert_eq!(measure(1791, 512), 24);
}
