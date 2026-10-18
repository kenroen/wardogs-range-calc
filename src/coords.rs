//! Game coordinates and the firing solution between two of them.
//!
//! WARDOGS map coordinates read like `x95.62, y104.98`: X increases to the east, Y to the north,
//! and one coordinate unit is 100 metres (so `0.01` is one metre).

use std::fmt;

/// A position on the map, in game coordinate units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coord {
    pub x: f64,
    pub y: f64,
}

impl fmt::Display for Coord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "x{:.2}, y{:.2}", self.x, self.y)
    }
}

/// Reads a coordinate from text such as `x95.62, y104.98`, `X: 95.62 Y: 104.98` or
/// `y104.98 x95.62`. Each axis is a letter followed (optionally after spaces, `:` or `=`) by a
/// number. With `lenient`, two bare numbers (`95.62, 104.98`) are accepted too, for typing into
/// the fields; the clipboard watcher is strict so it ignores unrelated copied numbers.
pub fn parse(text: &str, lenient: bool) -> Option<Coord> {
    let x = tagged(text, 'x');
    let y = tagged(text, 'y');
    match (x, y) {
        (Some(x), Some(y)) => Some(Coord { x, y }),
        _ if lenient && x.is_none() && y.is_none() => {
            let mut numbers = numbers(text);
            let (x, y) = (numbers.next()?, numbers.next()?);
            numbers.next().is_none().then_some(Coord { x, y })
        }
        _ => None,
    }
}

/// The first number that follows `axis` (either case) and is not part of a longer word.
fn tagged(text: &str, axis: char) -> Option<f64> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    for (i, &(_, c)) in chars.iter().enumerate() {
        if c.to_ascii_lowercase() != axis {
            continue;
        }
        // `max95` or `bx1` should not count as an axis tag.
        if i > 0 && chars[i - 1].1.is_alphanumeric() {
            continue;
        }
        let mut j = i + 1;
        while j < chars.len() && matches!(chars[j].1, ' ' | '\t' | ':' | '=') {
            j += 1;
        }
        let start = chars.get(j).map_or(text.len(), |&(b, _)| b);
        if let Some(n) = leading_number(&text[start..]) {
            return Some(n);
        }
    }
    None
}

/// A number at the very start of `s`: optional sign, digits, optional fraction.
fn leading_number(s: &str) -> Option<f64> {
    let bytes = s.as_bytes();
    let mut end = 0;
    if matches!(bytes.first(), Some(b'-' | b'+')) {
        end += 1;
    }
    let digits_start = end;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    let mut digits = end - digits_start;
    if end < bytes.len() && bytes[end] == b'.' {
        let frac_start = end + 1;
        let mut k = frac_start;
        while k < bytes.len() && bytes[k].is_ascii_digit() {
            k += 1;
        }
        if k > frac_start {
            digits += k - frac_start;
            end = k;
        }
    }
    if digits == 0 {
        return None;
    }
    s[..end].parse().ok()
}

/// Every bare number in `text`.
fn numbers(text: &str) -> impl Iterator<Item = f64> + '_ {
    text.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+'))
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
}

/// A weapon whose firing table covers a band of ranges, in metres.
#[derive(Clone, Copy, Debug)]
pub struct Weapon {
    pub name: &'static str,
    pub min_m: f64,
    pub max_m: f64,
    /// Where a low arc becomes available on flat ground, if the weapon has one.
    pub low_arc_from_m: Option<f64>,
}

/// The weapon firing table limits (L81 mortar and SPH-2 howitzer).
pub const WEAPONS: [Weapon; 2] = [
    Weapon { name: "L81 MORTAR", min_m: 52.0, max_m: 685.0, low_arc_from_m: None },
    Weapon { name: "SPH-2", min_m: 217.0, max_m: 2636.0, low_arc_from_m: Some(489.0) },
];

/// How a weapon can engage a given range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reach {
    TooClose,
    HighArc,
    HighOrLowArc,
    TooFar,
}

