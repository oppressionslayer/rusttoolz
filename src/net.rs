//! **The prime net.** Every prime to `largest` as `u8` half-gaps between consecutive odd primes,
//! eight times smaller than the primes themselves. A composite `n` has a factor at most `√n`, so
//! a number up to `reach = largest²` with no factor in the net is prime, by elimination.
//!
//! The net reads the Python package's own `base_gaps.npy` and `meta.json`, so the 15 MB base
//! that proves to 72 quadrillion loads unchanged; or build one to any limit here.

use std::io::{Read, Write};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Net {
    /// Half-gaps between consecutive odd primes from 3: `p[i+1] = p[i] + 2·hg[i]`.
    pub halfgaps: Vec<u8>,
    pub largest: u64,
    pub count: u64,
    /// The largest number this net proves: `largest²`.
    pub reach: u64,
    /// **Marks**: the prime at every `MARK`-th half-gap, so a walk to any prime starts from the
    /// nearest mark instead of from 3. rulerpack's checkpoint idea, inside the net.
    marks: Vec<u64>,
}

/// Half-gaps between marks.
pub const MARK: usize = 1024;

fn lay_marks(halfgaps: &[u8]) -> Vec<u64> {
    let mut marks = Vec::with_capacity(halfgaps.len() / MARK + 1);
    let mut p = 3u64;
    marks.push(p);
    for (i, &hg) in halfgaps.iter().enumerate() {
        p += 2 * hg as u64;
        if (i + 1) % MARK == 0 {
            marks.push(p);
        }
    }
    marks
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutOfReach(pub u64);

impl std::fmt::Display for OutOfReach {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} is past this net's reach", self.0)
    }
}
impl std::error::Error for OutOfReach {}

impl Net {
    /// **Build a net** holding every prime to `limit`, inclusive. A gap over 510 would not fit a
    /// half-gap byte; the first such gap is far past `2^40`, so a `u64` limit is safe.
    pub fn build(limit: u64) -> Net {
        let primes = crate::sieve(limit);
        let count = primes.len() as u64;
        let largest = *primes.last().unwrap_or(&0);
        let mut halfgaps = Vec::with_capacity(primes.len().saturating_sub(2));
        for w in primes.windows(2).skip(1) {
            let hg = (w[1] - w[0]) / 2;
            assert!(hg <= 255, "a half-gap of {hg} at {} does not fit a byte", w[0]);
            halfgaps.push(hg as u8);
        }
        let marks = lay_marks(&halfgaps);
        Net { halfgaps, largest, count, reach: largest.saturating_mul(largest), marks }
    }

