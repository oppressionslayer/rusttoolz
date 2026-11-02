use rusttoolz::xortri::*;

#[test]
fn the_session_triangle() {
    let a = [2u64, 6, 8, 11, 12, 5, 4];
    assert_eq!(triangle(&a), vec![vec![4, 14, 3, 7, 9, 1], vec![10, 13, 4, 14, 8], vec![7, 9, 10, 6], vec![14, 3, 12], vec![13, 15], vec![2]]);
    assert_eq!(square(&a).iter().map(|r| r[5]).collect::<Vec<_>>(), vec![1, 8, 6, 12, 15, 2]);
    assert_eq!(apex(&a), 2);
    assert_eq!(row_at(&a, 3), vec![7, 9, 10, 6]);
    for (k, r) in triangle(&a).iter().enumerate() {
        assert_eq!(&row_at(&a, k + 1), r);
    }
    assert_eq!(left_edge(&a), vec![2, 4, 10, 7, 14, 13, 2]);
    assert_eq!(right_edge(&a), vec![4, 1, 8, 6, 12, 15, 2]);
    assert_eq!(from_left_edge(&left_edge(&a)), a.to_vec());
    assert_eq!(from_right_edge(&right_edge(&a)), a.to_vec());
}