impl Weapon {
    pub fn reach(&self, range_m: f64) -> Reach {
        if range_m < self.min_m {
            Reach::TooClose
        } else if range_m > self.max_m {
            Reach::TooFar
        } else if self.low_arc_from_m.is_some_and(|low| range_m >= low) {
            Reach::HighOrLowArc
        } else {
            Reach::HighArc
        }
    }
}

use crate::ballistics::{self, FiringSolution, MORTAR_SPEC, SPH2_SPEC};
use crate::elevation::{self, GameMap};

/// Ballistic solutions for supported weapons (SPH-2 low/high arc, L81 mortar).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeaponSolutions {
    pub sph2_low: Option<FiringSolution>,
    pub sph2_high: Option<FiringSolution>,
    pub sph2_band_m: (f64, f64),
    pub mortar: Option<FiringSolution>,
    pub mortar_band_m: (f64, f64),
}

impl WeaponSolutions {
    pub fn compute(distance_m: f64, delta_z_m: f64, map: GameMap, me: Coord, target: Coord, me_alt_m: Option<f64>) -> Self {
        let hud_plane_m = map.hud_plane_m();
        let step_m = 16_384.0 / elevation::GRID_SIZE as f64;
        let from_m = 2.0 * step_m;
        let to_m = distance_m - 3.0 * step_m;

        let trace_fn = |spec: &ballistics::WeaponSpec, sol: &mut FiringSolution| {
            let mil_rounded = (sol.mil * 10.0).round() / 10.0;
            sol.hud_range_m = ballistics::compute_hud_range(spec, mil_rounded, me_alt_m, hud_plane_m);

            if let Some(gun_z) = me_alt_m
                && distance_m > 0.0
                && to_m > from_m
            {
                let muzzle_z = gun_z + ballistics::MUZZLE_M;
                let traj = ballistics::Trajectory::simulate(spec, sol.degrees.to_radians());
                sol.blocked_at_m = traj.trace_terrain(from_m, to_m, step_m, |r| {
                    let frac = r / distance_m;
                    let sample_coord = Coord {
                        x: me.x + (target.x - me.x) * frac,
                        y: me.y + (target.y - me.y) * frac,
                    };
                    elevation::elevation_at(map, sample_coord).map(|z| z - muzzle_z)
                });
            }
        };

        let sph2_sols = ballistics::solve_firing_solutions(&SPH2_SPEC, distance_m, delta_z_m);
        let mut sph2_low = None;
        let mut sph2_high = None;
        for mut s in sph2_sols {
            trace_fn(&SPH2_SPEC, &mut s);
            match s.arc {
                ballistics::TrajectoryArc::Low => sph2_low = Some(s),
                ballistics::TrajectoryArc::High => sph2_high = Some(s),
            }
        }

        let mortar_sols = ballistics::solve_firing_solutions(&MORTAR_SPEC, distance_m, delta_z_m);
        let mut mortar = mortar_sols.into_iter().next();
        if let Some(ref mut m) = mortar {
            trace_fn(&MORTAR_SPEC, m);
        }

        Self {
            sph2_low,
            sph2_high,
            sph2_band_m: ballistics::range_band(&SPH2_SPEC, delta_z_m),
            mortar,
            mortar_band_m: ballistics::range_band(&MORTAR_SPEC, delta_z_m),
        }
    }
}

/// Range and direction from the gun to the target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Solution {
    /// Horizontal 2D distance in metres (`DIST` on `wardogs.zone`).
    pub range_m: f64,
    /// 3D direct line-of-sight distance in metres: sqrt(range_m^2 + delta_z^2).
    pub slant_range_m: f64,
    /// Height difference (`RISE`: Target ASL - Gun ASL) in metres, if map elevation is enabled.
    pub delta_z_m: Option<f64>,
    pub me_alt_m: Option<f64>,
    pub target_alt_m: Option<f64>,
    /// Degrees clockwise from north, `0..360`.
    pub bearing_deg: f64,
    /// East and north offsets, in metres.
    pub east_m: f64,
    pub north_m: f64,
    /// Ballistic firing solutions (mils, time of flight, HUD RNG, and terrain collision).
    pub weapons: WeaponSolutions,
}

