//! **Lars Rocha's addressed city.** A district number lays nine street strips on each axis by
//! reading its own hash channels; each of the 64 lots between them raises a building whose six
//! fields — floors, width, depth, roof, colour, kind — come from the lot's source prime and the
//! same channels; and every building is one packed integer in rulerpack's `L = 256, c = 2`
//! family, the lot card, which opens back to its address and fields. Edit the card, the building
//! changes. Nothing is stored: the city is a formula over its address.
//!
//! The engine's `wlaw::city`, carried here unchanged. This is the law behind `spindexin`'s
//! `addressed-city-model.js`, cell for cell: the same
//! `hash::at` channels (the JavaScript checks its word arithmetic against this crate's), the
//! same source-prime index `64·(d mod 32) + cell` into the first 2048 primes, the same payload
//! bits, the same geometry. [`reference_json`] prints a district the way the studio reads it,
//! so the two can be held to each other.

use self::hash::{at, stream_of};

/// The engine's hash field, lifted from `wlaw::hash` so the city reads the same channels.
pub mod hash {
/// The 32-bit avalanche. Two rounds of xor-shift and an odd multiply, each way.
#[inline]
pub fn mix(h: u32) -> u32 {
    let mut h = h ^ (h >> 16);
    h = h.wrapping_mul(0x7FEB_352D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846C_A68B);
    h ^ (h >> 16)
}

/// The field itself: one `u32` per `(seed, x, y, stream)`, stored nowhere.
#[inline]
pub fn at(seed: u32, x: i32, y: i32, stream: u32) -> u32 {
    let mut h = seed;
    h ^= (x as u32).wrapping_mul(0x9E37_79B1);
    h ^= (y as u32).wrapping_mul(0x85EB_CA77);
    h ^= stream.wrapping_mul(0xC2B2_AE3D);
    mix(h)
}

/// A named stream, so two laws reading the same cell never read the same number. FNV-1a, mixed.
///
/// The JavaScript walks `charCodeAt`, which is UTF-16 code units, so this does too: a non-ASCII
/// stream name hashes the same in both. (Every name in the laws today is ASCII.)
pub fn stream_of(name: &str) -> u32 {
    let mut h: u32 = 2_166_136_261;
    for c in name.encode_utf16() {
        h ^= c as u32;
        h = h.wrapping_mul(16_777_619);
    }
    mix(h)
}
}

pub const MAX_DISTRICT: u32 = u32::MAX;
/// A lot is sixteen units square.
pub const LOT: f64 = 16.0;
pub const ROOFS: [&str; 4] = ["Flat", "Gable", "Terrace", "Spire"];
pub const KINDS: [&str; 4] = ["Home", "Workshop", "Library", "Landmark"];
pub const COLORS: [&str; 16] = [
    "#d3ba92", "#a6bca1", "#acb9c7", "#c79c86", "#c4b1ca", "#e1c887", "#8ea9a1", "#b6c5cd", "#c59478",
    "#d3c9b7", "#92afc4", "#b4b679", "#cb9aa0", "#7eaaa5", "#a49bbd", "#d1aa70",
];

/// The six fields of a building.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fields {
    /// 1 to 12, three units each.
    pub floors: u32,
    /// 4 to 11 units.
    pub width: u32,
    /// 4 to 11 units.
    pub depth: u32,
    /// 0 flat, 1 gable (+3), 2 terrace (+2), 3 spire (+5).
    pub roof: u32,
    /// 0 to 15, an index into `COLORS`.
    pub color: u32,
    /// 0 to 3, an index into `KINDS`.
    pub kind: u32,
}

impl Fields {
    pub fn valid(&self) -> bool {
        (1..=12).contains(&self.floors) && (4..=11).contains(&self.width) && (4..=11).contains(&self.depth) && self.roof < 4 && self.color < 16 && self.kind < 4
    }
    /// The 18-bit payload: floors−1 in bits 0–3, width−4 in 4–6, depth−4 in 7–9, roof 10–11,
    /// colour 12–15, kind 16–17.
    pub fn payload(&self) -> u32 {
        (self.floors - 1) | ((self.width - 4) << 4) | ((self.depth - 4) << 7) | (self.roof << 10) | (self.color << 12) | (self.kind << 16)
    }
    pub fn from_payload(p: u32) -> Option<Fields> {
        if p > 262_143 {
            return None;
        }
        let f = Fields { floors: (p & 15) + 1, width: ((p >> 4) & 7) + 4, depth: ((p >> 7) & 7) + 4, roof: (p >> 10) & 3, color: (p >> 12) & 15, kind: (p >> 16) & 3 };
        f.valid().then_some(f)
    }
}

