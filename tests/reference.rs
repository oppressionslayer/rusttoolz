//! **The studios' reference cases.** `spindexin-recovered/tests/triangle_reference.py` writes 128
//! cases straight from `sitoolz.xortri` for the JavaScript port; the Rust port is held to the
//! same file, so Python, JavaScript and Rust agree on every row, edge and square.

use rusttoolz::xortri::*;

fn nums(v: &serde_json::Value) -> Vec<u64> {
    v.as_array().unwrap().iter().map(|x| x.as_str().unwrap().parse().unwrap()).collect()
}

#[test]
fn the_triangle_matches_the_python_reference_cases() {
    let Ok(text) = std::fs::read_to_string("tests/data/triangle_reference.json") else { return };
    let cases: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap();
    assert_eq!(cases.len(), 128);
    for (i, c) in cases.iter().enumerate() {
        let input = nums(&c["input"]);
        let rows: Vec<Vec<u64>> = c["rows"].as_array().unwrap().iter().map(nums).collect();
        assert_eq!(rows[0], input, "case {i}: row 0 is the input");
        assert_eq!(triangle(&input), rows[1..].to_vec(), "case {i}: the rows");
        for (k, r) in rows.iter().enumerate() {
            assert_eq!(&row_at(&input, k), r, "case {i}: row {k} by the closed form");
        }
        assert_eq!(left_edge(&input), nums(&c["left"]), "case {i}: left edge");
        assert_eq!(right_edge(&input), nums(&c["right"]), "case {i}: right edge");
        assert_eq!(from_left_edge(&nums(&c["left"])), input, "case {i}: back from the left edge");
        assert_eq!(from_right_edge(&nums(&c["right"])), input, "case {i}: back from the right edge");
        if input.len() > 1 {
            let sq: Vec<Vec<u64>> = c["square"].as_array().unwrap().iter().map(nums).collect();
            assert_eq!(square(&input), sq, "case {i}: the square");
            assert_eq!(apex(&input), rows.last().unwrap()[0], "case {i}: the apex");
        }
    }
}
