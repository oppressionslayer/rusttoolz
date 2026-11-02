//! **District drive: the city, the car and the wall, as one adapter.** Spindexin's District Drive
//! studio composes three things this crate already holds: the addressed city
//! ([`wlaw::city`]), the one tick law ([`step_drive`] on flat ground) and first contact
//! ([`contact::sweep`]). This module is that composition, under the browser's named response
//! profile **`district-clearance-v1`**, with the studio's replay machinery: checkpoints every 32
//! ticks, the block of commands between them kept as one packed word of five-bit lanes holding
//! the XOR left edge (a block card, any command read in at most five shift/XORs), and the sweep
//! tested only against the lots the street layout puts under the car's box plus the four edges (the engine's `wcraft::district`, held to the same six portable cases), and branches that share a prefix with their original.
//!
//! The profile, as the studio states it:
//!
//! 1. The car is enclosed by a fixed, world-aligned square whose half-size is the car's largest
//!    distance from its centre, rounded up to a quarter metre. The square holds the car at every
//!    heading; the opening coupé's is 2.5 m. The half-size is an input here, because the car's
//!    shell lives in the studio's ruler workshop, not in this crate.
//! 2. That square is swept from the prior centre through the proposed displacement against all 64
//!    building footprints and the four district edges. Geometry is scaled by two metres per city
//!    unit exactly once, when the scene is built.
//! 3. The sweep keeps the first contact fraction, every solid met at it in id order, and the
//!    unique outward normals.
//! 4. At a contact the car takes `max(0, time − 1e-6/distance)` of the displacement, or none of it
//!    when it started overlapping; speed and velocity go to zero; the proposed yaw and cosmetics
//!    stay. That leaves up to a micrometre of clearance along the segment.
//!
//! Building ids are `64·district + cell`; the edges are `2^40 − 4` to `2^40 − 1` for west, east,
//! north and south. The walker and the arena keep their own response rules.

use crate::contact::{self, Contact, Kind, Motion, Obstacle};
use crate::motion::{self, Handling, Pose};
use std::f64::consts::PI;

/// **A block card**: up to 32 commands as one 160-bit word of five-bit lanes holding the XOR
/// triangle's left edge. Command `j` is read with at most five whole-word shift/XORs
/// ([`word_at`]); a lane is appended without the earlier commands ([`Card::extend`]). The portable
/// form is `{"format":"spindexin-drive-block","version":1,"length":L,"N":"<decimal>"}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Card {
    pub length: u8,
    pub word: Word,
}

/// 160 bits as three little-endian limbs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Word(pub [u64; 3]);

impl Word {
    pub const ZERO: Word = Word([0; 3]);
    fn shr(self, by: u32) -> Word {
        let (limbs, bits) = ((by / 64) as usize, by % 64);
        let mut out = [0u64; 3];
        for (i, slot) in out.iter_mut().enumerate() {
            let j = i + limbs;
            if j < 3 {
                *slot = self.0[j] >> bits;
                if bits != 0 && j + 1 < 3 {
                    *slot |= self.0[j + 1] << (64 - bits);
                }
            }
        }
        Word(out)
    }
    fn xor(self, o: Word) -> Word {
        Word([self.0[0] ^ o.0[0], self.0[1] ^ o.0[1], self.0[2] ^ o.0[2]])
    }
    fn set_lane(self, j: u32, lane: u64) -> Word {
        let (limb, bit) = ((5 * j / 64) as usize, 5 * j % 64);
        let mut out = self.0;
        out[limb] |= lane << bit;
        if bit > 59 && limb + 1 < 3 {
            out[limb + 1] |= lane >> (64 - bit);
        }
        Word(out)
    }
    fn lane(self, j: u32) -> u64 {
        self.shr(5 * j).0[0] & 31
    }
    fn is_zero(self) -> bool {
        self.0 == [0; 3]
    }
    /// The word in decimal.
    pub fn to_decimal(self) -> String {
        if self.is_zero() {
            return "0".into();
        }
        let mut limbs = self.0;
        let mut digits = Vec::new();
        while limbs != [0; 3] {
            let mut rem = 0u128;
            for l in limbs.iter_mut().rev() {
                let cur = (rem << 64) | *l as u128;
                *l = (cur / 10) as u64;
                rem = cur % 10;
            }
            digits.push(b'0' + rem as u8);
        }
        digits.reverse();
        String::from_utf8(digits).expect("digits")
    }
    /// A word from decimal, `None` past 160 bits or on a bad digit.
    pub fn from_decimal(s: &str) -> Option<Word> {
        if s.is_empty() || (s.len() > 1 && s.starts_with('0')) {
            return None;
        }
        let mut limbs = [0u64; 3];
        for c in s.bytes() {
            let d = c.checked_sub(b'0').filter(|d| *d < 10)? as u128;
            let mut carry = d;
            for l in limbs.iter_mut() {
                let cur = *l as u128 * 10 + carry;
                *l = cur as u64;
                carry = cur >> 64;
            }
            if carry != 0 || limbs[2] >> 32 != 0 {
                return None;
            }
        }
        Some(Word(limbs))
    }
}