/// A channel of the district: `hash::at(d, cell, p, stream("city.<name>"))`.
pub fn channel(d: u32, cell: u32, p: u32, name: &str) -> u32 {
    at(d, cell as i32, p as i32, stream_of(&format!("city.{name}")))
}

/// One street strip: its span along its axis and its width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Strip {
    pub index: u32,
    pub lo: f64,
    pub hi: f64,
    pub center: f64,
    pub width: f64,
}

/// The nine strips on each axis, and the city's extent.
#[derive(Clone, Debug, PartialEq)]
pub struct Streets {
    pub x: Vec<Strip>,
    pub z: Vec<Strip>,
    pub width: f64,
    pub depth: f64,
}

/// **The streets of district `d`.** Each width is 3 or 5 from the district's own channel, the
/// middle strip two wider, with a sixteen-unit lot between neighbours.
pub fn layout(d: u32) -> Streets {
    let roads = |axis: &str| -> Vec<Strip> {
        let mut cursor = 0.0;
        (0..9u32)
            .map(|k| {
                let width = 3.0 + 2.0 * (channel(d, k, 0, &format!("{axis}-street")) % 2) as f64 + if k == 4 { 2.0 } else { 0.0 };
                let s = Strip { index: k, lo: cursor, hi: cursor + width, center: cursor + width / 2.0, width };
                cursor += width + LOT;
                s
            })
            .collect()
    };
    let (x, z) = (roads("x"), roads("z"));
    let (width, depth) = (x[8].hi, z[8].hi);
    Streets { x, z, width, depth }
}

fn first_primes(n: usize) -> Vec<u32> {
    let mut s = vec![true; 20_000];
    let mut out = Vec::with_capacity(n);
    for i in 2..s.len() {
        if s[i] {
            out.push(i as u32);
            if out.len() == n {
                break;
            }
            let mut j = i * i;
            while j < s.len() {
                s[j] = false;
                j += i;
            }
        }
    }
    out
}

/// The lot's source: index `64·(d mod 32) + cell` into the first 2048 primes, and that prime.
pub fn source(d: u32, cell: u32) -> (u32, u32) {
    let index = (d % 32) * 64 + cell;
    (index, first_primes(2048)[index as usize])
}

/// **The building a lot raises on its own**, from its source prime and the district's channels.
pub fn generated(d: u32, cell: u32) -> Fields {
    let (_, p) = source(d, cell);
    let c = |name: &str| channel(d, cell, p, name);
    Fields { floors: 1 + c("floors") % 12, width: 6 + c("width") % 6, depth: 6 + c("depth") % 6, roof: c("roof") % 4, color: c("color") % 16, kind: c("kind") % 4 }
}

/// The absolute lot address `64·d + cell`.
pub fn address(d: u32, cell: u32) -> u64 {
    d as u64 * 64 + cell as u64
}

// ── the lot card: rulerpack's pack in the L = 256, c = 2 family, shift 64 ──
const L: u128 = 256;
const C: u128 = 2;

/// `N = K << 64 | X` with `K = 2p ^ L·(i ^ (i + 2))` and `X = 2p ^ L·i`.
pub fn pack(i: u64, p: u32) -> u128 {
    let (i, p) = (i as u128, p as u128);
    let x = (2 * p) ^ (L * i);
    let k = (2 * p) ^ (L * (i ^ (i + C)));
    (k << 64) | x
}

/// The address and payload back from a card; `None` when the integer is not in this family.
pub fn unpack(n: u128) -> Option<(u64, u32)> {
    let x = n & ((1u128 << 64) - 1);
    let k = n >> 64;
    let z = k ^ x;
    if z % L != 0 || z / L < C {
        return None;
    }
    let i = z / L - C;
    let v = x ^ (L * i);
    if v & 1 != 0 {
        return None;
    }
    let p = v >> 1;
    let p = u32::try_from(p).ok()?;
    let i = u64::try_from(i).ok()?;
    if i > MAX_DISTRICT as u64 * 64 + 63 || pack(i, p) != n {
        return None;
    }
    Some((i, p))
}

