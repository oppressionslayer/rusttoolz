//! **The braking reserve between two cars on one lane**, the studio's `convoy-brake-v1`.
//!
//! Two cars face east under the one tick law ([`motion::step`]), each with its own starting
//! speed and wetness, sharing a top speed and an acceleration. Before its brake tick a car
//! coasts (code 4) or accelerates (code 5); from its brake tick on it holds the brake (code 13).
//! The follower's brake comes `delay` ticks after the leader's. From the two free travel traces
//! the closure `C[k] = F[k] − L[k]` says how much of the starting gap the follower has eaten, and
//! the reserve is the gap that keeps them apart for good:
//!
//! ```text
//! tail     = follower speed at 600 · q / (120 · (1 − q)),   q = 1 − (3 − 1.3·wet) / 120
//! required = margin + max(0, max C[0..600], C[600] + tail)
//! fitted   = ceil((required + 1e-9) · 4) / 4
//! ```
//!
//! The tail is every metre the follower still travels after the five visible seconds, braking
//! all the way. First contact is the sweep of the follower's clearance against the leader's,
//! tick by tick, with `offset = gap + hx_leader + hx_follower`. The engine's `wcraft::convoy`,
//! held to the same eight receipts. Detection only: no impulse.

use crate::contact::{self, Fraction, Kind, Motion, Obstacle};
use crate::motion::{self, Handling, Pose};

fn tick(p: &Pose, code: u8, h: &Handling) -> Pose {
    motion::step(p, code as i64, h, DT)
}

pub const HZ: f64 = 120.0;
pub const DT: f64 = 1.0 / HZ;
pub const TICKS: usize = 600;
pub const PROFILE: &str = "convoy-brake-v1";
pub const COAST: u8 = 4;
pub const THROTTLE: u8 = 5;
pub const BRAKE: u8 = 13;

/// A car on the lane: its address, clearance half-sizes, starting speed and wetness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Car {
    pub address: u128,
    pub hx: f64,
    pub hz: f64,
    pub speed: f64,
    pub wet: f64,
}

/// The two studio cars at heading east: leader 7, follower 45, with the C013 clearances.
pub const LEADER: Car = Car { address: 7, hx: 2.75, hz: 1.25, speed: 18.0, wet: 0.0 };
pub const FOLLOWER: Car = Car { address: 45, hx: 2.5, hz: 1.25, speed: 18.0, wet: 0.0 };

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Coast,
    Throttle,
}

/// A braking plan.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plan {
    pub mode: Mode,
    pub lead_tick: usize,
    pub delay: usize,
    pub gap: f64,
    pub margin: f64,
    pub top: f64,
    pub accel: f64,
    pub leader: Car,
    pub follower: Car,
}

impl Default for Plan {
    /// The default plan: 18 m/s, coast, brakes at 60 and 120, a 4 m gap and a 2 m margin.
    fn default() -> Plan {
        Plan { mode: Mode::Coast, lead_tick: 60, delay: 60, gap: 4.0, margin: 2.0, top: 30.0, accel: 12.0, leader: LEADER, follower: FOLLOWER }
    }
}

/// One car's free travel from `x = 0`: distance and speed at every tick, 0 to [`TICKS`].
pub fn travel(car: &Car, top: f64, accel: f64, brake_tick: usize, mode: Mode) -> (Vec<f64>, Vec<f64>) {
    let h = Handling { speed: top, accel, turn: 1.2, grip: 1.0, wet: car.wet };
    let mut p = Pose::at(0.0, 0.0, 0.0);
    p.speed = car.speed;
    p.vx = car.speed;
    let (mut xs, mut vs) = (vec![0.0], vec![car.speed]);
    let free = if mode == Mode::Throttle { THROTTLE } else { COAST };
    for k in 0..TICKS {
        p = tick(&p, if k >= brake_tick { BRAKE } else { free }, &h);
        xs.push(p.x);
        vs.push(p.speed);
    }
    (xs, vs)
}

