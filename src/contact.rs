//! **First contact.** A footprint of fixed orientation sweeps a straight segment through a set of
//! axis-aligned solids; the answer is where it first enters one, as an exact fraction of the
//! segment, which solids it meets there, and the outward normals it meets them on.
//!
//! This is the contract Spindexin's First Contact study runs in the browser (`first-contact-model.js`,
//! contract version 1), written here in integer arithmetic so the two never disagree by a rounding.
//! Solid bounds and half-sizes are multiples of a quarter, so the expanded solids are exact; the
//! start and the displacement are any finite binary64 values, decoded into dyadic integers over
//! one shared exponent (the studio's `motion-contact-model.js`); every time of entry is a reduced
//! fraction of integers, and comparisons cross-multiply in a small big integer so nothing
//! overflows when a displacement is a thousandth of a micrometre. The policy:
//!
//! | case | answer |
//! |---|---|
//! | the start is strictly inside a solid | `Overlap`, fraction 0, every such solid, the start held |
//! | touching at the start and moving inward | a hit at fraction 0 |
//! | touching and moving outward or along the face | clear, unless another solid intervenes |
//! | entering on both axes together at a corner | both normals |
//! | touching only a point and leaving | clear: no open interval of penetration |
//! | an inward contact exactly at fraction 1 | a hit at 1 |
//! | several solids at the first time | every id and normal; the smallest id is the primary |
//! | a stationary footprint | clear unless it starts in overlap |
//!
//! The walker and the arena keep their own response rules; this is the primitive a district
//! adapter stops at before any sliding or impulse is added on top.

use std::cmp::Ordering;

/// What the sweep found.
#[derive(Clone, Debug, PartialEq)]
pub struct Contact {
    pub kind: Kind,
    /// Of the displacement: 1 when clear, 0 when overlapping at the start.
    pub fraction: Fraction,
    /// Every solid met at the first time, sorted by id; the smallest is the primary.
    pub ids: Vec<u128>,
    /// The unique outward axis normals met, sorted by x then z.
    pub normals: Vec<(i32, i32)>,
    /// Where the footprint stops.
    pub stop: (f64, f64),
    /// Where it asked to go.
    pub desired: (f64, f64),
}

/// The footprint's start, displacement and half-sizes, in metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    pub x: f64,
    pub z: f64,
    pub dx: f64,
    pub dz: f64,
    pub hx: f64,
    pub hz: f64,
}

/// A solid in the plane, with the stable decimal id the city gives its lots.
#[derive(Clone, Debug, PartialEq)]
pub struct Obstacle {
    pub id: u128,
    pub x0: f64,
    pub x1: f64,
    pub z0: f64,
    pub z1: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Clear,
    Hit,
    Overlap,
}

/// A reduced fraction with a positive denominator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fraction {
    pub n: i128,
    pub d: i128,
}

impl Fraction {
    pub fn value(&self) -> f64 {
        self.n as f64 / self.d as f64
    }
}

/// A small signed integer of any size: little-endian 64-bit limbs and a sign. Only what the
/// sweep needs: decode a float, add, subtract, multiply, compare, and reduce a fraction.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Big {
    neg: bool,
    mag: Vec<u64>,
}