/// The lot card for a building on a lot.
pub fn card(d: u32, cell: u32, f: &Fields) -> u128 {
    pack(address(d, cell), f.payload())
}

/// A card opened: the district, the cell and the fields.
pub fn open_card(n: u128) -> Option<(u32, u32, Fields)> {
    let (i, p) = unpack(n)?;
    let f = Fields::from_payload(p)?;
    Some(((i / 64) as u32, (i % 64) as u32, f))
}

/// The building's box on the ground: `x0, x1, z0, z1, y0, y1`, with the roof's height included.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub x0: f64,
    pub x1: f64,
    pub z0: f64,
    pub z1: f64,
    pub y0: f64,
    pub y1: f64,
}

/// A face of a building: its corners and what it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub vertices: Vec<[f64; 3]>,
    pub kind: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Geometry {
    pub faces: Vec<Face>,
    pub bounds: Bounds,
    pub center: [f64; 2],
    pub wall_height: f64,
    /// The sixteen-unit lot the building stands in.
    pub lot: Bounds,
}

/// **The building's geometry** on its lot: a box of `floors·3` units with the roof on top.
pub fn geometry(f: &Fields, cell: u32, streets: &Streets) -> Geometry {
    let (row, col) = ((cell / 8) as usize, (cell % 8) as usize);
    let x = streets.x[col].hi + LOT / 2.0;
    let z = streets.z[row].hi + LOT / 2.0;
    let (x0, x1) = (x - f.width as f64 / 2.0, x + f.width as f64 / 2.0);
    let (z0, z1) = (z - f.depth as f64 / 2.0, z + f.depth as f64 / 2.0);
    let h = f.floors as f64 * 3.0;
    let mut faces: Vec<Face> = Vec::new();
    let mut face = |v: Vec<[f64; 3]>, kind: &'static str| faces.push(Face { vertices: v, kind });
    let mut boxed = |a: f64, b: f64, c: f64, d: f64, y0: f64, y1: f64, roof: bool| {
        face(vec![[a, y0, c], [b, y0, c], [b, y1, c], [a, y1, c]], "north");
        face(vec![[b, y0, c], [b, y0, d], [b, y1, d], [b, y1, c]], "east");
        face(vec![[b, y0, d], [a, y0, d], [a, y1, d], [b, y1, d]], "south");
        face(vec![[a, y0, d], [a, y0, c], [a, y1, c], [a, y1, d]], "west");
        if roof {
            face(vec![[a, y1, c], [b, y1, c], [b, y1, d], [a, y1, d]], "roof");
        }
    };
    boxed(x0, x1, z0, z1, 0.0, h, f.roof != 1 && f.roof != 3);
    let mut top = h;
    match f.roof {
        1 => {
            top = h + 3.0;
            faces.push(Face { vertices: vec![[x0, h, z0], [x1, h, z0], [x, top, z0]], kind: "north" });
            faces.push(Face { vertices: vec![[x1, h, z1], [x0, h, z1], [x, top, z1]], kind: "south" });
            faces.push(Face { vertices: vec![[x0, h, z0], [x, top, z0], [x, top, z1], [x0, h, z1]], kind: "roof" });
            faces.push(Face { vertices: vec![[x, top, z0], [x1, h, z0], [x1, h, z1], [x, top, z1]], kind: "roof" });
        }
        2 => {
            top = h + 2.0;
            boxed(x0 + 1.0, x1 - 1.0, z0 + 1.0, z1 - 1.0, h, top, true);
        }
        3 => {
            top = h + 5.0;
            let tip = [x, top, z];
            faces.push(Face { vertices: vec![[x0, h, z0], [x1, h, z0], tip], kind: "roof" });
            faces.push(Face { vertices: vec![[x1, h, z0], [x1, h, z1], tip], kind: "roof" });
            faces.push(Face { vertices: vec![[x1, h, z1], [x0, h, z1], tip], kind: "roof" });
            faces.push(Face { vertices: vec![[x0, h, z1], [x0, h, z0], tip], kind: "roof" });
        }
        _ => {}
    }
    Geometry {
        faces,
        bounds: Bounds { x0, x1, z0, z1, y0: 0.0, y1: top },
        center: [x, z],
        wall_height: h,
        lot: Bounds { x0: streets.x[col].hi, x1: streets.x[col].hi + LOT, z0: streets.z[row].hi, z1: streets.z[row].hi + LOT, y0: 0.0, y1: 0.0 },
    }
}

