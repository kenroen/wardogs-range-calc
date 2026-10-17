//! Ballistic flight simulation and firing solution solver for WARDOGS artillery.
//!
//! Models trajectory integration with aerodynamic drag and gravity, matching
//! the game's artillery calibration and community ballistics tables.

use std::sync::OnceLock;

/// Gravitational acceleration (m/s^2).
const G: f64 = 9.8;
/// Simulation time step (seconds).
const DT: f64 = 0.01;
/// Maximum simulation steps (240 seconds).
const MAX_STEPS: usize = 24_000;
/// Conversion from degrees to radians.
const DEG_TO_RAD: f64 = std::f64::consts::PI / 180.0;

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
    pub degrees_per_mil: f64,
    pub offset: f64,
    pub min_mil: f64,
    pub max_mil: f64,
}

pub const SPH2_SPEC: WeaponSpec = WeaponSpec {
    id: "sph-2",
    name: "SPH-2",
    calibre: "155 mm HE",
    reload_s: 27.0,
    velocity: 220.0,
    drag: 26e-5,
    degrees_per_mil: 0.0529,
    offset: 143.7,
    min_mil: 0.0,
    max_mil: 1400.0,
};

pub const MORTAR_SPEC: WeaponSpec = WeaponSpec {
    id: "mortar",
    name: "L81 MORTAR",
    calibre: "81 mm",
    reload_s: 6.5,
    velocity: 93.5,
    drag: 26e-5,
    degrees_per_mil: 0.0372,
    offset: 1417.0,
    min_mil: 120.0,
    max_mil: 950.0,
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
    /// Sight elevation in mils.
    pub mil: f64,
    /// Elevation-corrected sight range to dial on the gun's RNG scale (metres).
    pub sight_range_m: f64,
    /// Time of flight in seconds.
    pub flight_time_s: f64,
}

#[derive(Clone, Copy, Debug)]
struct TrajPoint {
    x: f64,
    z: f64,
    t: f64,
}

#[derive(Clone, Debug)]
struct Trajectory {
    degrees: f64,
    points: Vec<TrajPoint>,
}

impl Trajectory {
    fn simulate(spec: &WeaponSpec, degrees: f64) -> Self {
        let rad = degrees * DEG_TO_RAD;
        let mut vx = spec.velocity * rad.cos();
        let mut vz = spec.velocity * rad.sin();
        let mut x = 0.0;
        let mut z = 0.0;

        let mut points = Vec::with_capacity(2400);
        points.push(TrajPoint { x: 0.0, z: 0.0, t: 0.0 });

        for step in 1..=MAX_STEPS {
            if z <= -1000.0 || vx <= 0.0 {
                break;
            }
            let speed = (vx * vx + vz * vz).sqrt();
            let drag_force = spec.drag * speed;

            x += vx * DT;
            z += vz * DT;
            vx -= drag_force * vx * DT;
            vz -= (drag_force * vz + G) * DT;

            if step % 10 == 0 || z <= -1000.0 {
                points.push(TrajPoint {
                    x,
                    z,
                    t: step as f64 * DT,
                });
            }
        }

        Self { degrees, points }
    }

    /// Samples the height (z) and time (t) at a horizontal distance (x).
    fn sample_at_x(&self, target_x: f64) -> Option<TrajPoint> {
        let n = self.points.len();
        if n == 0 || target_x < 0.0 || target_x > self.points[n - 1].x {
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
        let s = if dx.abs() > 1e-6 { (target_x - a.x) / dx } else { 0.0 };

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
            if a.z >= z_target && b.z <= z_target {
                let dz = b.z - a.z;
                let s = if dz.abs() > 1e-6 { (a.z - z_target) / (a.z - b.z) } else { 0.0 };
                return Some(a.x + (b.x - a.x) * s);
            }
        }
        None
    }
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
    while deg < max_deg {
        list.push(Trajectory::simulate(spec, deg));
        deg += 0.5;
    }
    list.push(Trajectory::simulate(spec, max_deg));
    list
}