impl Big {
    fn zero() -> Big {
        Big { neg: false, mag: Vec::new() }
    }
    fn trim(mut self) -> Big {
        while self.mag.last() == Some(&0) {
            self.mag.pop();
        }
        if self.mag.is_empty() {
            self.neg = false;
        }
        self
    }
    fn is_zero(&self) -> bool {
        self.mag.is_empty()
    }
    fn from_u128(n: u128) -> Big {
        Big { neg: false, mag: vec![n as u64, (n >> 64) as u64] }.trim()
    }
    /// The float as `mantissa · 2^exp`, exactly, with the mantissa as a magnitude and sign.
    fn dyadic(x: f64) -> (Big, i32) {
        if x == 0.0 {
            return (Big::zero(), 0);
        }
        let bits = x.to_bits();
        let exp = ((bits >> 52) & 0x7ff) as i32;
        let mut m = bits & ((1u64 << 52) - 1);
        if exp != 0 {
            m |= 1u64 << 52;
        }
        let e = if exp != 0 { exp - 1075 } else { -1074 };
        (Big { neg: bits >> 63 == 1, mag: vec![m] }.trim(), e)
    }
    fn shl(&self, by: u32) -> Big {
        if self.is_zero() || by == 0 {
            return self.clone();
        }
        let (limbs, bits) = ((by / 64) as usize, by % 64);
        let mut mag = vec![0u64; limbs];
        let mut carry = 0u64;
        for &l in &self.mag {
            if bits == 0 {
                mag.push(l);
            } else {
                mag.push((l << bits) | carry);
                carry = l >> (64 - bits);
            }
        }
        if carry != 0 {
            mag.push(carry);
        }
        Big { neg: self.neg, mag }.trim()
    }
    fn shr1(&self) -> Big {
        let mut mag = self.mag.clone();
        for i in 0..mag.len() {
            let next = if i + 1 < mag.len() { mag[i + 1] & 1 } else { 0 };
            mag[i] = (mag[i] >> 1) | (next << 63);
        }
        Big { neg: self.neg, mag }.trim()
    }
    fn cmp_mag(a: &[u64], b: &[u64]) -> Ordering {
        a.len().cmp(&b.len()).then_with(|| a.iter().rev().cmp(b.iter().rev()))
    }
    fn add_mag(a: &[u64], b: &[u64]) -> Vec<u64> {
        let mut out = Vec::with_capacity(a.len().max(b.len()) + 1);
        let mut carry = 0u128;
        for i in 0..a.len().max(b.len()) {
            let s = carry + *a.get(i).unwrap_or(&0) as u128 + *b.get(i).unwrap_or(&0) as u128;
            out.push(s as u64);
            carry = s >> 64;
        }
        if carry != 0 {
            out.push(carry as u64);
        }
        out
    }
    /// `a − b` for `|a| ≥ |b|`.
    fn sub_mag(a: &[u64], b: &[u64]) -> Vec<u64> {
        let mut out = Vec::with_capacity(a.len());
        let mut borrow = 0i128;
        for (i, &x) in a.iter().enumerate() {
            let mut d = x as i128 - *b.get(i).unwrap_or(&0) as i128 - borrow;
            borrow = 0;
            if d < 0 {
                d += 1i128 << 64;
                borrow = 1;
            }
            out.push(d as u64);
        }
        out
    }
    fn add(&self, o: &Big) -> Big {
        if self.neg == o.neg {
            return Big { neg: self.neg, mag: Big::add_mag(&self.mag, &o.mag) }.trim();
        }
        match Big::cmp_mag(&self.mag, &o.mag) {
            Ordering::Less => Big { neg: o.neg, mag: Big::sub_mag(&o.mag, &self.mag) }.trim(),
            _ => Big { neg: self.neg, mag: Big::sub_mag(&self.mag, &o.mag) }.trim(),
        }
    }
    fn neg(&self) -> Big {
        Big { neg: !self.neg && !self.is_zero(), mag: self.mag.clone() }
    }
    fn sub(&self, o: &Big) -> Big {
        self.add(&o.neg())
    }
    fn mul(&self, o: &Big) -> Big {
        if self.is_zero() || o.is_zero() {
            return Big::zero();
        }
        let mut mag = vec![0u64; self.mag.len() + o.mag.len()];
        for (i, &a) in self.mag.iter().enumerate() {
            let mut carry = 0u128;
            for (j, &b) in o.mag.iter().enumerate() {
                let t = a as u128 * b as u128 + mag[i + j] as u128 + carry;
                mag[i + j] = t as u64;
                carry = t >> 64;
            }
            let mut k = i + o.mag.len();
            while carry != 0 {
                let t = mag[k] as u128 + carry;
                mag[k] = t as u64;
                carry = t >> 64;
                k += 1;
            }
        }
        Big { neg: self.neg != o.neg, mag }.trim()
    }
    fn is_even(&self) -> bool {
        self.mag.first().is_none_or(|l| l & 1 == 0)
    }
    /// Stein's binary gcd of the magnitudes.
    fn gcd(a: &Big, b: &Big) -> Big {
        let (mut a, mut b) = (Big { neg: false, mag: a.mag.clone() }, Big { neg: false, mag: b.mag.clone() });
        if a.is_zero() {
            return b;
        }
        if b.is_zero() {
            return a;
        }
        let mut shift = 0u32;
        while a.is_even() && b.is_even() {
            a = a.shr1();
            b = b.shr1();
            shift += 1;
        }
        while a.is_even() {
            a = a.shr1();
        }
        loop {
            while b.is_even() {
                b = b.shr1();
            }
            if Big::cmp_mag(&a.mag, &b.mag) == Ordering::Greater {
                std::mem::swap(&mut a, &mut b);
            }
            b = b.sub(&a);
            if b.is_zero() {
                return a.shl(shift);
            }
        }
    }
    fn to_i128(&self) -> i128 {
        debug_assert!(self.mag.len() <= 2 && self.mag.get(1).is_none_or(|h| h >> 63 == 0), "a reduced fraction outgrew 127 bits");
        let m = self.mag.first().copied().unwrap_or(0) as u128 | ((self.mag.get(1).copied().unwrap_or(0) as u128) << 64);
        if self.neg { -(m as i128) } else { m as i128 }
    }
}