/// One lot of a district, built or edited.
#[derive(Clone, Debug, PartialEq)]
pub struct Lot {
    pub cell: u32,
    pub address: u64,
    pub fields: Fields,
    pub payload: u32,
    pub n: u128,
    /// The source prime index and prime.
    pub source: (u32, u32),
    pub edited: bool,
    pub geometry: Geometry,
}

/// The lot, from its own law or from an edited card that belongs to it.
pub fn lot(d: u32, cell: u32, edited: Option<u128>, streets: &Streets) -> Option<Lot> {
    let (fields, n, was_edited) = match edited {
        Some(n) => {
            let (ed, ec, f) = open_card(n)?;
            if ed != d || ec != cell {
                return None;
            }
            (f, n, true)
        }
        None => {
            let f = generated(d, cell);
            (f, card(d, cell, &f), false)
        }
    };
    Some(Lot { cell, address: address(d, cell), fields, payload: fields.payload(), n, source: source(d, cell), edited: was_edited, geometry: geometry(&fields, cell, streets) })
}

/// **The whole district**: streets and the 64 lots, with any edited cards in place.
#[derive(Clone, Debug, PartialEq)]
pub struct District {
    pub d: u32,
    pub streets: Streets,
    pub lots: Vec<Lot>,
}

pub fn district(d: u32, edits: &[(u32, u128)]) -> Option<District> {
    let streets = layout(d);
    let mut lots = Vec::with_capacity(64);
    for cell in 0..64 {
        let e = edits.iter().find(|(c, _)| *c == cell).map(|(_, n)| *n);
        lots.push(lot(d, cell, e, &streets)?);
    }
    Some(District { d, streets, lots })
}

/// The construction row: prefix XOR over `[address, floors, width, depth, roof, color, kind]`,
/// from zero; XOR of neighbours recovers the fields.
pub fn prefix_row(l: &Lot) -> Vec<u64> {
    let f = &l.fields;
    let values = [l.address, f.floors as u64, f.width as u64, f.depth as u64, f.roof as u64, f.color as u64, f.kind as u64];
    let mut out = vec![0u64];
    for v in values {
        out.push(out[out.len() - 1] ^ v);
    }
    out
}

/// **The district as the studio reads it**: streets and lots as JSON, for holding the two
/// implementations to each other.
pub fn reference_json(d: u32) -> String {
    let c = district(d, &[]).expect("a district with no edits always builds");
    let strips = |v: &[Strip]| v.iter().map(|s| format!("{{\"index\":{},\"lo\":{},\"hi\":{},\"center\":{},\"width\":{}}}", s.index, s.lo, s.hi, s.center, s.width)).collect::<Vec<_>>().join(",");
    let lots = c
        .lots
        .iter()
        .map(|l| {
            let f = &l.fields;
            let b = &l.geometry.bounds;
            format!(
                "{{\"cell\":{},\"address\":\"{}\",\"floors\":{},\"width\":{},\"depth\":{},\"roof\":{},\"color\":{},\"kind\":{},\"payload\":{},\"N\":\"{}\",\"sourceIndex\":{},\"sourcePrime\":{},\"bounds\":[{},{},{},{},{},{}]}}",
                l.cell, l.address, f.floors, f.width, f.depth, f.roof, f.color, f.kind, l.payload, l.n, l.source.0, l.source.1, b.x0, b.x1, b.z0, b.z1, b.y0, b.y1
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"district\":\"{}\",\"streets\":{{\"x\":[{}],\"z\":[{}],\"width\":{},\"depth\":{}}},\"lots\":[{}]}}", d, strips(&c.streets.x), strips(&c.streets.z), c.streets.width, c.streets.depth, lots)
}
