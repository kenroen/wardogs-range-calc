//! Ballistic flight simulation and firing solution solver for WARDOGS artillery.
//!
//! Models trajectory integration with speed-dependent aerodynamic drag, gravity,
//! sight HUD RNG simulation, and terrain line-of-sight collision checks, matching
//! `wardogs.zone/calculators/artillery`.

use std::sync::OnceLock;

/// Gravitational acceleration (m/s^2).
const G: f64 = 9.8;
/// Maximum simulation time (seconds).
const MAX_TIME_S: f64 = 240.0;
/// Muzzle height above ground (metres) used for terrain collision checks.
pub const MUZZLE_M: f64 = 2.0;
/// Conversion from degrees to radians.
const DEG_TO_RAD: f64 = std::f64::consts::PI / 180.0;
/// Conversion from radians to degrees.
const RAD_TO_DEG: f64 = 180.0 / std::f64::consts::PI;

/// In-game HUD RNG sight estimator parameters (SPH-2 sight RNG column).
#[derive(Clone, Copy, Debug)]
pub struct HudSpec {
    pub velocity: f64,
    pub gravity: f64,
    pub drag: f64,
    pub step_s: f64,
    pub degrees_per_mil: f64,
    pub offset: f64,
    pub height_scale: f64,
}

/// Weapon physical parameters.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct WeaponSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub calibre: &'static str,
    pub reload_s: f64,
    pub velocity: f64,
    pub drag: f64,
    pub drag_curve: Option<&'static [(f64, f64)]>,
    pub step_s: f64,
    pub degrees_per_mil: f64,
    pub offset: f64,
    pub min_mil: f64,
    pub max_mil: f64,
    pub hud: Option<HudSpec>,
}

pub const SPH2_DRAG_CURVE: [(f64, f64); 21] = [
    (0.0, 0.71),
    (30.0, 0.69),
    (61.0, 0.66),
    (91.0, 0.64),
    (122.0, 0.65),
    (150.0, 0.66),
    (183.0, 0.68),
    (213.0, 0.72),
    (244.0, 0.8),
    (274.0, 0.92),
    (300.0, 1.1),
    (335.0, 1.25),
    (366.0, 1.33),
    (396.0, 1.39),
    (427.0, 1.44),
    (450.0, 1.48),
    (488.0, 1.5),
    (518.0, 1.54),
    (549.0, 1.57),
    (579.0, 1.58),
    (600.0, 1.6),
];

pub const SPH2_SPEC: WeaponSpec = WeaponSpec {
    id: "sph-2",
    name: "SPH-2",
    calibre: "155 mm HE",
    reload_s: 27.0,
    velocity: 300.0,
    drag: 659e-6,
    drag_curve: Some(&SPH2_DRAG_CURVE),
    step_s: 0.02,
    degrees_per_mil: 0.0625,
    offset: 0.0,
    min_mil: 30.0,
    max_mil: 1400.0,
    hud: Some(HudSpec {
        velocity: 222.3,
        gravity: 9.84,
        drag: 2743e-7,
        step_s: 0.02,
        degrees_per_mil: 0.053,
        offset: 140.6,
        height_scale: 0.879,
    }),
};