/// The first contact between the two clearances, if the drive has one.
#[derive(Clone, Debug, PartialEq)]
pub struct First {
    pub tick: usize,
    pub kind: Kind,
    pub fraction: Fraction,
    pub part: f64,
    pub at_tick: f64,
    pub seconds: f64,
    pub normals: Vec<(i32, i32)>,
    pub motion: Motion,
}

/// The receipt.
#[derive(Clone, Debug, PartialEq)]
pub struct Receipt {
    pub plan: Plan,
    pub leader_x: Vec<f64>,
    pub leader_speed: Vec<f64>,
    pub follower_x: Vec<f64>,
    pub follower_speed: Vec<f64>,
    pub closure: Vec<f64>,
    pub tail: f64,
    pub peak: f64,
    pub closest_tick: usize,
    pub minimum_gap: f64,
    pub required: f64,
    pub fitted: f64,
    pub contact: Option<First>,
}

/// **The reserve and the first contact** for a plan.
pub fn run(plan: &Plan) -> Receipt {
    let (lx, lv) = travel(&plan.leader, plan.top, plan.accel, plan.lead_tick, plan.mode);
    let (fx, fv) = travel(&plan.follower, plan.top, plan.accel, plan.lead_tick + plan.delay, plan.mode);
    let closure: Vec<f64> = fx.iter().zip(&lx).map(|(f, l)| f - l).collect();
    let b = 3.0 - 1.3 * plan.follower.wet;
    let q = 1.0 - b / HZ;
    let tail = fv[TICKS] * q / (HZ * (1.0 - q));
    let (mut closest, mut peak) = (0usize, closure[0]);
    for (k, &c) in closure.iter().enumerate() {
        if c > peak {
            peak = c;
            closest = k;
        }
    }
    let required = plan.margin + 0.0f64.max(peak).max(closure[TICKS] + tail);
    let fitted = ((required + 1e-9) * 4.0).ceil() / 4.0;
    let offset = plan.gap + plan.leader.hx + plan.follower.hx;
    let solid = Obstacle { id: plan.leader.address, x0: -plan.leader.hx, x1: plan.leader.hx, z0: -plan.leader.hz, z1: plan.leader.hz };
    let mut first = None;
    for k in 0..TICKS {
        let m = Motion { x: fx[k] - lx[k] - offset, z: 0.0, dx: (fx[k + 1] - fx[k]) - (lx[k + 1] - lx[k]), dz: 0.0, hx: plan.follower.hx, hz: plan.follower.hz };
        let hit = contact::sweep(&m, std::slice::from_ref(&solid));
        if hit.kind != Kind::Clear {
            let part = hit.fraction.value();
            first = Some(First { tick: k, kind: hit.kind, fraction: hit.fraction, part, at_tick: k as f64 + part, seconds: (k as f64 + part) / HZ, normals: hit.normals, motion: m });
            break;
        }
    }
    Receipt { plan: *plan, leader_x: lx, leader_speed: lv, follower_x: fx, follower_speed: fv, closure, tail, peak, closest_tick: closest, minimum_gap: plan.gap - peak, required, fitted, contact: first }
}

impl Receipt {
    /// The front-to-back gap at tick `k`.
    pub fn gap_at(&self, k: usize) -> f64 {
        self.plan.gap - self.closure[k]
    }
    /// The reserve still needed at tick `k`: the larger of the closures still to come and the
    /// terminal closure plus the tail, less the closure so far, never below zero, plus the margin.
    pub fn reserve_at(&self, k: usize) -> f64 {
        let ahead = self.closure[k..].iter().cloned().fold(f64::MIN, f64::max).max(self.closure[TICKS] + self.tail);
        self.plan.margin + 0.0f64.max(ahead - self.closure[k])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_plan_fits_to_six_and_a_quarter() {
        let r = run(&Plan::default());
        assert!((r.required - 6.108417666118766).abs() < 1e-9, "{}", r.required);
        assert_eq!(r.fitted, 6.25);
        let c = r.contact.clone().unwrap();
        assert_eq!((c.tick, c.fraction.n, c.fraction.d), (235, 9491405770, 109221290427));
        assert!((r.reserve_at(0) - r.required).abs() < 1e-12);
        assert_eq!(r.gap_at(0), 4.0);
    }
}
