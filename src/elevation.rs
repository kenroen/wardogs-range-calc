//! Terrain elevation lookup and map models.

use serde::{Deserialize, Serialize};
use crate::coords::Coord;

pub const GRID_SIZE: usize = 512;
pub const MAX_COORD: f64 = 163.2;

const BAKURANI_BYTES: &[u8] = include_bytes!("assets/terrain/bakurani.bin");
const OZETI_BYTES: &[u8] = include_bytes!("assets/terrain/ozeti.bin");
const ZESTAFONA_BYTES: &[u8] = include_bytes!("assets/terrain/zestafona.bin");

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

    fn grid_bytes(self) -> Option<&'static [u8]> {
        match self {
            GameMap::Bakurani => Some(BAKURANI_BYTES),
            GameMap::Ozeti => Some(OZETI_BYTES),
            GameMap::Zestafona => Some(ZESTAFONA_BYTES),
            GameMap::Flat => None,
        }
    }
}

pub fn elevation_at(map: GameMap, coord: Coord) -> Option<f64> {
    let bytes = map.grid_bytes()?;
    if coord.x < 0.0 || coord.x > MAX_COORD || coord.y < 0.0 || coord.y > MAX_COORD {
        return None;
    }
    if !coord.x.is_finite() || !coord.y.is_finite() {
        return None;
    }

    let u = (coord.x / MAX_COORD) * (GRID_SIZE - 1) as f64;
    let v = (coord.y / MAX_COORD) * (GRID_SIZE - 1) as f64;

    let c0 = (u.floor() as usize).min(GRID_SIZE - 1);
    let r0 = (v.floor() as usize).min(GRID_SIZE - 1);
    let c1 = (c0 + 1).min(GRID_SIZE - 1);
    let r1 = (r0 + 1).min(GRID_SIZE - 1);

    let fu = u - c0 as f64;
    let fv = v - r0 as f64;

    let sample = |row: usize, col: usize| -> f64 {
        let idx = (row * GRID_SIZE + col) * 2;
        let decimeters = i16::from_le_bytes([bytes[idx], bytes[idx + 1]]);
        decimeters as f64 * 0.1
    };

    let q00 = sample(r0, c0);
    let q10 = sample(r0, c1);
    let q01 = sample(r1, c0);
    let q11 = sample(r1, c1);

    let top = q00 + (q10 - q00) * fu;
    let bottom = q01 + (q11 - q01) * fu;

    Some(top + (bottom - top) * fv)
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
    fn test_bakurani_calibration_case() {
        let me = Coord { x: 97.95, y: 109.49 };
        let target = Coord { x: 76.78, y: 103.61 };
        let me_alt = elevation_at(GameMap::Bakurani, me).expect("should have elevation");
        let target_alt = elevation_at(GameMap::Bakurani, target).expect("should have elevation");
        let dz = delta_z(GameMap::Bakurani, me, target).expect("should have elevation");
        assert!((me_alt - 147.0).abs() < 5.0, "expected ~147m, got {}", me_alt);
        assert!((target_alt - 360.0).abs() < 10.0, "expected ~360m, got {}", target_alt);
        assert!((dz - 213.0).abs() < 10.0, "expected ~213m, got {}", dz);
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
