//! Terrain elevation lookup and map models.
//!
//! Uses the exact 2048x2048 terrain heightfields (8 m cell resolution over 16.384 km)
//! and ASL sea-level offsets from wardogs.zone.

use std::sync::OnceLock;
use serde::{Deserialize, Serialize};
use crate::coords::Coord;

pub const GRID_SIZE: usize = 2048;
pub const MAX_COORD: f64 = 163.84;

const BAKURANI_BYTES: &[u8] = include_bytes!("assets/terrain/bakurani.bin");
const OZETI_BYTES: &[u8] = include_bytes!("assets/terrain/ozeti.bin");
const ZESTAFONA_BYTES: &[u8] = include_bytes!("assets/terrain/zestafona.bin");

static BAKURANI_CELLS: OnceLock<Vec<u16>> = OnceLock::new();
static OZETI_CELLS: OnceLock<Vec<u16>> = OnceLock::new();
static ZESTAFONA_CELLS: OnceLock<Vec<u16>> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GameMap {
    #[default]
    Bakurani,
    Ozeti,
    Zestafona,
    Flat,
}

impl GameMap {
    pub const ALL: [GameMap; 4] = [
        GameMap::Bakurani,
        GameMap::Ozeti,
        GameMap::Zestafona,
        GameMap::Flat,
    ];

    pub fn name(self) -> &'static str {
        match self {
            GameMap::Bakurani => "BAKURANI",
            GameMap::Ozeti => "OZETI",
            GameMap::Zestafona => "ZESTAFONA",
            GameMap::Flat => "FLAT",
        }
    }

    /// Minimum ASL altitude in metres (`minM`) and cell step (`stepM`).
    pub fn terrain_params(self) -> Option<(f64, f64)> {
        match self {
            GameMap::Bakurani => Some((-1008.0, 0.25)),
            GameMap::Ozeti => Some((-1037.0, 0.25)),
            GameMap::Zestafona => Some((25.0, 0.25)),
            GameMap::Flat => None,
        }
    }

    /// Reference altitude plane (`hudPlaneM`) used by the SPH-2 in-game sight HUD RNG scale.
    pub fn hud_plane_m(self) -> Option<f64> {
        match self {
            GameMap::Bakurani => Some(-863.0),
            GameMap::Ozeti | GameMap::Zestafona | GameMap::Flat => None,
        }
    }

    fn grid_cells(self) -> Option<&'static [u16]> {
        match self {
            GameMap::Bakurani => Some(BAKURANI_CELLS.get_or_init(|| decode_cells(BAKURANI_BYTES))),
            GameMap::Ozeti => Some(OZETI_CELLS.get_or_init(|| decode_cells(OZETI_BYTES))),
            GameMap::Zestafona => Some(ZESTAFONA_CELLS.get_or_init(|| decode_cells(ZESTAFONA_BYTES))),
            GameMap::Flat => None,
        }
    }
}

fn decode_cells(compressed: &[u8]) -> Vec<u16> {
    let raw = miniz_oxide::inflate::decompress_to_vec_zlib(compressed)
        .expect("valid zlib terrain data");
    let n = GRID_SIZE * GRID_SIZE;
    assert_eq!(raw.len(), n * 2, "unexpected terrain buffer size");

    let (lo, hi) = raw.split_at(n);
    let mut cells = vec![0u16; n];
    let mut col_acc = vec![0i16; GRID_SIZE];

    for r in 0..GRID_SIZE {
        let row_offset = r * GRID_SIZE;
        let mut row_sum: i16 = 0;
        for (c, acc) in col_acc.iter_mut().enumerate() {
            let idx = row_offset + c;
            let u = u16::from_le_bytes([lo[idx], hi[idx]]);
            let d2 = u as i16;
            *acc = acc.wrapping_add(d2);
            row_sum = row_sum.wrapping_add(*acc);
            cells[idx] = row_sum as u16;
        }
    }
    cells
}

/// Rounds to nearest integer using JavaScript `Math.round` tie-breaking (`+0.5` towards `+∞`).
pub fn js_round(x: f64) -> i64 {
    (x + 0.5).floor() as i64
}

