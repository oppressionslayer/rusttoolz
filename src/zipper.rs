//! **Two queues through one gate**, the studio's `zipper-gate-v1`.
//!
//! Two lanes of cars wait at a gate, each a packed rulerpack car card with a ready tick. Cars
//! leave east at one common speed, spaced by a stride of ticks that keeps every adjacent pair at
//! least the chosen gap apart on the quarter grid:
//!
//! ```text
//! clearance_ticks = ceil(120 · (2·halfMax + gap) / speed)       (all on the quarter grid)
//! stride          = max(gateTicks, clearance_ticks)
//! ```
//!
//! At each admission only the two live heads are read. The time is `max(nextAllowed, earliest
//! ready head)`. One eligible head is admitted as it is (`only`); two under the zipper policy go
//! to the preferred lane (`zipper`), which flips after every admission; two under the arrival
//! policy go to the earlier head (`oldest`, a tie to the start lane). Once both cars of an
//! adjacent pair are on the exit their gap is exact: `speed·(t_follower − t_leader)/120 −
//! h_leader − h_follower`. In braking mode the gap is the convoy reserve's fitted gap
//! ([`crate::convoy`]). The engine's `wcraft::zipper`, held to the same eight receipts.

use crate::contact::Fraction;
use crate::convoy::{self, Car, Mode, Plan as Brake, FOLLOWER, LEADER};

pub const HZ: i64 = 120;
pub const PROFILE: &str = "zipper-gate-v1";
pub const CAR_L: u128 = 262144;
pub const END_BOUND: i64 = 4800;

/// The studio's eight cars at heading east: address and longitudinal half-extent (C013 enclosure).
pub const HALF: [(u128, f64); 8] = [(45, 2.5), (7, 2.75), (3, 2.5), (31, 2.5), (15, 2.75), (63, 2.25), (127, 2.25), (255, 2.25)];
/// The cards the studio's cases carry, lane A then lane B.
pub const CARDS: [[u128; 4]; 2] = [
    [9676571645257672083898648, 29020048841878392412045628, 145077591607669446730842464, 1228275790065163840840794500],
    [67701174063992540873097288, 299815669299764290805039216, 609301416990872307920273560, 2466212213788705668701356224],
];

/// A version 1 rulerpack car card back to `(address, paint)`: `N = K<<64 | X` with
/// `X = 2p ^ L·i` and `K = 2p ^ L·(i^(i+2))`, so `K ^ X = L·(i+2)`.
pub fn open_car(n: u128, l: u128) -> Option<(u128, u128)> {
    let (x, k) = (n & ((1u128 << 64) - 1), n >> 64);
    let z = k ^ x;
    if z % l != 0 || z / l < 2 {
        return None;
    }
    let i = z / l - 2;
    let v = x ^ (l * i);
    if v & 1 != 0 {
        return None;
    }
    Some((i, v >> 1))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    Zipper,
    Arrival,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GapMode {
    Fixed,
    Braking,
}

/// A car in a queue.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Queued {
    pub card: u128,
    pub address: u128,
    pub paint: u128,
    pub ready: i64,
    pub lane: usize,
    pub position: usize,
    /// The flattened index, lane A then lane B.
    pub actor: usize,
    pub hx: f64,
}

/// A merge: the two queues and the gate's settings.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub lanes: [Vec<Queued>; 2],
    pub start_lane: usize,
    pub policy: Policy,
    pub speed: f64,
    pub gate_ticks: i64,
    pub gap_mode: GapMode,
    pub gap: f64,
    pub delay: usize,
    pub wet: f64,
    pub margin: f64,
}

/// The gate's settings without the queues.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gate {
    pub start_lane: usize,
    pub policy: Policy,
    pub speed: f64,
    pub gate_ticks: i64,
    pub gap_mode: GapMode,
    pub gap: f64,
    pub delay: usize,
    pub wet: f64,
    pub margin: f64,
}

impl Default for Gate {
    /// The opening: zipper from lane A, 18 m/s, a 36-tick gate, a fixed 3 m gap.
    fn default() -> Gate {
        Gate { start_lane: 0, policy: Policy::Zipper, speed: 18.0, gate_ticks: 36, gap_mode: GapMode::Fixed, gap: 3.0, delay: 24, wet: 0.0, margin: 2.0 }
    }
}