    /// **The primes, in order**, decoded from the half-gaps as they go.
    pub fn primes(&self) -> impl Iterator<Item = u64> + '_ {
        let head = [2u64, 3].into_iter().take(self.count.min(2) as usize);
        let mut p = 3u64;
        head.chain(self.halfgaps.iter().map(move |&hg| {
            p += 2 * hg as u64;
            p
        }))
    }

    /// The primes up to `limit`, inclusive.
    pub fn primes_upto(&self, limit: u64) -> Vec<u64> {
        self.primes().take_while(|&p| p <= limit).collect()
    }

    /// The first `n` primes.
    pub fn first(&self, n: usize) -> Vec<u64> {
        self.primes().take(n).collect()
    }

    /// The `k`-th prime, from one: `nth(1) == 2`. A walk from the nearest mark.
    pub fn nth(&self, k: u64) -> Option<u64> {
        if k == 0 || k > self.count {
            return None;
        }
        if k <= 2 {
            return Some([2, 3][k as usize - 1]);
        }
        // prime number k (1-based) is 3 plus the first k-2 half-gaps
        let steps = (k - 2) as usize;
        let m = steps / MARK;
        let mut p = self.marks[m];
        for &hg in &self.halfgaps[m * MARK..steps] {
            p += 2 * hg as u64;
        }
        Some(p)
    }

    /// How many primes in the net are at most `x`: a binary search over the marks, then a walk.
    pub fn count_upto(&self, x: u64) -> u64 {
        if x < 2 {
            return 0;
        }
        if x < 3 {
            return 1;
        }
        // marks[m] is prime number 2 + m·MARK (1-based); find the last mark <= x
        let m = self.marks.partition_point(|&p| p <= x).saturating_sub(1);
        let mut p = self.marks[m];
        let mut n = 2 + (m * MARK) as u64;
        for &hg in &self.halfgaps[m * MARK..] {
            let q = p + 2 * hg as u64;
            if q > x {
                break;
            }
            p = q;
            n += 1;
        }
        n
    }

    /// **The smallest prime factor of `n`**, or `None` when `n` is prime, for `2 <= n <= reach`.
    pub fn factor(&self, n: u64) -> Result<Option<u64>, OutOfReach> {
        if n > self.reach {
            return Err(OutOfReach(n));
        }
        if n < 2 {
            return Ok(None);
        }
        let root = (n as f64).sqrt() as u64 + 1;
        for p in self.primes().take_while(|&p| p <= root) {
            if p * p > n {
                break;
            }
            if n % p == 0 {
                return Ok(Some(p));
            }
        }
        Ok(None)
    }

    /// **Proven prime**, for `n <= reach`: no prime in the net divides it.
    pub fn is_prime(&self, n: u64) -> Result<bool, OutOfReach> {
        if n < 2 {
            return Ok(false);
        }
        Ok(self.factor(n)?.is_none())
    }

    /// The full factorization as `(prime, exponent)` pairs, ascending, for `n <= reach`.
    pub fn factorize(&self, mut n: u64) -> Result<Vec<(u64, u32)>, OutOfReach> {
        if n > self.reach {
            return Err(OutOfReach(n));
        }
        let mut out = Vec::new();
        for p in self.primes() {
            if n == 1 || p * p > n {
                break;
            }
            if n % p == 0 {
                let mut e = 0;
                while n % p == 0 {
                    n /= p;
                    e += 1;
                }
                out.push((p, e));
            }
        }
        if n > 1 {
            out.push((n, 1));
        }
        Ok(out)
    }

    /// **Every prime in `[lo, hi)`**, by a segmented sieve seeded from the net, anywhere up to the
    /// reach. RAM is the window, not the net.
    pub fn primes_between(&self, lo: u64, hi: u64) -> Result<Vec<u64>, OutOfReach> {
        if hi > self.reach + 1 {
            return Err(OutOfReach(hi));
        }
        let lo = lo.max(2);
        if lo >= hi {
            return Ok(Vec::new());
        }
        let n = (hi - lo) as usize;
        let mut alive = vec![true; n];
        let root = ((hi - 1) as f64).sqrt() as u64 + 1;
        for p in self.primes().take_while(|&p| p <= root) {
            if p * p >= hi {
                break;
            }
            let start = (p * p).max(lo.div_ceil(p) * p);
            let mut j = start;
            while j < hi {
                alive[(j - lo) as usize] = false;
                j += p;
            }
        }
        Ok((0..n).filter(|&i| alive[i]).map(|i| lo + i as u64).collect())
    }

    /// Twin pairs `(p, p + 2)` with `lo <= p` and `p + 2 < hi`.
    pub fn twins(&self, lo: u64, hi: u64) -> Result<Vec<(u64, u64)>, OutOfReach> {
        let ps = self.primes_between(lo, (hi + 2).min(self.reach + 1))?;
        Ok(ps.windows(2).filter(|w| w[1] - w[0] == 2 && w[1] < hi).map(|w| (w[0], w[1])).collect())
    }

    /// **Exact `π(x)`**: from the net while `x` is within it, by the Lucy_Hedgehog count beyond.
    pub fn pi(&self, x: u64) -> u64 {
        if x <= self.largest {
            self.count_upto(x)
        } else {
            prime_count(x)
        }
    }

    /// **Save** as the net's own file: three little-endian `u64` (largest, count, reach) then
    /// the half-gaps.
    pub fn save(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let mut f = std::fs::File::create(path)?;
        for v in [self.largest, self.count, self.reach] {
            f.write_all(&v.to_le_bytes())?;
        }
        f.write_all(&self.halfgaps)
    }

    pub fn load(path: impl AsRef<Path>) -> std::io::Result<Net> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)?.read_to_end(&mut bytes)?;
        if bytes.len() < 24 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "too short for a net"));
        }
        let u = |i: usize| u64::from_le_bytes(bytes[i..i + 8].try_into().unwrap());
        let halfgaps = bytes[24..].to_vec();
        let marks = lay_marks(&halfgaps);
        Ok(Net { largest: u(0), count: u(8), reach: u(16), halfgaps, marks })
    }

    /// **Load the Python package's net**: `base_gaps.npy` (a NumPy `uint8` array, version 1
    /// header) and its `meta.json` with `largest`, `count` and `reach`.
    pub fn from_npy(gaps: impl AsRef<Path>, meta: impl AsRef<Path>) -> std::io::Result<Net> {
        let mut bytes = Vec::new();
        std::fs::File::open(gaps)?.read_to_end(&mut bytes)?;
        let bad = |m: &str| std::io::Error::new(std::io::ErrorKind::InvalidData, m.to_string());
        if bytes.len() < 10 || &bytes[..6] != b"\x93NUMPY" {
            return Err(bad("not a .npy file"));
        }
        let (major, hlen_at) = (bytes[6], 8usize);
        let (hlen, data_at) = if major == 1 {
            (u16::from_le_bytes([bytes[hlen_at], bytes[hlen_at + 1]]) as usize, hlen_at + 2)
        } else {
            (u32::from_le_bytes(bytes[hlen_at..hlen_at + 4].try_into().unwrap()) as usize, hlen_at + 4)
        };
        let header = std::str::from_utf8(&bytes[data_at..data_at + hlen]).map_err(|_| bad("header"))?;
        if !header.contains("|u1") {
            return Err(bad("the net must be a uint8 array"));
        }
        let text = std::fs::read_to_string(meta)?;
        let field = |k: &str| -> std::io::Result<u64> {
            let key = format!("\"{k}\"");
            let i = text.find(&key).ok_or_else(|| bad("meta field missing"))?;
            let rest = &text[i + key.len()..];
            let digits: String = rest.chars().skip_while(|c| !c.is_ascii_digit()).take_while(|c| c.is_ascii_digit()).collect();
            digits.parse().map_err(|_| bad("meta field"))
        };
        let (largest, count, reach) = (field("largest")?, field("count")?, field("reach")?);
        let n = (count.saturating_sub(2)) as usize;
        let data = &bytes[data_at + hlen..];
        if data.len() < n {
            return Err(bad("the array is shorter than the count"));
        }
        let halfgaps = data[..n].to_vec();
        let marks = lay_marks(&halfgaps);
        Ok(Net { halfgaps, largest, count, reach, marks })
    }
}