/// Solves firing solutions for a weapon to hit a target at `distance_m` and relative `delta_z_m`.
pub fn solve_firing_solutions(spec: &WeaponSpec, distance_m: f64, delta_z_m: f64) -> Vec<FiringSolution> {
    if !distance_m.is_finite() || !delta_z_m.is_finite() || distance_m <= 0.0 {
        return Vec::new();
    }

    let family = get_base_trajectories(spec);
    let mut results: Vec<FiringSolution> = Vec::new();

    let mut add_candidate = |traj: &Trajectory, pt: &TrajPoint, arc: TrajectoryArc| {
        if !results.iter().any(|r| (r.degrees - traj.degrees).abs() < 0.01) {
            let sight_range = traj.zero_crossing_x(0.0).unwrap_or(distance_m);
            let mil = traj.degrees / spec.degrees_per_mil - spec.offset;
            results.push(FiringSolution {
                arc,
                degrees: traj.degrees,
                mil,
                sight_range_m: sight_range,
                flight_time_s: pt.t,
            });
        }
    };

    if family.is_empty() {
        return results;
    }

    let mut prev_traj = &family[0];
    let mut prev_pt = prev_traj.sample_at_x(distance_m);

    if let Some(ref pt) = prev_pt {
        if (pt.z - delta_z_m).abs() < 1e-7 {
            add_candidate(prev_traj, pt, TrajectoryArc::Low);
        }
    }

    for curr_traj in &family[1..] {
        let curr_pt = curr_traj.sample_at_x(distance_m);

        let crossed = match (&prev_pt, &curr_pt) {
            (Some(p), Some(c)) => (p.z > delta_z_m) != (c.z > delta_z_m),
            _ => false,
        };

        if crossed {
            let was_above = prev_pt.as_ref().unwrap().z > delta_z_m;
            let mut l = prev_traj.degrees;
            let mut u = curr_traj.degrees;
            let mut best_traj = if was_above { prev_traj.clone() } else { curr_traj.clone() };
            let mut best_pt = if was_above { prev_pt.clone().unwrap() } else { curr_pt.clone().unwrap() };

            for _ in 0..14 {
                if (best_pt.z - delta_z_m).abs() <= 0.02 {
                    break;
                }
                let mid_deg = (l + u) / 2.0;
                let mid_traj = Trajectory::simulate(spec, mid_deg);
                if let Some(mid_pt) = mid_traj.sample_at_x(distance_m) {
                    if (mid_pt.z - delta_z_m).abs() < (best_pt.z - delta_z_m).abs() {
                        best_traj = mid_traj;
                        best_pt = mid_pt;
                    }
                    if (mid_pt.z > delta_z_m) == was_above {
                        l = mid_deg;
                    } else {
                        u = mid_deg;
                    }
                }
            }

            add_candidate(&best_traj, &best_pt, if was_above { TrajectoryArc::High } else { TrajectoryArc::Low });
        } else if let Some(ref c) = curr_pt {
            if (c.z - delta_z_m).abs() < 1e-7 {
                add_candidate(curr_traj, c, TrajectoryArc::High);
            }
        }

        prev_traj = curr_traj;
        prev_pt = curr_pt;
    }

    results.sort_by(|a, b| a.degrees.partial_cmp(&b.degrees).unwrap_or(std::cmp::Ordering::Equal));
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sph2_flat_ground_solution() {
        let dist = 2344.6;
        let sols = solve_firing_solutions(&SPH2_SPEC, dist, 0.0);
        assert_eq!(sols.len(), 2, "SPH-2 should have low and high arc at 2345m");
        
        let low = &sols[0];
        assert_eq!(low.arc, TrajectoryArc::Low);
        assert!((low.sight_range_m - 2345.0).abs() < 2.0);
        assert!((low.mil - 325.0).abs() < 5.0);

        let high = &sols[1];
        assert_eq!(high.arc, TrajectoryArc::High);
        assert!((high.sight_range_m - 2345.0).abs() < 2.0);
        assert!((high.mil - 925.0).abs() < 5.0);
    }

    #[test]
    fn test_sph2_uphill_bakurani_case() {
        // Test case from user report:
        // Gun (97.83, 109.32) to Target (75.28, 102.90)
        // Distance ~2344.6m, Target is ~248-251m higher
        let dist = 2344.6;
        let delta_z = 248.3;
        let sols = solve_firing_solutions(&SPH2_SPEC, dist, delta_z);
        assert!(!sols.is_empty(), "should find solution");

        let low = &sols[0];
        assert_eq!(low.arc, TrajectoryArc::Low);
        // Sight range should be ~2612-2613 m
        assert!((low.sight_range_m - 2613.0).abs() < 2.0, "expected ~2612-2613m, got {}", low.sight_range_m);
        assert!((low.mil - 501.0).abs() < 3.0, "expected ~501 mil, got {}", low.mil);
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
