//! **The XOR build system.** `xor_prefix(index)` is the prefix-XOR of an index with a leading
//! zero, the anchor row `a`; `buildsystem(index)` is the table `T[i][j] = a_i ^ a_j`, symmetric,
//! zero on the diagonal, and rebuilt entirely from its own row 0, so the storage is the index
//! alone; `recover_index` reads the index back. `structured_index(hi)` is the index that is
//! itself a formula: the powers of two and the multiples of six below `hi`, plus the end marker.

/// `[0, idx[0], idx[0]^idx[1], ...]`: one longer than the index.
pub fn xor_prefix(index: &[u64]) -> Vec<u64> {
    let mut out = Vec::with_capacity(index.len() + 1);
    let mut acc = 0u64;
    out.push(0);
    for &v in index {
        acc ^= v;
        out.push(acc);
    }
    out
}

/// `T[i][j] = a_i ^ a_j` with `a = xor_prefix(index)`.
pub fn buildsystem(index: &[u64]) -> Vec<Vec<u64>> {
    let a = xor_prefix(index);
    a.iter().map(|&x| a.iter().map(|&y| x ^ y).collect()).collect()
}

/// The index back from the anchor row, or from a table's row 0: `idx[k] = a[k] ^ a[k+1]`.
pub fn recover_index(row0: &[u64]) -> Vec<u64> {
    row0.windows(2).map(|w| w[0] ^ w[1]).collect()
}

/// Whether a table is a build system: every row is row 0 XOR a constant, zero on the diagonal.
pub fn is_buildsystem(t: &[Vec<u64>]) -> bool {
    if t.is_empty() {
        return false;
    }
    let a = &t[0];
    t.iter().enumerate().all(|(i, row)| row.len() == a.len() && row[i] == 0 && row.iter().zip(a).all(|(&v, &x)| v == x ^ a[i]))
}

/// The powers of two in `[lo, hi]`.
pub fn powers_of_two(lo: u64, hi: u64) -> Vec<u64> {
    (0..64).map(|k| 1u64 << k).filter(|&p| lo <= p && p <= hi).collect()
}

/// The multiples of `step` in `[lo, hi)`.
pub fn multiples_of(step: u64, lo: u64, hi: u64) -> Vec<u64> {
    let first = lo + (step - lo % step) % step;
    (first..hi).step_by(step as usize).collect()
}

/// **The structured index**: powers of two from `lo` and multiples of `step`, both strictly
/// below `hi`, plus the end marker `hi − end_offset`. Sorted, deduplicated. `structured_index(128)`
/// is the hand-built thirty-wide index of the sessions.
pub fn structured_index(hi: u64, step: u64, lo: u64, end_offset: u64) -> Vec<u64> {
    let mut v = powers_of_two(lo, hi - 1);
    v.extend(multiples_of(step, 0, hi));
    v.push(hi - end_offset);
    v.sort_unstable();
    v.dedup();
    v
}

/// The session's default: `structured_index(hi, 6, 1, 4)`.
pub fn structured(hi: u64) -> Vec<u64> {
    structured_index(hi, 6, 1, 4)
}