/// **Exact `π(x)` with no prime list**, the Lucy_Hedgehog method, `O(x^0.75)`.
pub fn prime_count(x: u64) -> u64 {
    if x < 2 {
        return 0;
    }
    let r = (x as f64).sqrt() as u64;
    let r = if (r + 1) * (r + 1) <= x { r + 1 } else { r };
    // V = {x/1, x/2, ..., x/r} ∪ {r-1, ..., 1}; S[v] starts at v-1
    let mut big: Vec<u64> = (1..=r).map(|i| x / i).collect(); // big[i-1] = x / i
    let mut small: Vec<u64> = (0..=r).collect(); // small[v] = v, for v <= r
    let mut sb: Vec<u64> = big.iter().map(|&v| v - 1).collect();
    let mut ss: Vec<u64> = small.iter().map(|&v| v.saturating_sub(1)).collect();
    for p in 2..=r {
        if ss[p as usize] == ss[p as usize - 1] {
            continue; // p is not prime
        }
        let sp = ss[p as usize - 1];
        let p2 = p * p;
        for i in 0..big.len() {
            let v = big[i];
            if v < p2 {
                break;
            }
            let w = v / p;
            let sw = if w <= r { ss[w as usize] } else { sb[(x / w) as usize - 1] };
            sb[i] -= sw - sp;
        }
        let mut v = r;
        while v >= p2 {
            ss[v as usize] -= ss[(v / p) as usize] - sp;
            v -= 1;
        }
    }
    let _ = (&mut big, &mut small);
    sb[0]
}
