use rusttoolz::buildsystem::*;

#[test]
fn the_table_rebuilds_from_its_own_row_zero() {
    assert_eq!(xor_prefix(&[0, 1, 2, 4, 6]), vec![0, 0, 1, 3, 7, 1]);
    let idx = structured(128);
    assert_eq!(idx, vec![0, 1, 2, 4, 6, 8, 12, 16, 18, 24, 30, 32, 36, 42, 48, 54, 60, 64, 66, 72, 78, 84, 90, 96, 102, 108, 114, 120, 124, 126]);
    let t = buildsystem(&idx);
    assert_eq!(t.len(), idx.len() + 1);
    assert!(is_buildsystem(&t));
    for i in 0..t.len() {
        for j in 0..t.len() {
            assert_eq!(t[i][j], t[j][i]);
            assert_eq!(t[i][j], t[0][i] ^ t[0][j]);
        }
    }
    assert_eq!(recover_index(&t[0]), idx);
    assert_eq!(structured(32), vec![0, 1, 2, 4, 6, 8, 12, 16, 18, 24, 28, 30]);
    assert_eq!(powers_of_two(2, 128), vec![2, 4, 8, 16, 32, 64, 128]);
    assert_eq!(&multiples_of(6, 0, 128)[..5], &[0, 6, 12, 18, 24]);
    let mut broken = t.clone();
    broken[2][3] ^= 1;
    assert!(!is_buildsystem(&broken));
}
