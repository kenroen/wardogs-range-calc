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
    /// Where a low arc becomes available, if the weapon has one.
    pub low_arc_from_m: Option<f64>,
}

/// The community firing tables' limits (L81 mortar and SPH-2 howitzer).
pub const WEAPONS: [Weapon; 2] = [
    Weapon { name: "L81 MORTAR", min_m: 132.0, max_m: 684.0, low_arc_from_m: None },
    Weapon { name: "SPH-2", min_m: 780.0, max_m: 2629.0, low_arc_from_m: Some(1181.0) },
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

/// Range and direction from the gun to the target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Solution {
    /// Horizontal 2D distance in metres (used for community artillery firing tables).
    pub range_m: f64,
    /// 3D direct line-of-sight distance in metres: sqrt(range_m^2 + delta_z^2).
    pub slant_range_m: f64,
    /// Height difference (Target Z - Me Z) in metres, if map elevation is enabled.
    pub delta_z_m: Option<f64>,
    pub me_alt_m: Option<f64>,
    pub target_alt_m: Option<f64>,
    /// Degrees clockwise from north, `0..360`.
    pub bearing_deg: f64,
    /// East and north offsets, in metres.
    pub east_m: f64,
    pub north_m: f64,
}

/// NATO mils in a full circle.
pub const MILS_PER_CIRCLE: f64 = 6400.0;

impl Solution {
    /// `metres_per_unit` is the map scale; `y_north` is false on maps whose Y grows southwards.
    pub fn between(me: Coord, target: Coord, metres_per_unit: f64, y_north: bool, map: crate::elevation::GameMap) -> Self {
        let east_m = (target.x - me.x) * metres_per_unit;
        let north_m = (target.y - me.y) * metres_per_unit * if y_north { 1.0 } else { -1.0 };
        let range_m = east_m.hypot(north_m);
        let bearing_deg = if range_m == 0.0 { 0.0 } else { east_m.atan2(north_m).to_degrees().rem_euclid(360.0) };
        let me_alt_m = crate::elevation::elevation_at(map, me);
        let target_alt_m = crate::elevation::elevation_at(map, target);
        let delta_z_m = match (me_alt_m, target_alt_m) {
            (Some(m), Some(t)) => Some(t - m),
            _ => None,
        };
        let slant_range_m = match delta_z_m {
            Some(dz) => range_m.hypot(dz),
            None => range_m,
        };
        Self {
            range_m,
            slant_range_m,
            delta_z_m,
            me_alt_m,
            target_alt_m,
            bearing_deg,
            east_m,
            north_m,
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
        let s = Solution::between(c(10.0, 10.0), c(13.0, 14.0), 100.0, true, crate::elevation::GameMap::Flat);
        assert!((s.range_m - 500.0).abs() < 1e-9);
        assert!((s.slant_range_m - 500.0).abs() < 1e-9);
        assert_eq!(s.delta_z_m, None);
        assert!((s.bearing_deg - 36.869_897_645).abs() < 1e-6);

        let east = Solution::between(c(0.0, 0.0), c(1.0, 0.0), 100.0, true, crate::elevation::GameMap::Flat);
        assert!((east.bearing_deg - 90.0).abs() < 1e-9);
        assert!((east.bearing_mils() - 1600.0).abs() < 1e-9);
        assert_eq!(east.compass(), "E");

        let west = Solution::between(c(0.0, 0.0), c(-1.0, 0.0), 100.0, true, crate::elevation::GameMap::Flat);
        assert!((west.bearing_deg - 270.0).abs() < 1e-9);

        let south_flipped = Solution::between(c(0.0, 0.0), c(0.0, 1.0), 100.0, false, crate::elevation::GameMap::Flat);
        assert!((south_flipped.bearing_deg - 180.0).abs() < 1e-9);
    }

    #[test]
    fn example_from_the_game() {
        let s = Solution::between(c(95.62, 104.98), c(98.10, 101.55), 100.0, true, crate::elevation::GameMap::Flat);
        assert!((s.range_m - 423.265).abs() < 0.01, "{}", s.range_m);
    }

    #[test]
    fn weapon_reach() {
        let [l81, sph] = WEAPONS;
        assert_eq!(l81.reach(100.0), Reach::TooClose);
        assert_eq!(l81.reach(500.0), Reach::HighArc);
        assert_eq!(l81.reach(700.0), Reach::TooFar);
        assert_eq!(sph.reach(900.0), Reach::HighArc);
        assert_eq!(sph.reach(1500.0), Reach::HighOrLowArc);
    }

    #[test]
    fn test_solution_with_elevation_slant_range() {
        let me = c(97.95, 109.49);
        let target = c(76.78, 103.61);
        let sol = Solution::between(me, target, 100.0, true, crate::elevation::GameMap::Bakurani);
        assert!((sol.range_m - 2197.14).abs() < 1.0);
        assert!((sol.slant_range_m - 2207.0).abs() < 5.0);
        assert!(sol.delta_z_m.is_some());
    }
}