impl Plan {
    /// Two lanes of `(card, ready)` and the gate; `half` maps address to half-length for cars
    /// outside the studio's eight.
    pub fn new(lanes: [&[(u128, i64)]; 2], gate: Gate, half: &[(u128, f64)]) -> Result<Plan, &'static str> {
        let total = lanes[0].len() + lanes[1].len();
        if lanes[0].len() > 4 || lanes[1].len() > 4 || !(2..=8).contains(&total) {
            return Err("two lanes, up to four cars each, two to eight in all");
        }
        if gate.start_lane > 1 || !(12..=90).contains(&gate.gate_ticks) || !(5.0..=30.0).contains(&gate.speed) || !(0.0..=12.0).contains(&gate.gap) || gate.delay > 120 {
            return Err("gate 12 to 90 ticks, speed 5 to 30, gap 0 to 12, delay 0 to 120, start lane 0 or 1");
        }
        let mut out: [Vec<Queued>; 2] = [Vec::new(), Vec::new()];
        let mut seen = Vec::new();
        let mut actor = 0;
        for (lane, cars) in lanes.iter().enumerate() {
            let mut last = 0;
            for (position, &(card, ready)) in cars.iter().enumerate() {
                if !(0..=240).contains(&ready) || ready < last {
                    return Err("ready ticks are 0 to 240 and nondecreasing within a lane");
                }
                last = ready;
                let (address, paint) = open_car(card, CAR_L).ok_or("not a car card")?;
                if seen.contains(&address) {
                    return Err("addresses are distinct across the fleet");
                }
                seen.push(address);
                let hx = half.iter().chain(HALF.iter()).find(|(a, _)| *a == address).map(|(_, h)| *h).ok_or("no half-length for this address")?;
                out[lane].push(Queued { card, address, paint, ready, lane, position, actor, hx });
                actor += 1;
            }
        }
        Ok(Plan { lanes: out, start_lane: gate.start_lane, policy: gate.policy, speed: gate.speed, gate_ticks: gate.gate_ticks, gap_mode: gate.gap_mode, gap: gate.gap, delay: gate.delay, wet: gate.wet, margin: gate.margin })
    }
    /// The studio's cards with these ready ticks.
    pub fn studio(ready: [&[i64]; 2], gate: Gate) -> Result<Plan, &'static str> {
        let a: Vec<(u128, i64)> = ready[0].iter().enumerate().map(|(i, &t)| (CARDS[0][i], t)).collect();
        let b: Vec<(u128, i64)> = ready[1].iter().enumerate().map(|(i, &t)| (CARDS[1][i], t)).collect();
        Plan::new([&a, &b], gate, &[])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    Only,
    Zipper,
    Oldest,
    Tie,
}

/// One admission.
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub order: usize,
    pub actor: usize,
    pub address: u128,
    pub lane: usize,
    pub position: usize,
    pub ready: i64,
    pub tick: i64,
    pub exit: i64,
    pub wait: i64,
    /// The heads ready at the chosen time, flattened ids in A/B order.
    pub eligible: Vec<usize>,
    pub preferred_before: usize,
    pub slot_open: i64,
    pub arrival_idle: i64,
    pub reason: Reason,
    /// The exact gap to the car admitted before, once both are on the exit.
    pub pair_gap: Option<Fraction>,
}

impl Event {
    pub fn label(&self) -> String {
        format!("{}{}", if self.lane == 0 { 'A' } else { 'B' }, self.position + 1)
    }
}

/// The braking reserve the gap came from, in braking mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrakeNote {
    pub required: f64,
    pub fit: f64,
    pub tail: f64,
}

/// The schedule.
#[derive(Clone, Debug, PartialEq)]
pub struct Receipt {
    pub gap: f64,
    pub half_max: f64,
    pub clearance_ticks: i64,
    pub stride: i64,
    pub end: i64,
    pub brake: Option<BrakeNote>,
    pub events: Vec<Event>,
}

fn q4(v: f64) -> i64 {
    (v * 4.0).round() as i64
}