/// **Command `j` out of a packed edge word.** For each set bit `b` of `j` fold `word >> 5b` in;
/// the low lane is then `XOR(edge[m] for m submask of j)`, the triangle's closed form.
pub fn word_at(word: Word, j: u32) -> u8 {
    let mut v = word;
    let mut b = 1u32;
    while b <= j {
        if j & b != 0 {
            v = v.xor(v.shr(5 * b));
        }
        b *= 2;
    }
    (v.0[0] & 31) as u8
}

impl Card {
    /// One more command: lane `j = length` becomes `code ^ word_at(N, j)`, the transform's own
    /// involution, so no earlier command is needed.
    pub fn extend(self, code: u8) -> Result<Card, &'static str> {
        if code > 17 {
            return Err("an input code is 0 to 17");
        }
        if self.length as usize >= EVERY {
            return Err("a block holds 32 commands");
        }
        let j = self.length as u32;
        let e = word_at(self.word, j) ^ code;
        Ok(Card { length: self.length + 1, word: self.word.set_lane(j, e as u64) })
    }
    /// Up to 32 commands as one card.
    pub fn pack(codes: &[u8]) -> Result<Card, &'static str> {
        codes.iter().try_fold(Card::default(), |c, &code| c.extend(code))
    }
    /// Command `j` straight from the card, validated as a code.
    pub fn command_at(&self, j: u32) -> Result<u8, &'static str> {
        if j >= self.length as u32 {
            return Err("the command index is inside the block");
        }
        let code = word_at(self.word, j);
        if code > 17 {
            return Err("the card holds a command outside 0 to 17");
        }
        Ok(code)
    }
    /// The edge lanes.
    pub fn edge(&self) -> Vec<u64> {
        (0..self.length as u32).map(|i| self.word.lane(i)).collect()
    }
    /// Every command read back.
    pub fn commands(&self) -> Result<Vec<u8>, &'static str> {
        (0..self.length as u32).map(|i| self.command_at(i)).collect()
    }
    /// The portable form.
    pub fn to_json(&self) -> String {
        format!("{{\"format\":\"spindexin-drive-block\",\"version\":1,\"length\":{},\"N\":\"{}\"}}", self.length, self.word.to_decimal())
    }
    /// A card from its portable form: version 1, length 0..32, the word inside its lanes.
    pub fn from_json(text: &str) -> Option<Card> {
        let field = |key: &str| -> Option<&str> {
            let i = text.find(&format!("\"{key}\":"))? + key.len() + 3;
            let rest = text[i..].trim_start();
            let end = rest.find([',', '}']).unwrap_or(rest.len());
            Some(rest[..end].trim().trim_matches('"'))
        };
        if field("format")? != "spindexin-drive-block" || field("version")? != "1" {
            return None;
        }
        let length: u8 = field("length")?.parse().ok().filter(|l| *l as usize <= EVERY)?;
        let word = Word::from_decimal(field("N")?)?;
        if length < 32 && !word.shr(5 * length as u32).is_zero() {
            return None;
        }
        Some(Card { length, word })
    }
}