/// NATO mils in a full circle.
pub const MILS_PER_CIRCLE: f64 = 6400.0;

impl Solution {
    /// `metres_per_unit` is the map scale; `y_north` is false on maps whose Y grows southwards.
    pub fn between(me: Coord, target: Coord, metres_per_unit: f64, y_north: bool, map: GameMap) -> Self {
        let east_m = (target.x - me.x) * metres_per_unit;
        let north_m = (target.y - me.y) * metres_per_unit * if y_north { 1.0 } else { -1.0 };
        let range_m = east_m.hypot(north_m);
        let bearing_deg = if range_m == 0.0 { 0.0 } else { east_m.atan2(north_m).to_degrees().rem_euclid(360.0) };
        let me_alt_m = elevation::elevation_at(map, me);
        let target_alt_m = elevation::elevation_at(map, target);
        let delta_z_m = elevation::delta_z(map, me, target);
        let slant_range_m = match delta_z_m {
            Some(dz) => range_m.hypot(dz),
            None => range_m,
        };
        let ballistic_dz = delta_z_m.unwrap_or(0.0);
        let weapons = WeaponSolutions::compute(range_m, ballistic_dz, map, me, target, me_alt_m);
        Self {
            range_m,
            slant_range_m,
            delta_z_m,
            me_alt_m,
            target_alt_m,
            bearing_deg,
            east_m,
            north_m,
            weapons,
        }
    }

    pub fn bearing_mils(&self) -> f64 {
        (self.bearing_deg * MILS_PER_CIRCLE / 360.0).rem_euclid(MILS_PER_CIRCLE)
    }

    /// The 16-point compass name of the bearing.
    pub fn compass(&self) -> &'static str {
        const POINTS: [&str; 16] = ["N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW", "NW", "NNW"];
        POINTS[((self.bearing_deg / 22.5).round() as usize) % 16]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(x: f64, y: f64) -> Coord {
        Coord { x, y }
    }

    #[test]
    fn parses_game_format() {
        assert_eq!(parse("x95.62, y104.98", false), Some(c(95.62, 104.98)));
        assert_eq!(parse("  X95.62 Y104.98\r\n", false), Some(c(95.62, 104.98)));
        assert_eq!(parse("x: 95.62; y = 104.98", false), Some(c(95.62, 104.98)));
        assert_eq!(parse("y104.98, x95.62", false), Some(c(95.62, 104.98)));
        assert_eq!(parse("x95, y-3.5", false), Some(c(95.0, -3.5)));
    }

    #[test]
    fn strict_rejects_other_text() {
        assert_eq!(parse("95.62, 104.98", false), None);
        assert_eq!(parse("max 95 yy", false), None);
        assert_eq!(parse("x95.62", false), None);
        assert_eq!(parse("hello world", false), None);
        assert_eq!(parse("", false), None);
    }

    #[test]
    fn lenient_accepts_bare_pair() {
        assert_eq!(parse("95.62, 104.98", true), Some(c(95.62, 104.98)));
        assert_eq!(parse("95.62 104.98 7", true), None);
        assert_eq!(parse("x95.62", true), None);
    }

    #[test]
    fn display_round_trips() {
        let p = c(95.62, 104.98);
        assert_eq!(p.to_string(), "x95.62, y104.98");
        assert_eq!(parse(&p.to_string(), false), Some(p));
    }