/// **The admission schedule**: every event with its reason, and the exact gap of each pair.
pub fn run(p: &Plan) -> Receipt {
    let brake = (p.gap_mode == GapMode::Braking).then(|| {
        let r = convoy::run(&Brake { mode: Mode::Coast, lead_tick: 0, delay: p.delay, gap: p.gap, margin: p.margin, top: 30.0, accel: 12.0, leader: Car { speed: p.speed, wet: p.wet, ..LEADER }, follower: Car { speed: p.speed, wet: p.wet, ..FOLLOWER } });
        BrakeNote { required: r.required, fit: r.fitted, tail: r.tail }
    });
    let gap = brake.map_or(p.gap, |b| b.fit);
    let half_max = p.lanes.iter().flatten().map(|c| c.hx).fold(0.0, f64::max);
    let (h4, g4, v4) = (q4(half_max), q4(gap), q4(p.speed));
    let clearance = (HZ * (2 * h4 + g4) + v4 - 1) / v4;
    let stride = p.gate_ticks.max(clearance);
    let mut heads = [0usize, 0usize];
    let (mut next_allowed, mut preferred) = (0i64, p.start_lane);
    let mut events: Vec<Event> = Vec::new();
    let mut prev: Option<(i64, f64)> = None;
    while heads[0] < p.lanes[0].len() || heads[1] < p.lanes[1].len() {
        let live: Vec<&Queued> = (0..2).filter(|&l| heads[l] < p.lanes[l].len()).map(|l| &p.lanes[l][heads[l]]).collect();
        let earliest = live.iter().map(|c| c.ready).min().expect("a live head");
        let tick = next_allowed.max(earliest);
        let eligible: Vec<&Queued> = live.iter().copied().filter(|c| c.ready <= tick).collect();
        let (car, reason) = if eligible.len() == 1 {
            (eligible[0], Reason::Only)
        } else if p.policy == Policy::Zipper {
            (*eligible.iter().find(|c| c.lane == preferred).expect("the preferred head"), Reason::Zipper)
        } else if eligible[0].ready == eligible[1].ready {
            (*eligible.iter().find(|c| c.lane == p.start_lane).expect("the start lane head"), Reason::Tie)
        } else {
            (*eligible.iter().min_by_key(|c| c.ready).expect("two heads"), Reason::Oldest)
        };
        let pair_gap = prev.map(|(t0, h0)| {
            // speed·Δt/120 − h_leader − h_follower, every term on the quarter grid
            let n = q4(p.speed) as i128 * (tick - t0) as i128 - (q4(h0) as i128 + q4(car.hx) as i128) * HZ as i128;
            reduce(n, 4 * HZ as i128)
        });
        events.push(Event { order: events.len(), actor: car.actor, address: car.address, lane: car.lane, position: car.position, ready: car.ready, tick, exit: tick + p.gate_ticks, wait: tick - car.ready, eligible: eligible.iter().map(|c| c.actor).collect(), preferred_before: preferred, slot_open: next_allowed, arrival_idle: (tick - next_allowed).max(0), reason, pair_gap });
        heads[car.lane] += 1;
        preferred = 1 - car.lane;
        next_allowed = tick + stride;
        prev = Some((tick, car.hx));
    }
    let end = END_BOUND.min(240.max(events.last().expect("an event").exit + 120));
    Receipt { gap, half_max, clearance_ticks: clearance, stride, end, brake, events }
}

fn reduce(n: i128, d: i128) -> Fraction {
    let mut a = n.unsigned_abs();
    let mut b = d.unsigned_abs();
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    let g = a.max(1) as i128;
    Fraction { n: n / g, d: d / g }
}

/// Where a car is on the exit at tick `now`: centre zero at its exit, then `speed` east.
pub fn exit_x(speed: f64, exit_tick: i64, now: i64) -> f64 {
    speed * (now - exit_tick) as f64 / HZ as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_opening_admits_every_fifty_seven_ticks() {
        assert_eq!(open_car(CARDS[0][0], CAR_L), Some((45, 140)));
        let p = Plan::studio([&[0, 30, 60, 90], &[0, 30, 60, 90]], Gate::default()).unwrap();
        let r = run(&p);
        assert_eq!((r.half_max, r.clearance_ticks, r.stride, r.end), (2.75, 57, 57, 555));
        assert_eq!(r.events.iter().map(|e| e.tick).collect::<Vec<_>>(), vec![0, 57, 114, 171, 228, 285, 342, 399]);
        assert_eq!(r.events[1].pair_gap, Some(Fraction { n: 33, d: 10 }));
        assert_eq!(r.events[7].label(), "B4");
        assert_eq!(exit_x(18.0, 36, 156), 18.0);
    }
}