/// Samples ASL terrain elevation in metres at normalized map UV coordinates `ux, uy` in `[0.0, 1.0]`,
/// where `ux = x / 163.84` (east) and `uy = (163.84 - y) / 163.84` (south from top edge).
pub fn elevation_at_uv(map: GameMap, ux: f64, uy: f64) -> Option<f64> {
    let (min_m, step_m) = map.terrain_params()?;
    let cells = map.grid_cells()?;
    if !ux.is_finite() || !uy.is_finite() || !(0.0..=1.0).contains(&ux) || !(0.0..=1.0).contains(&uy) {
        return None;
    }

    let size_f = GRID_SIZE as f64;
    let o = ux * size_f - 0.5;
    let n = uy * size_f - 0.5;
    let a = o.floor() as isize;
    let r = n.floor() as isize;
    let s = o - a as f64;
    let i = n - r as f64;

    let cell_m = |col: isize, row: isize| -> Option<f64> {
        if col < 0 || row < 0 || col >= GRID_SIZE as isize || row >= GRID_SIZE as isize {
            return None;
        }
        let v = cells[row as usize * GRID_SIZE + col as usize];
        if v == 0 {
            None
        } else {
            Some(min_m + (f64::from(v) - 1.0) * step_m)
        }
    };

    let l = cell_m(a, r);
    let d = cell_m(a + 1, r);
    let m = cell_m(a, r + 1);
    let p = cell_m(a + 1, r + 1);

    match (l, d, m, p) {
        (Some(l), Some(d), Some(m), Some(p)) => {
            Some((l * (1.0 - s) + d * s) * (1.0 - i) + (m * (1.0 - s) + p * s) * i)
        }
        _ => {
            let mut sum = 0.0;
            let mut count = 0;
            for val in [l, d, m, p].into_iter().flatten() {
                sum += val;
                count += 1;
            }
            if count == 0 {
                None
            } else {
                Some(sum / f64::from(count))
            }
        }
    }
}

/// Samples ASL terrain elevation in metres at game coordinate `coord` (`x, y` in `0.0..=163.84`).
pub fn elevation_at(map: GameMap, coord: Coord) -> Option<f64> {
    if !coord.x.is_finite() || !coord.y.is_finite() {
        return None;
    }
    if coord.x < 0.0 || coord.x > MAX_COORD || coord.y < 0.0 || coord.y > MAX_COORD {
        return None;
    }
    let ux = coord.x / MAX_COORD;
    let uy = (MAX_COORD - coord.y) / MAX_COORD;
    elevation_at_uv(map, ux, uy)
}

pub fn delta_z(map: GameMap, me: Coord, target: Coord) -> Option<f64> {
    let me_z = elevation_at(map, me)?;
    let target_z = elevation_at(map, target)?;
    Some(target_z - me_z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bakurani_user_case_asl() {
        let me = Coord { x: 97.99, y: 109.57 };
        let target = Coord { x: 95.62, y: 104.98 };
        let me_alt = elevation_at(GameMap::Bakurani, me).expect("should have elevation");
        let target_alt = elevation_at(GameMap::Bakurani, target).expect("should have elevation");
        let dz = delta_z(GameMap::Bakurani, me, target).expect("should have elevation");
        assert_eq!(js_round(me_alt), -863);
        assert_eq!(js_round(target_alt), -786);
        assert_eq!(js_round(dz), 76);
    }

    #[test]
    fn test_bakurani_calibration_case() {
        let me = Coord { x: 97.95, y: 109.49 };
        let target = Coord { x: 76.78, y: 103.61 };
        let me_alt = elevation_at(GameMap::Bakurani, me).expect("should have elevation");
        let target_alt = elevation_at(GameMap::Bakurani, target).expect("should have elevation");
        let dz = delta_z(GameMap::Bakurani, me, target).expect("should have elevation");
        assert!((me_alt - -863.0).abs() < 2.0, "expected ~-863m, got {}", me_alt);
        assert!((target_alt - -644.9).abs() < 2.0, "expected ~-644.9m, got {}", target_alt);
        assert!((dz - 218.1).abs() < 2.0, "expected ~218.1m, got {}", dz);
    }

    #[test]
    fn test_bakurani_shooting_range_case() {
        let me = Coord { x: 97.24, y: 109.54 };
        let target = Coord { x: 81.55, y: 105.17 };
        let me_alt = elevation_at(GameMap::Bakurani, me).expect("should have elevation");
        let target_alt = elevation_at(GameMap::Bakurani, target).expect("should have elevation");
        let dz = delta_z(GameMap::Bakurani, me, target).expect("should have elevation");
        assert!((me_alt - -863.2).abs() < 2.0, "expected ~-863.2m, got {}", me_alt);
        assert!((target_alt - -621.5).abs() < 2.0, "expected ~-621.5m, got {}", target_alt);
        assert!((dz - 241.7).abs() < 2.0, "expected ~241.7m, got {}", dz);
    }

    #[test]
    fn test_flat_map_returns_none() {
        let me = Coord { x: 97.95, y: 109.49 };
        assert_eq!(elevation_at(GameMap::Flat, me), None);
        assert_eq!(delta_z(GameMap::Flat, me, me), None);
    }

    #[test]
    fn test_out_of_bounds_returns_none() {
        let oob = Coord { x: 200.0, y: 50.0 };
        assert_eq!(elevation_at(GameMap::Bakurani, oob), None);
        let neg = Coord { x: -5.0, y: 50.0 };
        assert_eq!(elevation_at(GameMap::Bakurani, neg), None);
    }

    #[test]
    fn test_all_maps_have_elevation() {
        let coord = Coord { x: 80.0, y: 80.0 };
        assert!(elevation_at(GameMap::Bakurani, coord).is_some());
        assert!(elevation_at(GameMap::Ozeti, coord).is_some());
        assert!(elevation_at(GameMap::Zestafona, coord).is_some());
    }
}