impl PartialOrd for Big {
    fn partial_cmp(&self, o: &Big) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl Ord for Big {
    fn cmp(&self, o: &Big) -> Ordering {
        match (self.neg, o.neg) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => Big::cmp_mag(&self.mag, &o.mag),
            (true, true) => Big::cmp_mag(&o.mag, &self.mag),
        }
    }
}

/// A time as a ratio of big integers, with a positive denominator; compared by cross-multiplying.
#[derive(Clone, Debug)]
struct Ratio {
    n: Big,
    d: Big,
}

impl Ratio {
    fn new(n: Big, d: Big) -> Ratio {
        if d.neg { Ratio { n: n.neg(), d: d.neg() } } else { Ratio { n, d } }
    }
    fn int(v: i128) -> Ratio {
        Ratio { n: Big { neg: v < 0, mag: vec![v.unsigned_abs() as u64, (v.unsigned_abs() >> 64) as u64] }.trim(), d: Big::from_u128(1) }
    }
    fn cmp(&self, o: &Ratio) -> Ordering {
        self.n.mul(&o.d).cmp(&o.n.mul(&self.d))
    }
    fn lt(&self, o: &Ratio) -> bool {
        self.cmp(o) == Ordering::Less
    }
    fn le(&self, o: &Ratio) -> bool {
        self.cmp(o) != Ordering::Greater
    }
    fn eq(&self, o: &Ratio) -> bool {
        self.cmp(o) == Ordering::Equal
    }
    /// The reduced fraction, in machine words.
    fn reduce(&self) -> Fraction {
        let g = Big::gcd(&self.n, &self.d);
        if g.mag.len() <= 2 && g.mag.get(1).is_none_or(|h| h >> 63 == 0) && g.to_i128() == 1 {
            return Fraction { n: self.n.to_i128(), d: self.d.to_i128() };
        }
        // divide both by g through subtraction-free shifting: g is a product of the common
        // powers of two and odd factors, so divide by the shared binary scale first
        let (mut n, mut d, mut g) = (self.n.clone(), self.d.clone(), g);
        while g.is_even() {
            n = n.shr1();
            d = d.shr1();
            g = g.shr1();
        }
        Fraction { n: div_small(&n, &g), d: div_small(&d, &g) }
    }
}