/// The studio's rate and step.
pub use crate::motion::{DT, HZ};
/// A history holds at most this many input codes.
pub const LIMIT: usize = 600;
/// A checkpoint every this many ticks; the block between two is one XOR edge.
pub const EVERY: usize = 32;
/// Metres per city unit.
pub const SCALE: f64 = 2.0;
/// The clearance left along the segment at a contact, in metres.
pub const SKIN: f64 = 1e-6;
pub const PROFILE: &str = "district-clearance-v1";
/// The first of the four edge ids: west, then east, north, south.
pub const EDGE_BASE: u128 = (1u128 << 40) - 4;

/// Where a drive begins: on the middle avenue heading north (`yaw = π/2`), or in front of
/// building 27 heading west (`yaw = π`) at the wall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start {
    Avenue,
    Wall,
}

/// The input code of a throttle, a steer and a brake, each in `-1 ..= 1`.
pub fn encode(throttle: i8, steer: i8, brake: bool) -> u8 {
    ((throttle + 1) + 3 * (steer + 1) + 9 * brake as i8) as u8
}

/// **A district ready to drive**: its solids in metres, the spawn, the car's clearance square
/// and the handling, with the key the studio uses to tell one scene from another.
#[derive(Clone, Debug)]
pub struct Scene {
    pub district: crate::city::District,
    pub solids: Vec<Obstacle>,
    /// The district's extent in metres.
    pub width: f64,
    pub depth: f64,
    pub spawn: Pose,
    pub half: f64,
    pub handling: Handling,
    pub start: Start,
}

impl Scene {
    /// Build district `d` with its edited cards, scale it to metres, place the four edges, and set
    /// the spawn. Returns `None` when the district does not build or the starting clearance
    /// already overlaps the city.
    pub fn new(d: u32, edits: &[(u32, u128)], start: Start, half: f64, handling: Handling) -> Option<Scene> {
        let district = crate::city::district(d, edits)?;
        let width = district.streets.width * SCALE;
        let depth = district.streets.depth * SCALE;
        let mut solids: Vec<Obstacle> = district
            .lots
            .iter()
            .map(|l| {
                let b = l.geometry.bounds;
                Obstacle { id: l.address as u128, x0: b.x0 * SCALE, x1: b.x1 * SCALE, z0: b.z0 * SCALE, z1: b.z1 * SCALE }
            })
            .collect();
        let edges = [(-16.0, 0.0, -16.0, depth + 16.0), (width, width + 16.0, -16.0, depth + 16.0), (0.0, width, -16.0, 0.0), (0.0, width, depth, depth + 16.0)];
        for (i, (x0, x1, z0, z1)) in edges.into_iter().enumerate() {
            solids.push(Obstacle { id: EDGE_BASE + i as u128, x0, x1, z0, z1 });
        }
        let x = district.streets.x[4].center * SCALE;
        let (z, yaw) = match start {
            Start::Wall => (district.lots[27].geometry.center[1] * SCALE, PI),
            Start::Avenue => (district.streets.z[3].center * SCALE, PI / 2.0),
        };
        let spawn = Pose::at(x, z, yaw);
        let scene = Scene { district, solids, width, depth, spawn, half, handling, start };
        let m = Motion { x: spawn.x, z: spawn.z, dx: 0.0, dz: 0.0, hx: half, hz: half };
        (contact::sweep(&m, &scene.solids).kind != Kind::Overlap).then_some(scene)
    }

