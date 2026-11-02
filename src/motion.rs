//! **The one tick law of speed**, shared by the site, the engine (`wcraft::longitudinal_step`)
//! and here: accelerate, then drag the new speed, then clamp, three statements at 120 Hz. Below the
//! cap a constant throttle approaches `throttle·accel·(1/drag − dt)` as `q^n`, `q = 1 − drag·dt`.

pub const HZ: f64 = 120.0;
pub const DT: f64 = 1.0 / HZ;

#[inline]
pub fn longitudinal_step(speed: f64, throttle: f64, accel: f64, drag: f64, dt: f64, reverse_cap: f64, forward_cap: f64) -> f64 {
    let mut next = speed;
    next += throttle * accel * dt;
    next -= next * drag * dt;
    next.clamp(-reverse_cap, forward_cap)
}

/// The studio's drag: `3 − 1.3·wet` braking, `0.7` coasting.
pub fn drag_of(brake: bool, wet: f64) -> f64 {
    if brake { 3.0 - 1.3 * wet } else { 0.7 }
}

pub fn settles_at(throttle: f64, accel: f64, drag: f64, dt: f64) -> f64 {
    throttle * accel * (1.0 / drag - dt)
}

/// `v_n = v_inf + (v_0 − v_inf)·q^n`, below the cap.
pub fn closed_form(v0: f64, throttle: f64, accel: f64, drag: f64, n: u32, dt: f64) -> f64 {
    let q = 1.0 - drag * dt;
    let v_inf = settles_at(throttle, accel, drag, dt);
    v_inf + (v0 - v_inf) * q.powi(n as i32)
}

/// Signed distance after `n` ticks along a fixed heading, the position taking the new speed
/// each step.
pub fn distance(v0: f64, throttle: f64, accel: f64, drag: f64, n: u32, dt: f64) -> f64 {
    let q = 1.0 - drag * dt;
    let v_inf = settles_at(throttle, accel, drag, dt);
    dt * (n as f64 * v_inf + (v0 - v_inf) * q * (1.0 - q.powi(n as i32)) / (1.0 - q))
}

/// The first tick full throttle from rest reaches `speed_cap`, within `limit`.
pub fn cap_tick(speed_cap: f64, accel: f64, drag: f64, dt: f64, limit: u32) -> Option<u32> {
    let mut v = 0.0;
    for n in 1..=limit {
        v = longitudinal_step(v, 1.0, accel, drag, dt, 0.45 * speed_cap, speed_cap);
        if v >= speed_cap {
            return Some(n);
        }
    }
    None
}

/// The studio's input code `(throttle + 1) + 3·(steer + 1) + 9·brake` as `(throttle, steer, brake)`.
pub fn decode_input(code: i64) -> (f64, f64, bool) {
    ((code % 3 - 1) as f64, ((code / 3) % 3 - 1) as f64, code / 9 == 1)
}

/// The ten fields of a flat-ground pose, as a checkpoint holds them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub x: f64,
    pub z: f64,
    pub yaw: f64,
    pub speed: f64,
    pub vx: f64,
    pub vz: f64,
    pub steer_vis: f64,
    pub lean: f64,
    pub squat: f64,
    pub braking: bool,
}

impl Pose {
    pub fn at(x: f64, z: f64, yaw: f64) -> Pose {
        Pose { x, z, yaw, speed: 0.0, vx: 0.0, vz: 0.0, steer_vis: 0.0, lean: 0.0, squat: 0.0, braking: false }
    }
}

/// The five handling settings the studio exposes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Handling {
    pub speed: f64,
    pub accel: f64,
    pub turn: f64,
    pub grip: f64,
    pub wet: f64,
}

impl Default for Handling {
    /// The opening handling: 30 m/s, 12 m/s², 1.2 rad/s, rigid grip, dry.
    fn default() -> Self {
        Handling { speed: 30.0, accel: 12.0, turn: 1.2, grip: 1.0, wet: 0.0 }
    }
}

/// An angle into `(-π, π]`, as the engine's `wrap`.
pub fn wrap(a: f64) -> f64 {
    let mut a = (a + std::f64::consts::PI) % (2.0 * std::f64::consts::PI);
    if a < 0.0 {
        a += 2.0 * std::f64::consts::PI;
    }
    a - std::f64::consts::PI
}

/// **One tick of the whole flat-ground law**: accelerate, drag, clamp, turn, the cosmetics, blend
/// the velocity, move. The engine's `step_drive` on flat ground, bit for bit.
pub fn step(p: &Pose, code: i64, h: &Handling, dt: f64) -> Pose {
    let (throttle, steer, brake) = decode_input(code);
    let mut n = *p;
    let grip = if h.grip >= 1.0 { h.grip } else { h.grip - 0.055 * h.wet };
    let top = h.speed * if throttle < 0.0 { 0.45 } else { 1.0 };
    n.speed = longitudinal_step(n.speed, throttle, h.accel, drag_of(brake, h.wet), dt, h.speed * 0.45, top);
    let turn = h.turn * steer * (n.speed.abs() / 5.0).min(1.0) * if n.speed < 0.0 { -1.0 } else { 1.0 } * if brake { 1.5 } else { 1.0 };
    n.yaw = wrap(n.yaw + turn * dt);
    let (k, sp) = ((dt * 8.0).min(1.0), (n.speed.abs() / 22.0).min(1.0));
    n.steer_vis += (steer * 0.5 - n.steer_vis) * k;
    n.lean += (steer * sp * 0.11 * if h.turn > 2.2 { 1.0 } else { 0.6 } - n.lean) * (dt * 5.0).min(1.0);
    n.squat += ((if brake { -0.05 } else { throttle * 0.035 * (1.0 - sp) }) - n.squat) * (dt * 6.0).min(1.0);
    n.braking = brake || throttle < 0.0;
    let (fx, fz) = (n.yaw.cos(), n.yaw.sin());
    let g = grip.powf(dt * 60.0);
    n.vx = n.vx * g + fx * n.speed * (1.0 - g);
    n.vz = n.vz * g + fz * n.speed * (1.0 - g);
    if grip >= 1.0 {
        n.vx = fx * n.speed;
        n.vz = fz * n.speed;
    }
    n.x += n.vx * dt;
    n.z += n.vz * dt;
    n
}
