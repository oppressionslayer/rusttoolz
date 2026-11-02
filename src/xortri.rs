//! **The XOR difference triangle.** XOR each neighbour pair, repeat down to one number: Pascal's
//! triangle with XOR, so row `k` is `XOR of arr[j + m]` over the submasks `m` of `k` (Lucas), the
//! gasket read as arithmetic. Any row comes straight from the input in `popcount(k)` shift-XORs,
//! and either edge rebuilds the whole triangle.

/// The rows under `arr`, down to one number.
pub fn triangle(arr: &[u64]) -> Vec<Vec<u64>> {
    let mut rows = Vec::new();
    let mut a = arr.to_vec();
    while a.len() > 1 {
        a = a.windows(2).map(|w| w[0] ^ w[1]).collect();
        rows.push(a.clone());
    }
    rows
}

/// Row `k` (row 0 is `arr`) in `popcount(k)` shift-XORs.
pub fn row_at(arr: &[u64], k: usize) -> Vec<u64> {
    assert!(k < arr.len(), "row {k} of a triangle with {} rows", arr.len());
    let mut a = arr.to_vec();
    let mut b = 0;
    let mut k = k;
    while k > 0 {
        if k & 1 == 1 {
            let s = 1usize << b;
            a = (0..a.len() - s).map(|i| a[i] ^ a[i + s]).collect();
        }
        k >>= 1;
        b += 1;
    }
    a
}

/// The bottom of the triangle: XOR of `arr[m]` over the submasks `m` of `n − 1`.
pub fn apex(arr: &[u64]) -> u64 {
    row_at(arr, arr.len() - 1)[0]
}

/// The rows packed into the upper triangle of an `(n−1)×(n−1)` array, row `k` from column `k`.
pub fn square(arr: &[u64]) -> Vec<Vec<u64>> {
    let rows = triangle(arr);
    let m = rows.len();
    let mut out = vec![vec![0u64; m]; m];
    for (k, r) in rows.iter().enumerate() {
        out[k][k..].copy_from_slice(r);
    }
    out
}

/// The first entry of every row, `arr[0]` first.
pub fn left_edge(arr: &[u64]) -> Vec<u64> {
    std::iter::once(arr[0]).chain(triangle(arr).iter().map(|r| r[0])).collect()
}

/// The last entry of every row, `arr[n−1]` first.
pub fn right_edge(arr: &[u64]) -> Vec<u64> {
    std::iter::once(arr[arr.len() - 1]).chain(triangle(arr).iter().map(|r| r[r.len() - 1])).collect()
}

/// The input back from its left edge: `row_k[j+1] = row_k[j] ^ row_{k+1}[j]`, built from the apex.
pub fn from_left_edge(edge: &[u64]) -> Vec<u64> {
    let mut row = vec![edge[edge.len() - 1]];
    for k in (0..edge.len() - 1).rev() {
        let mut next = Vec::with_capacity(row.len() + 1);
        next.push(edge[k]);
        let mut acc = edge[k];
        for &v in &row {
            acc ^= v;
            next.push(acc);
        }
        row = next;
    }
    row
}

/// The input back from its right edge, by mirror.
pub fn from_right_edge(edge: &[u64]) -> Vec<u64> {
    let mut v = from_left_edge(edge);
    v.reverse();
    v
}