    /// **The solids the segment's clearance box can touch**: the lot rows and columns under it
    /// by binary search over the nine street strips, as `cell = 8·row + col`, then the four
    /// edges. Indices into `solids`.
    pub fn lots_near(&self, m: &Motion) -> Vec<usize> {
        let axis = |strips: &[crate::city::Strip], p: f64, d: f64, h: f64| -> (usize, usize) {
            let (low, high) = (p.min(p + d), p.max(p + d));
            let (mut a, mut b) = (0usize, 8usize);
            while a < b {
                let k = (a + b) >> 1;
                if strips[k + 1].lo * SCALE + h < low { a = k + 1 } else { b = k }
            }
            let first = a;
            let (mut a, mut b) = (0usize, 8usize);
            while a < b {
                let k = (a + b) >> 1;
                if strips[k].hi * SCALE - h <= high { a = k + 1 } else { b = k }
            }
            (first, a)
        };
        let st = &self.district.streets;
        let (c0, c1) = axis(&st.x, m.x, m.dx, m.hx);
        let (r0, r1) = axis(&st.z, m.z, m.dz, m.hz);
        let mut out: Vec<usize> = (r0..r1).flat_map(|r| (c0..c1).map(move |c| 8 * r + c)).collect();
        out.extend(64..68);
        out
    }

    /// **One tick under the profile.** The law proposes the next pose; the clearance square is
    /// swept to it; a contact stops the car short by the skin and zeroes its motion. The receipt
    /// is `Some` only when something was met.
    pub fn step(&self, pose: &Pose, code: u8, tick: usize) -> (Pose, Option<Receipt>) {
        let v = motion::step(pose, code as i64, &self.handling, DT);
        let motion = Motion { x: pose.x, z: pose.z, dx: v.x - pose.x, dz: v.z - pose.z, hx: self.half, hz: self.half };
        let near: Vec<Obstacle> = self.lots_near(&motion).into_iter().map(|i| self.solids[i].clone()).collect();
        let hit = contact::sweep(&motion, &near);
        let mut next = v;
        if hit.kind == Kind::Clear {
            return (next, None);
        }
        let distance = motion.dx.hypot(motion.dz);
        let used = if hit.kind == Kind::Overlap { 0.0 } else { (hit.fraction.value() - if distance > 0.0 { SKIN / distance } else { 0.0 }).max(0.0) };
        next.x = pose.x + motion.dx * used;
        next.z = pose.z + motion.dz * used;
        next.speed = 0.0;
        next.vx = 0.0;
        next.vz = 0.0;
        (next, Some(Receipt { tick, code, motion, contact: hit, used, stop: (next.x, next.z) }))
    }

    /// A fresh trace from the spawn.
    pub fn empty(&self) -> Trace {
        Trace { inputs: Vec::new(), frames: vec![self.spawn], contacts: Vec::new(), checkpoints: vec![Checkpoint { tick: 0, pose: self.spawn }], cards: vec![Card::default()] }
    }