pub const MORTAR_SPEC: WeaponSpec = WeaponSpec {
    id: "mortar",
    name: "L81 MORTAR",
    calibre: "81 mm",
    reload_s: 6.5,
    velocity: 93.5,
    drag: 26e-5,
    drag_curve: None,
    step_s: 0.01,
    degrees_per_mil: 0.0372,
    offset: 1417.0,
    min_mil: 120.0,
    max_mil: 950.0,
    hud: None,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrajectoryArc {
    Low,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FiringSolution {
    pub arc: TrajectoryArc,
    /// Barrel elevation angle in degrees.
    pub degrees: f64,
    /// Sight elevation in mils (gun's own sight scale).
    pub mil: f64,
    /// Elevation-corrected sight range on level ground (metres).
    pub sight_range_m: f64,
    /// What the SPH-2 in-game sight RNG column displays when `mil` is dialed, if available.
    pub hud_range_m: Option<f64>,
    /// Time of flight in seconds.
    pub flight_time_s: f64,
    /// Distance (metres from gun) where terrain blocks this arc before the target, if obstructed.
    pub blocked_at_m: Option<f64>,
}

#[derive(Clone, Copy, Debug)]
struct TrajPoint {
    x: f64,
    z: f64,
    t: f64,
}

#[derive(Clone, Debug)]
pub struct Trajectory {
    pub angle_rad: f64,
    points: Vec<TrajPoint>,
}

fn interp_drag_curve(curve: &[(f64, f64)], speed: f64) -> f64 {
    let n = curve.len();
    if n == 0 {
        return 1.0;
    }
    if speed <= curve[0].0 {
        return curve[0].1;
    }
    if speed >= curve[n - 1].0 {
        return curve[n - 1].1;
    }
    let mut lo = 0;
    let mut hi = n - 1;
    while hi - lo > 1 {
        let mid = (lo + hi) >> 1;
        if curve[mid].0 <= speed {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let (r, s) = curve[lo];
    let (i, l) = curve[hi];
    if i > r {
        s + (l - s) * (speed - r) / (i - r)
    } else {
        s
    }
}

struct SimState<'a> {
    x: f64,
    z: f64,
    t: f64,
    vx: f64,
    vz: f64,
    dt: f64,
    steps: usize,
    gravity: f64,
    drag: f64,
    curve: Option<&'a [(f64, f64)]>,
}

impl<'a> SimState<'a> {
    fn new(velocity: f64, gravity: f64, drag: f64, curve: Option<&'a [(f64, f64)]>, dt: f64, angle_rad: f64) -> Self {
        Self {
            x: 0.0,
            z: 0.0,
            t: 0.0,
            vx: velocity * angle_rad.cos(),
            vz: velocity * angle_rad.sin(),
            dt,
            steps: 0,
            gravity,
            drag,
            curve,
        }
    }

    fn flying(&self) -> bool {
        self.t < MAX_TIME_S && self.z > -1000.0 && self.vx > 0.0
    }

    fn step(&mut self) {
        let speed = self.vx.hypot(self.vz);
        let mult = self.curve.map_or(1.0, |c| interp_drag_curve(c, speed));
        let a = self.drag * mult * speed;
        self.x += self.vx * self.dt;
        self.z += self.vz * self.dt;
        self.vx -= a * self.vx * self.dt;
        self.vz -= (a * self.vz + self.gravity) * self.dt;
        self.steps += 1;
        self.t = self.steps as f64 * self.dt;
    }
}

impl Trajectory {
    pub fn simulate(spec: &WeaponSpec, angle_rad: f64) -> Self {
        let mut sim = SimState::new(spec.velocity, G, spec.drag, spec.drag_curve, spec.step_s, angle_rad);
        let sample_every = (0.1 / sim.dt).round().max(1.0) as usize;

        let mut points = Vec::with_capacity(2400);
        points.push(TrajPoint { x: 0.0, z: 0.0, t: 0.0 });

        while sim.flying() {
            sim.step();
            if sim.steps.is_multiple_of(sample_every) {
                points.push(TrajPoint {
                    x: sim.x,
                    z: sim.z,
                    t: sim.t,
                });
            }
        }
        if !sim.steps.is_multiple_of(sample_every) {
            points.push(TrajPoint {
                x: sim.x,
                z: sim.z,
                t: sim.t,
            });
        }

        Self { angle_rad, points }
    }

    /// Samples the height (z) and time (t) at a horizontal distance (x).
    fn sample_at_x(&self, target_x: f64) -> Option<TrajPoint> {
        let n = self.points.len();
        if n < 2 || target_x < 0.0 || target_x > self.points[n - 1].x {
            return None;
        }

        let mut lo = 0;
        let mut hi = n - 1;
        while hi - lo > 1 {
            let mid = (lo + hi) >> 1;
            if self.points[mid].x <= target_x {
                lo = mid;
            } else {
                hi = mid;
            }
        }

        let a = &self.points[lo];
        let b = &self.points[hi];
        let dx = b.x - a.x;
        let s = if dx > 0.0 { (target_x - a.x) / dx } else { 0.0 };

        Some(TrajPoint {
            x: target_x,
            z: a.z + (b.z - a.z) * s,
            t: a.t + (b.t - a.t) * s,
        })
    }

    /// Finds where the trajectory falls back through z_target (e.g. z = 0 for level ground).
    fn zero_crossing_x(&self, z_target: f64) -> Option<f64> {
        for i in 1..self.points.len() {
            let a = &self.points[i - 1];
            let b = &self.points[i];
            let da = a.z - z_target;
            let db = b.z - z_target;
            if da >= 0.0 && db <= 0.0 {
                let s = if (da - db).abs() > 1e-12 { da / (da - db) } else { 0.0 };
                return Some(a.x + (b.x - a.x) * s);
            }
        }
        None
    }

    /// Traces the trajectory against a terrain height function `terrain_rel_z(dist_m)` (relative to muzzle height).
    /// Returns the distance `dist_m` where the shell strikes terrain early, or `None` if clear.
    pub fn trace_terrain<F>(&self, from_m: f64, to_m: f64, step_m: f64, terrain_rel_z: F) -> Option<f64>
    where
        F: Fn(f64) -> Option<f64>,
    {
        let max_x = self.points.last().map_or(0.0, |p| p.x);
        let a = to_m.min(max_x);
        let s = from_m.max(0.0);
        if a <= s {
            return None;
        }
        let step = step_m.max(1.0);
        let clearance = |dist: f64| -> f64 {
            let Some(pt) = self.sample_at_x(dist) else {
                return f64::INFINITY;
            };
            let Some(terr) = terrain_rel_z(dist) else {
                return f64::INFINITY;
            };
            pt.z - terr
        };

        if clearance(s) <= 0.0 {
            return Some(s);
        }

        let mut prev = s;
        let mut curr_step = s + step;
        loop {
            let t = curr_step.min(a);
            if clearance(t) <= 0.0 {
                let mut lo = prev;
                let mut hi = t;
                for _ in 0..10 {
                    let mid = (lo + hi) * 0.5;
                    if clearance(mid) > 0.0 {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                return Some(hi);
            }
            prev = t;
            if t >= a {
                break;
            }
            curr_step += step;
        }
        None
    }
}

/// Evaluates exact step-by-step trajectory height and time at `target_x`, matching `wardogs.zone` solver.
fn evaluate_exact_at_x(spec: &WeaponSpec, angle_rad: f64, target_x: f64) -> Option<(f64, f64)> {
    let mut sim = SimState::new(spec.velocity, G, spec.drag, spec.drag_curve, spec.step_s, angle_rad);
    while sim.flying() {
        let prev_x = sim.x;
        let prev_z = sim.z;
        let prev_t = sim.t;
        let prev_vx = sim.vx;
        let prev_vz = sim.vz;
        sim.step();
        if sim.x >= target_x {
            let frac = if prev_vx > 0.0 { (target_x - prev_x) / (sim.x - prev_x) } else { 0.0 };
            return Some((prev_z + prev_vz * sim.dt * frac, prev_t + sim.dt * frac));
        }
    }
    None
}

/// Computes the SPH-2 in-game sight HUD RNG readout for a dialed mil value and gun altitude.
pub fn compute_hud_range(spec: &WeaponSpec, mil_rounded: f64, gun_asl_m: Option<f64>, hud_plane_m: Option<f64>) -> Option<f64> {
    let hud = spec.hud?;
    let z_offset = match (gun_asl_m, hud_plane_m) {
        (Some(gun_m), Some(plane_m)) => gun_m - plane_m,
        _ => 0.0,
    };
    let target_z = -hud.height_scale * z_offset;
    let angle_rad = ((mil_rounded + hud.offset) * hud.degrees_per_mil) * DEG_TO_RAD;
    let mut sim = SimState::new(hud.velocity, hud.gravity, hud.drag, None, hud.step_s, angle_rad);

    while sim.flying() {
        let prev_x = sim.x;
        let prev_z = sim.z;
        sim.step();
        if prev_z >= target_z && sim.z < target_z {
            let r = (prev_z - target_z) / (prev_z - sim.z);
            return Some(prev_x + (sim.x - prev_x) * r);
        }
    }
    None
}

/// Precomputed coarse trajectory families for fast solution lookup.
static SPH2_BASE_TRAJECTORIES: OnceLock<Vec<Trajectory>> = OnceLock::new();
static MORTAR_BASE_TRAJECTORIES: OnceLock<Vec<Trajectory>> = OnceLock::new();

fn get_base_trajectories(spec: &WeaponSpec) -> &'static [Trajectory] {
    match spec.id {
        "sph-2" => SPH2_BASE_TRAJECTORIES.get_or_init(|| precompute_family(&SPH2_SPEC)),
        _ => MORTAR_BASE_TRAJECTORIES.get_or_init(|| precompute_family(&MORTAR_SPEC)),
    }
}

fn precompute_family(spec: &WeaponSpec) -> Vec<Trajectory> {
    let min_deg = (spec.min_mil + spec.offset) * spec.degrees_per_mil;
    let max_deg = (spec.max_mil + spec.offset) * spec.degrees_per_mil;

    let mut list = Vec::new();
    let mut deg = min_deg;
    while deg <= max_deg + 1e-9 {
        list.push(Trajectory::simulate(spec, deg * DEG_TO_RAD));
        deg += 0.5;
    }
    if list.last().is_none_or(|last| (last.angle_rad * RAD_TO_DEG - max_deg).abs() > 1e-9) {
        list.push(Trajectory::simulate(spec, max_deg * DEG_TO_RAD));
    }
    list
}

/// Returns `(min_range_m, max_range_m)` across the weapon's sight table for a given relative elevation `delta_z_m`.
pub fn range_band(spec: &WeaponSpec, delta_z_m: f64) -> (f64, f64) {
    let family = get_base_trajectories(spec);
    let mut min_r = f64::INFINITY;
    let mut max_r = 0.0_f64;
    for traj in family {
        if let Some(x) = traj.zero_crossing_x(delta_z_m)
            && x >= 0.0
        {
            if x < min_r {
                min_r = x;
            }
            if x > max_r {
                max_r = x;
            }
        }
    }
    (if min_r.is_finite() { min_r } else { 0.0 }, max_r)
}

#[derive(Clone, Copy)]
struct BracketSample {
    angle_rad: f64,
    above: bool,
    z: f64,
    t: f64,
    short: bool,
}

/// Solves firing solutions for a weapon to hit a target at `distance_m` and relative `delta_z_m`.
pub fn solve_firing_solutions(spec: &WeaponSpec, distance_m: f64, delta_z_m: f64) -> Vec<FiringSolution> {
    if !distance_m.is_finite() || !delta_z_m.is_finite() || distance_m <= 0.0 {
        return Vec::new();
    }

    let family = get_base_trajectories(spec);
    if family.is_empty() {
        return Vec::new();
    }

    let eval_sample = |angle_rad: f64, precomp: Option<&Trajectory>| -> BracketSample {
        let sampled = match precomp {
            Some(traj) => traj.sample_at_x(distance_m).map(|pt| (pt.z, pt.t)),
            None => evaluate_exact_at_x(spec, angle_rad, distance_m),
        };
        match sampled {
            Some((z, t)) => BracketSample {
                angle_rad,
                above: z > delta_z_m,
                z,
                t,
                short: false,
            },
            None => BracketSample {
                angle_rad,
                above: false,
                z: 0.0,
                t: 0.0,
                short: true,
            },
        }
    };

    let refine = |e: BracketSample, t: BracketSample| -> (f64, f64) {
        let mut n = e;
        let mut a = t;
        let mut r = if e.short { t } else { e };
        let mut best_err = (r.z - delta_z_m).abs();

        for _ in 0..12 {
            if best_err < 0.05 {
                break;
            }
            let ez = n.z - delta_z_m;
            let tz = a.z - delta_z_m;
            let span = a.angle_rad - n.angle_rad;
            let raw_ang = if n.short || a.short || (ez - tz).abs() < 1e-12 {
                n.angle_rad + span * 0.5
            } else {
                n.angle_rad + (ez / (ez - tz)) * span
            };
            let margin = 0.02 * span;
            let clamped_ang = raw_ang.clamp(n.angle_rad + margin, a.angle_rad - margin);
            let p = eval_sample(clamped_ang, None);
            if !p.short {
                let err = (p.z - delta_z_m).abs();
                if err < best_err {
                    r = p;
                    best_err = err;
                }
            }
            if p.above == n.above {
                n = p;
            } else {
                a = p;
            }
        }
        (r.angle_rad, r.t)
    };

    let build_solution = |angle_rad: f64, tof: f64, arc: TrajectoryArc| -> FiringSolution {
        let degrees = angle_rad * RAD_TO_DEG;
        let mil = degrees / spec.degrees_per_mil - spec.offset;
        let traj = Trajectory::simulate(spec, angle_rad);
        let sight_range = traj.zero_crossing_x(0.0).unwrap_or(distance_m);
        FiringSolution {
            arc,
            degrees,
            mil,
            sight_range_m: sight_range,
            hud_range_m: None,
            flight_time_s: tof,
            blocked_at_m: None,
        }
    };

    let mut low_sol: Option<FiringSolution> = None;
    let mut high_sol: Option<FiringSolution> = None;
    let mut prev: Option<BracketSample> = None;

    for traj in family {
        let curr = eval_sample(traj.angle_rad, Some(traj));
        if let Some(p) = prev
            && p.above != curr.above
        {
            let (ang, tof) = refine(p, curr);
            if p.above {
                high_sol = Some(build_solution(ang, tof, TrajectoryArc::High));
            } else {
                low_sol = Some(build_solution(ang, tof, TrajectoryArc::Low));
            }
        }
        prev = Some(curr);
    }

    let mut results = Vec::with_capacity(2);
    if let Some(low) = low_sol {
        results.push(low);
    }
    if let Some(high) = high_sol {
        results.push(high);
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_case_sph2_and_mortar() {
        // User test case: x97.99, y109.57 (ME) -> x95.62, y104.98 (Target) on Bakurani
        // dist = 516.584m, gun_asl = -862.9375m, target_asl = -786.5m, rise = +76.4375m
        let dist = 516.58397;
        let rise = 76.4375;
        let gun_asl = Some(-862.9375);
        let hud_plane = Some(-863.0);

        let mortar_sols = solve_firing_solutions(&MORTAR_SPEC, dist, rise);
        assert_eq!(mortar_sols.len(), 1);
        let m = &mortar_sols[0];
        assert_eq!(m.arc, TrajectoryArc::High);
        assert!((m.mil - 374.8).abs() < 0.1, "expected 374.8 mil, got {:.1}", m.mil);
        assert!((m.flight_time_s - 15.6).abs() < 0.1, "expected 15.6s, got {:.1}", m.flight_time_s);

        let sph2_sols = solve_firing_solutions(&SPH2_SPEC, dist, rise);
        assert_eq!(sph2_sols.len(), 2);
        let low = &sph2_sols[0];
        let high = &sph2_sols[1];

        assert_eq!(low.arc, TrajectoryArc::Low);
        assert!((low.mil - 167.0).abs() < 0.1, "expected 167.0 mil, got {:.1}", low.mil);
        assert!((low.flight_time_s - 2.1).abs() < 0.1, "expected 2.1s, got {:.1}", low.flight_time_s);
        let low_hud = compute_hud_range(&SPH2_SPEC, (low.mil * 10.0).round() / 10.0, gun_asl, hud_plane).unwrap();
        assert_eq!(low_hud.round() as i64, 1859);

        assert_eq!(high.arc, TrajectoryArc::High);
        assert!((high.mil - 1342.8).abs() < 0.1, "expected 1342.8 mil, got {:.1}", high.mil);
        assert!((high.flight_time_s - 36.9).abs() < 0.1, "expected 36.9s, got {:.1}", high.flight_time_s);
        let high_hud = compute_hud_range(&SPH2_SPEC, (high.mil * 10.0).round() / 10.0, gun_asl, hud_plane).unwrap();
        assert_eq!(high_hud.round() as i64, 984);
    }

    #[test]
    fn test_sph2_flat_ground_solution() {
        let dist = 2344.6;
        let sols = solve_firing_solutions(&SPH2_SPEC, dist, 0.0);
        assert_eq!(sols.len(), 2, "SPH-2 should have low and high arc at 2345m");

        let low = &sols[0];
        assert_eq!(low.arc, TrajectoryArc::Low);
        assert!((low.sight_range_m - 2345.0).abs() < 2.0);

        let high = &sols[1];
        assert_eq!(high.arc, TrajectoryArc::High);
        assert!((high.sight_range_m - 2345.0).abs() < 2.0);
    }

    #[test]
    fn test_mortar_firing_solution() {
        let dist = 500.0;
        let sols = solve_firing_solutions(&MORTAR_SPEC, dist, 0.0);
        assert_eq!(sols.len(), 1, "mortar should have high arc solution");
        let sol = &sols[0];
        assert!((sol.sight_range_m - 500.0).abs() < 2.0);
        assert!((sol.mil - 443.0).abs() < 5.0);
    }
}