/// `a / g` where `g` is odd and divides `a` exactly, by shift-and-subtract long division.
fn div_small(a: &Big, g: &Big) -> i128 {
    let bits = a.mag.len() * 64;
    let mut rem = Big::zero();
    let mut quo = Big::zero();
    for i in (0..bits).rev() {
        let bit = (a.mag[i / 64] >> (i % 64)) & 1;
        rem = rem.shl(1).add(&Big::from_u128(bit as u128));
        quo = quo.shl(1);
        if Big::cmp_mag(&rem.mag, &g.mag) != Ordering::Less {
            rem = rem.sub(g);
            quo = quo.add(&Big::from_u128(1));
        }
    }
    if a.neg { -quo.to_i128() } else { quo.to_i128() }
}

/// The values as integers over one shared exponent, the studio's `integers`.
fn integers(values: &[f64]) -> Vec<Big> {
    let parts: Vec<(Big, i32)> = values.iter().map(|&v| Big::dyadic(v)).collect();
    let e = parts.iter().filter(|p| !p.0.is_zero()).map(|p| p.1).min().unwrap_or(0);
    parts.iter().map(|(m, k)| m.shl((k - e) as u32)).collect()
}

const Q: f64 = 4.0;

fn quarter(v: f64) -> f64 {
    let s = v * Q;
    debug_assert!(s == s.round() && v.abs() <= 4096.0, "{v} is not a multiple of a quarter within 4096 m");
    v
}

/// One axis: the open interval of times the point `p + t·d` lies strictly inside `(lo, hi)`,
/// as `(t0, t1)` ratios, or `None` when it never does. For `d = 0` the interval is everything
/// when `p` is strictly inside and nothing otherwise.
fn axis(p: &Big, d: &Big, lo: &Big, hi: &Big) -> Option<(Option<Ratio>, Option<Ratio>)> {
    if d.is_zero() {
        return (lo < p && p < hi).then_some((None, None));
    }
    let (a, b) = (Ratio::new(lo.sub(p), d.clone()), Ratio::new(hi.sub(p), d.clone()));
    let (t0, t1) = if a.lt(&b) { (a, b) } else { (b, a) };
    Some((Some(t0), Some(t1)))
}