    /// Drive every code of `inputs` from the spawn.
    pub fn trace(&self, inputs: &[u8]) -> Result<Trace, &'static str> {
        let mut t = self.empty();
        for &c in inputs {
            self.append(&mut t, c)?;
        }
        Ok(t)
    }

    /// One more tick on a trace: the frame, any receipt, and a checkpoint when the tick is a
    /// multiple of [`EVERY`].
    pub fn append(&self, t: &mut Trace, code: u8) -> Result<(), &'static str> {
        if t.inputs.len() >= LIMIT {
            return Err("this drive has reached 600 ticks");
        }
        if code > 17 {
            return Err("an input code is 0 to 17");
        }
        let tick = t.inputs.len() + 1;
        let (pose, receipt) = self.step(t.frames.last().expect("a trace holds its spawn"), code, tick);
        t.inputs.push(code);
        t.frames.push(pose);
        let last = t.cards.last_mut().expect("a trace holds a card");
        *last = last.extend(code)?;
        if tick % EVERY == 0 && tick < LIMIT {
            t.cards.push(Card::default());
        }
        if let Some(r) = receipt {
            t.contacts.push(r);
        }
        if tick % EVERY == 0 {
            t.checkpoints.push(Checkpoint { tick, pose });
        }
        Ok(())
    }

    /// **Return to any tick from its checkpoint**, reading each command it needs straight from
    /// the block's card: at most 31 steps. The pose must equal the frame the trace recorded.
    pub fn seek(&self, t: &Trace, tick: usize) -> Result<Seek, &'static str> {
        if tick > t.inputs.len() {
            return Err("the replay tick is past the drive");
        }
        let mark = t.checkpoints[tick / EVERY];
        let card = t.cards[tick / EVERY];
        let mut pose = mark.pose;
        let mut contacts = Vec::new();
        for i in 0..tick - mark.tick {
            let (next, r) = self.step(&pose, card.command_at(i as u32)?, mark.tick + i + 1);
            pose = next;
            contacts.extend(r);
        }
        Ok(Seek { pose, checkpoint: mark.tick, steps: tick - mark.tick, card, contacts })
    }
}

/// What a contact left behind: the tick and code, the motion asked for, the sweep's answer, the
/// share of the displacement taken, and where the car stopped.
#[derive(Clone, Debug, PartialEq)]
pub struct Receipt {
    pub tick: usize,
    pub code: u8,
    pub motion: Motion,
    pub contact: Contact,
    pub used: f64,
    pub stop: (f64, f64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Checkpoint {
    pub tick: usize,
    pub pose: Pose,
}

/// A drive: its codes, a frame per tick from the spawn, every receipt, and the checkpoints.
#[derive(Clone, Debug, PartialEq)]
pub struct Trace {
    pub inputs: Vec<u8>,
    pub frames: Vec<Pose>,
    pub contacts: Vec<Receipt>,
    pub checkpoints: Vec<Checkpoint>,
    /// One card per block of 32 commands, built as the drive goes.
    pub cards: Vec<Card>,
}

impl Trace {
    /// The first receipt, if the drive met anything.
    pub fn first_contact(&self) -> Option<&Receipt> {
        self.contacts.first()
    }
}

/// **The XOR difference triangle's left edge** of a row: the row's first value, then the first
/// value of each row of neighbour XORs down to the apex.
pub fn left_edge(values: &[u64]) -> Vec<u64> {
    let mut row = values.to_vec();
    let mut edge = Vec::with_capacity(row.len());
    while !row.is_empty() {
        edge.push(row[0]);
        row = row.windows(2).map(|w| w[0] ^ w[1]).collect();
    }
    edge
}

/// The row back from its left edge: the inverse of [`left_edge`], exact.
pub fn from_left_edge(edge: &[u64]) -> Vec<u64> {
    let Some(&apex) = edge.last() else { return Vec::new() };
    let mut row = vec![apex];
    for k in (0..edge.len() - 1).rev() {
        let mut v = edge[k];
        let mut next = vec![v];
        for &x in &row {
            v ^= x;
            next.push(v);
        }
        row = next;
    }
    row
}

/// The 32-command block holding `tick`: where it starts, its codes, the card, its edge lanes, and
/// the codes read back from the card.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub start: usize,
    pub values: Vec<u8>,
    pub card: Card,
    pub edge: Vec<u64>,
    pub recovered: Vec<u8>,
}

pub fn block(inputs: &[u8], tick: usize) -> Block {
    let start = tick / EVERY * EVERY;
    let values: Vec<u8> = inputs.iter().skip(start).take(EVERY).copied().collect();
    let card = Card::pack(&values).expect("codes of a drive are 0 to 17");
    Block { start, values, edge: card.edge(), recovered: card.commands().expect("the card holds what was packed"), card }
}

/// Where a seek landed.
#[derive(Clone, Debug, PartialEq)]
pub struct Seek {
    pub pose: Pose,
    pub checkpoint: usize,
    pub steps: usize,
    pub card: Card,
    pub contacts: Vec<Receipt>,
}

/// **A branch**: the original's codes with `count` of them replaced by `code` from `tick` on.
/// The prefix before the fork is the original's, exactly, which [`shares_prefix`] checks.
pub fn fork(inputs: &[u8], tick: usize, code: u8, count: usize) -> Result<Vec<u8>, &'static str> {
    if tick > inputs.len() {
        return Err("the fork tick is past the drive");
    }
    if !(1..=240).contains(&count) {
        return Err("change 1 to 240 ticks");
    }
    if code > 17 {
        return Err("an input code is 0 to 17");
    }
    let mut out = inputs.to_vec();
    for c in out.iter_mut().skip(tick).take(count) {
        *c = code;
    }
    Ok(out)
}