    #[test]
    fn solution_range_and_bearing() {
        // 3-4-5 triangle: 3 units east, 4 north.
        let s = Solution::between(c(10.0, 10.0), c(13.0, 14.0), 100.0, true, GameMap::Flat);
        assert!((s.range_m - 500.0).abs() < 1e-9);
        assert!((s.slant_range_m - 500.0).abs() < 1e-9);
        assert_eq!(s.delta_z_m, None);
        assert!((s.bearing_deg - 36.869_897_645).abs() < 1e-6);

        let east = Solution::between(c(0.0, 0.0), c(1.0, 0.0), 100.0, true, GameMap::Flat);
        assert!((east.bearing_deg - 90.0).abs() < 1e-9);
        assert!((east.bearing_mils() - 1600.0).abs() < 1e-9);
        assert_eq!(east.compass(), "E");

        let west = Solution::between(c(0.0, 0.0), c(-1.0, 0.0), 100.0, true, GameMap::Flat);
        assert!((west.bearing_deg - 270.0).abs() < 1e-9);

        let south_flipped = Solution::between(c(0.0, 0.0), c(0.0, 1.0), 100.0, false, GameMap::Flat);
        assert!((south_flipped.bearing_deg - 180.0).abs() < 1e-9);
    }

    #[test]
    fn example_from_the_game() {
        let s = Solution::between(c(95.62, 104.98), c(98.10, 101.55), 100.0, true, GameMap::Flat);
        assert!((s.range_m - 423.265).abs() < 0.01, "{}", s.range_m);
    }

    #[test]
    fn weapon_reach() {
        let [l81, sph] = WEAPONS;
        assert_eq!(l81.reach(40.0), Reach::TooClose);
        assert_eq!(l81.reach(500.0), Reach::HighArc);
        assert_eq!(l81.reach(700.0), Reach::TooFar);
        assert_eq!(sph.reach(300.0), Reach::HighArc);
        assert_eq!(sph.reach(1500.0), Reach::HighOrLowArc);
    }

    #[test]
    fn test_user_screenshots_exact_match() {
        let me = c(97.99, 109.57);
        let target = c(95.62, 104.98);
        let sol = Solution::between(me, target, 100.0, true, GameMap::Bakurani);

        // Matches both L81.png and sph2.png:
        // AZM 207.3°, DIST 517 m, GUN ASL -863 m, TGT ASL -786 m, RISE +76 m
        assert!((sol.bearing_deg - 207.3).abs() < 0.05);
        assert_eq!(sol.range_m.round() as i64, 517);
        assert_eq!(elevation::js_round(sol.me_alt_m.unwrap()), -863);
        assert_eq!(elevation::js_round(sol.target_alt_m.unwrap()), -786);
        assert_eq!(elevation::js_round(sol.delta_z_m.unwrap()), 76);

        // L81.png: HIGH 374.8 mil 15.6s
        let mortar = sol.weapons.mortar.expect("L81 in range");
        assert!((mortar.mil - 374.8).abs() < 0.1);
        assert!((mortar.flight_time_s - 15.6).abs() < 0.1);
        assert!(mortar.blocked_at_m.is_none());

        // sph2.png: HIGH 1,342.8 mil 36.9s HUD RNG 984 m, LOW 167.0 mil 2.1s HUD RNG 1859 m (GROUND STOPS THE LOW ARC)
        let high = sol.weapons.sph2_high.expect("SPH-2 high arc in range");
        assert!((high.mil - 1342.8).abs() < 0.1);
        assert!((high.flight_time_s - 36.9).abs() < 0.1);
        assert_eq!(high.hud_range_m.unwrap().round() as i64, 984);
        assert!(high.blocked_at_m.is_none());

        let low = sol.weapons.sph2_low.expect("SPH-2 low arc solved");
        assert!((low.mil - 167.0).abs() < 0.1);
        assert!((low.flight_time_s - 2.1).abs() < 0.1);
        assert_eq!(low.hud_range_m.unwrap().round() as i64, 1859);
        assert!(low.blocked_at_m.is_some(), "ground should stop the low arc");
    }

    #[test]
    fn test_solution_with_elevation_slant_range() {
        let me = c(97.95, 109.49);
        let target = c(76.78, 103.61);
        let sol = Solution::between(me, target, 100.0, true, GameMap::Bakurani);
        assert!((sol.range_m - 2197.14).abs() < 1.0);
        assert!((sol.slant_range_m - 2208.0).abs() < 5.0);
        assert!(sol.delta_z_m.is_some());
    }
}