/// **The sweep.**
pub fn sweep(m: &Motion, solids: &[Obstacle]) -> Contact {
    let desired = (m.x + m.dx, m.z + m.dz);
    for v in [m.x, m.z, m.dx, m.dz] {
        assert!(v.is_finite(), "the sweep takes finite motion");
    }
    let (hx, hz) = (quarter(m.hx), quarter(m.hz));
    let zero = Ratio::int(0);
    let one = Ratio::int(1);
    let mut overlap: Vec<u128> = Vec::new();
    let mut best: Option<Ratio> = None;
    let mut hits: Vec<(u128, Vec<(i32, i32)>)> = Vec::new();
    for s in solids {
        // the solid expanded by the footprint's half-sizes: exact, every term a quarter
        let (lx, ux, lz, uz) = (quarter(s.x0) - hx, quarter(s.x1) + hx, quarter(s.z0) - hz, quarter(s.z1) + hz);
        // the cheap rejection the studio makes first, in floats, exact for the same reason
        if m.x.max(m.x + m.dx) < lx || m.x.min(m.x + m.dx) > ux || m.z.max(m.z + m.dz) < lz || m.z.min(m.z + m.dz) > uz {
            continue;
        }
        let v = integers(&[m.x, m.z, m.dx, m.dz, lx, ux, lz, uz]);
        let (px, pz, dx, dz) = (&v[0], &v[1], &v[2], &v[3]);
        let Some(ix) = axis(px, dx, &v[4], &v[5]) else { continue };
        let Some(iz) = axis(pz, dz, &v[6], &v[7]) else { continue };
        // the entry time is the later of the two axis entries, the exit the earlier of the exits
        let t0 = match (&ix.0, &iz.0) {
            (None, None) => None,
            (Some(a), None) | (None, Some(a)) => Some(a.clone()),
            (Some(a), Some(b)) => Some(if a.lt(b) { b.clone() } else { a.clone() }),
        };
        let t1 = match (&ix.1, &iz.1) {
            (None, None) => None,
            (Some(a), None) | (None, Some(a)) => Some(a.clone()),
            (Some(a), Some(b)) => Some(if a.lt(b) { a.clone() } else { b.clone() }),
        };
        // an open interval of penetration must be non-empty and reach past the start
        let entry = t0.unwrap_or(Ratio::int(-1));
        let exit = t1.unwrap_or(Ratio::int(2));
        if !entry.lt(&exit) || !zero.lt(&exit) {
            continue;
        }
        if entry.lt(&zero) {
            overlap.push(s.id);
            continue;
        }
        if !entry.le(&one) {
            continue;
        }
        // which axes enter at this time: a normal per axis that does
        let mut normals = Vec::new();
        if let Some(ax) = &ix.0 {
            if ax.eq(&entry) && !dx.is_zero() {
                normals.push((if dx.neg { 1 } else { -1 }, 0));
            }
        }
        if let Some(az) = &iz.0 {
            if az.eq(&entry) && !dz.is_zero() {
                normals.push((0, if dz.neg { 1 } else { -1 }));
            }
        }
        match &best {
            Some(b) if b.lt(&entry) => {}
            Some(b) if entry.lt(b) => {
                best = Some(entry);
                hits = vec![(s.id, normals)];
            }
            Some(_) => hits.push((s.id, normals)),
            None => {
                best = Some(entry);
                hits = vec![(s.id, normals)];
            }
        }
    }
    if !overlap.is_empty() {
        overlap.sort_unstable();
        overlap.dedup();
        return Contact { kind: Kind::Overlap, fraction: Fraction { n: 0, d: 1 }, ids: overlap, normals: Vec::new(), stop: (m.x, m.z), desired };
    }
    match best {
        None => Contact { kind: Kind::Clear, fraction: Fraction { n: 1, d: 1 }, ids: Vec::new(), normals: Vec::new(), stop: desired, desired },
        Some(t) => {
            let mut ids: Vec<u128> = hits.iter().map(|h| h.0).collect();
            ids.sort_unstable();
            ids.dedup();
            let mut normals: Vec<(i32, i32)> = hits.iter().flat_map(|h| h.1.iter().copied()).collect();
            normals.sort_unstable();
            normals.dedup();
            let t = t.reduce();
            let stop = (m.x + m.dx * t.value(), m.z + m.dz * t.value());
            Contact { kind: Kind::Hit, fraction: t, ids, normals, stop, desired }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_big_integer_does_arithmetic() {
        let a = Big::from_u128(u128::MAX);
        let sq = a.mul(&a);
        assert_eq!(sq.mag.len(), 4);
        assert_eq!(sq.sub(&a.shl(128)).add(&a.shl(1)), a);
        assert_eq!(Big::gcd(&Big::from_u128(841062666872 * 12), &Big::from_u128(879286712481 * 12)), Big::from_u128(12));
        let (m, e) = Big::dyadic(-0.75);
        assert_eq!((m, e), (Big { neg: true, mag: vec![3 << 51] }, -53));
        assert_eq!(Big::dyadic(0.0), (Big::zero(), 0));
        let r = Ratio::new(Big::from_u128(6), Big::from_u128(4));
        assert_eq!(r.reduce(), Fraction { n: 3, d: 2 });
        let r = Ratio::new(Big::from_u128(6).neg(), Big::from_u128(4).neg());
        assert_eq!(r.reduce(), Fraction { n: 3, d: 2 });
    }

    #[test]
    fn a_live_position_sweeps_exactly() {
        // the studio's wall approach at tick 208: the fraction it recorded
        let m = Motion { x: 154.5956177549224, z: 148.0, dx: -0.0999633257926007, dz: 0.0, hx: 2.5, hz: 2.5 };
        let wall = Obstacle { id: 2907, x0: 140.0, x1: 152.0, z0: 140.0, z1: 156.0 };
        let c = sweep(&m, std::slice::from_ref(&wall));
        assert_eq!(c.kind, Kind::Hit);
        assert_eq!(c.fraction, Fraction { n: 841062666872, d: 879286712481 });
        assert_eq!(c.normals, vec![(1, 0)]);
        // a displacement a thousandth of a micrometre long, at the far end of the exponent range
        let m = Motion { x: 154.5 + 1e-9, z: 148.0, dx: -2e-9, dz: 1e-20, hx: 2.5, hz: 2.5 };
        let c = sweep(&m, std::slice::from_ref(&wall));
        assert_eq!(c.kind, Kind::Hit);
        assert!((c.fraction.value() - 0.5).abs() < 1e-4, "{:?}", c.fraction);
    }
}

/// A moving car for [`pair`]: its stable address, start, displacement and clearance half-sizes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    pub id: u128,
    pub x: f64,
    pub z: f64,
    pub dx: f64,
    pub dz: f64,
    pub hx: f64,
    pub hz: f64,
}

/// An exact position: `(x, z)` as reduced fractions.
pub type Point = (Fraction, Fraction);

/// What two moving cars did: the studio's `moving-pair-exact-v1`.
#[derive(Clone, Debug, PartialEq)]
pub struct Pair {
    pub kind: Kind,
    /// Of the second, shared, as a fraction: 1 when clear, 0 when overlapping at the start.
    pub fraction: Fraction,
    /// Both addresses, sorted.
    pub ids: Vec<u128>,
    /// Outward from B toward A, the sweep's convention.
    pub normals_on_a: Vec<(i32, i32)>,
    /// The one relative sweep the pair became.
    pub relative: Motion,
    /// B as the solid the relative sweep met, centred on the origin.
    pub obstacle: Obstacle,
    /// Where each car is at the fraction: A then B, exact.
    pub contact: [Point; 2],
    /// Where each asked to go.
    pub desired: [Point; 2],
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a.max(1)
}

fn q4(v: f64) -> i128 {
    quarter(v);
    (v * Q).round() as i128
}

/// `start + t·displacement` on the quarter grid, exact.
fn along(start: f64, disp: f64, t: Fraction) -> Fraction {
    // start, disp are multiples of a quarter: (4·start·d + 4·disp·n) / (4·d)
    let n = q4(start) * t.d + q4(disp) * t.n;
    let g = gcd(n.unsigned_abs(), (4 * t.d).unsigned_abs()) as i128;
    Fraction { n: n / g, d: 4 * t.d / g }
}

/// **Two moving cars as one sweep.** The relative motion of A against B, with B as a solid of its
/// own half-sizes at the origin, through [`sweep`]; the exact fraction then places both cars on
/// their own world paths. Adding one displacement to both cars leaves the fraction and the
/// normals alone; swapping the cars keeps the kind and fraction, swaps the positions and negates
/// the normals.
pub fn pair(a: &Body, b: &Body) -> Pair {
    let relative = Motion { x: a.x - b.x, z: a.z - b.z, dx: a.dx - b.dx, dz: a.dz - b.dz, hx: a.hx, hz: a.hz };
    let obstacle = Obstacle { id: b.id, x0: -b.hx, x1: b.hx, z0: -b.hz, z1: b.hz };
    let hit = sweep(&relative, std::slice::from_ref(&obstacle));
    let t = hit.fraction;
    let mut ids = vec![a.id, b.id];
    ids.sort_unstable();
    let one = Fraction { n: 1, d: 1 };
    Pair {
        kind: hit.kind,
        fraction: t,
        ids,
        normals_on_a: hit.normals,
        relative,
        obstacle,
        contact: [(along(a.x, a.dx, t), along(a.z, a.dz, t)), (along(b.x, b.dx, t), along(b.z, b.dz, t))],
        desired: [(along(a.x, a.dx, one), along(a.z, a.dz, one)), (along(b.x, b.dx, one), along(b.z, b.dz, one))],
    }
}