/// Both histories agree before the fork.
pub fn shares_prefix(original: &[u8], branch: &[u8], fork: usize) -> bool {
    fork <= original.len() && fork <= branch.len() && original[..fork] == branch[..fork]
}

/// The first state where two traces differ, if any.
pub fn first_difference(a: &Trace, b: &Trace) -> Option<usize> {
    a.frames.iter().zip(&b.frames).position(|(p, q)| p != q)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip() {
        for code in 0..18u8 {
            let (t, st, b) = motion::decode_input(code as i64);
            assert_eq!(encode(t as i8, st as i8, b), code);
        }
        assert_eq!(encode(1, 0, false), 5);
        assert_eq!(encode(1, 1, false), 8);
    }

    #[test]
    fn the_edge_rebuilds_any_block_and_the_card_reads_it() {
        let row: Vec<u64> = (0..32).map(|i| (i * 7 + 3) % 18).collect();
        assert_eq!(from_left_edge(&left_edge(&row)), row);
        assert_eq!(from_left_edge(&left_edge(&[])), Vec::<u64>::new());
        let codes: Vec<u8> = row.iter().map(|&v| v as u8).collect();
        let card = Card::pack(&codes).unwrap();
        assert_eq!(card.edge(), left_edge(&row));
        assert_eq!(card.commands().unwrap(), codes);
        assert_eq!(Card::pack(&[17, 8, 11]).unwrap().word.to_decimal(), "27441");
        assert_eq!(Card::from_json(&card.to_json()), Some(card));
        assert_eq!(Word::from_decimal("946145646054794016856395215117").map(|w| w.to_decimal()), Some("946145646054794016856395215117".to_string()));
        assert!(Word::from_decimal("1461501637330902918203684832716283019655932542976").is_none()); // 2^160
        assert!(Card::pack(&[18]).is_err());
    }

    #[test]
    fn district_45_spawns_where_the_studio_does() {
        let s = Scene::new(45, &[], Start::Wall, 2.5, Handling::default()).unwrap();
        assert_eq!((s.spawn.x, s.spawn.z, s.spawn.yaw), (167.0, 148.0, PI));
        assert_eq!(s.solids.len(), 68);
        assert_eq!(s.solids[27].id, 64 * 45 + 27);
        assert_eq!(s.solids[64].id, EDGE_BASE);
        let a = Scene::new(45, &[], Start::Avenue, 2.5, Handling::default()).unwrap();
        assert_eq!(a.spawn.yaw, PI / 2.0);
    }

    #[test]
    fn a_fork_keeps_its_prefix() {
        let inputs = vec![2u8; 100];
        let b = fork(&inputs, 60, 5, 160).unwrap();
        assert!(shares_prefix(&inputs, &b, 60));
        assert_eq!(&b[60..], &[5u8; 40][..]);
        assert!(fork(&inputs, 101, 5, 1).is_err());
        assert!(fork(&inputs, 0, 18, 1).is_err());
    }
}
