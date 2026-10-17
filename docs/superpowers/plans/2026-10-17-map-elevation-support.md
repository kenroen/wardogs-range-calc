# Map Selection & Offline Elevation Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add map selection and pre-baked offline terrain elevation lookups to provide accurate $\Delta Z$ and 3D slant range alongside flat 2D artillery firing solutions.

**Architecture:** Embed pre-baked $512 \times 512$ elevation grids for *Bakurani*, *Ozeti*, and *Zestafona* into the Rust binary. A new `GameMap` enum and `elevation` module query heights via bilinear interpolation in $O(1)$ time with no allocations. The UI adds a quick-switching map selector bar under the title bar, elevation readouts on ME/TARGET cards, and a dual 2D Range / 3D Slant Range display in the solution card.

**Tech Stack:** Rust 2024 edition, `egui` / `eframe`, `serde`, raw binary heightfields (`include_bytes!`), Python (for offline raster generation).

**Spec:** [docs/superpowers/specs/2026-10-17-map-elevation-support-design.md](file:///D:/PrivateRepos/wardogs-range-calc/docs/superpowers/specs/2026-10-17-map-elevation-support-design.md)

## Global Constraints

- Standalone executable portability: The compiled `.exe` must remain a single, fully self-contained binary with zero runtime external asset files or network calls.
- Game coordinate coverage: Grid spans `0.0` to `163.2` game units ($16.32\text{ km}$) for all maps.
- Elevation unit: Stored internally as signed 16-bit integers (`i16`) in decimeters ($0.1\text{ m}$), decoded to `f64` metres at runtime.
- Backward compatibility: Saved state in `app.ron` must deserialize gracefully using `#[serde(default)]` if the `map` field is absent from previous runs.
- Firing table integrity: The primary large distance number in the firing solution must remain the 2D horizontal range (`range_m`), as artillery ballistic tables require planar ground distance.

## Review Focus

1. Coordinate boundary safety: Coordinates $< 0.0$ or $> 163.2$ or `NaN` return `None` without index out-of-bounds panics.
2. Flat map mode: When `GameMap::Flat` is active, `delta_z_m` is `None`, slant range equals 2D range, and ALT is hidden in UI cards.
3. Bilinear interpolation accuracy: Sub-cell coordinates smoothly interpolate between vertices without grid-stepping discontinuities.
4. Reproduction calibration: Gun at `x97.95, y109.49` and Target at `x76.78, y103.61` on `Bakurani` must yield 2D Range $= 2,197\text{ m} \pm 1\text{ m}$, $\Delta Z \approx +687\text{ m} \pm 5\text{ m}$, and 3D Slant Range $= 2,302\text{ m} \pm 2\text{ m}$.
5. Window layout constraints: The window size is fixed at $560 \times 880$; the new map selector bar and solution card details must fit without triggering unexpected vertical scrollbars.

---

### Task 1: Generate Pre-Baked Elevation Grids

**Files:**
- Create: `scripts/generate_elevation_grids.py`
- Create: `src/assets/terrain/bakurani.bin`
- Create: `src/assets/terrain/ozeti.bin`
- Create: `src/assets/terrain/zestafona.bin`

**Interfaces:**
- Produces: Binary heightfields ($512 \times 512 \times 2 = 524,288$ bytes each) containing `i16` decimeter samples in little-endian format.

- [ ] **Step 1: Write elevation grid generator script**

Create `scripts/generate_elevation_grids.py` that:
1. Downloads or reads `contours.json` and `manifest.json` for Bakurani, Ozeti, and Zestafona from the verified open-source repository.
2. Reconstructs a $512 \times 512$ grid over the bounding box $[0.0, 163.2] \times [0.0, 163.2]$.
3. Encodes heights as little-endian `i16` decimeters and saves to `src/assets/terrain/<map>.bin`.

- [ ] **Step 2: Run generator script and verify binary outputs**

Run: `python scripts/generate_elevation_grids.py`
Expected: 3 binary files created under `src/assets/terrain/`, each exactly 524,288 bytes.

- [ ] **Step 3: Verify calibration values on Bakurani**

Test via python that $(97.95, 109.49)$ and $(76.78, 103.61)$ on Bakurani yield $\Delta Z \approx +687\text{ m}$.

- [ ] **Step 4: Commit binary assets and script**

```bash
git add scripts/generate_elevation_grids.py src/assets/terrain/*.bin
git commit -m "feat(terrain): generate and embed 512x512 pre-baked elevation grids"
```

---

### Task 2: Implement Elevation Module (`src/elevation.rs`)

**Files:**
- Create: `src/elevation.rs`
- Modify: `src/main.rs:1-15` (register `mod elevation;`)

**Interfaces:**
- Produces:
  - `pub enum GameMap { Bakurani, Ozeti, Zestafona, Flat }`
  - `pub fn elevation_at(map: GameMap, coord: Coord) -> Option<f64>`
  - `pub fn delta_z(map: GameMap, me: Coord, target: Coord) -> Option<f64>`

- [ ] **Step 1: Write the failing tests for `elevation`**

Create `src/elevation.rs` tests:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::coords::Coord;

    #[test]
    fn test_bakurani_calibration_case() {
        let me = Coord { x: 97.95, y: 109.49 };
        let target = Coord { x: 76.78, y: 103.61 };
        let dz = delta_z(GameMap::Bakurani, me, target).expect("should have elevation");
        assert!((dz - 687.0).abs() < 5.0, "expected ~687m, got {}", dz);
    }

    #[test]
    fn test_flat_map_returns_none() {
        let me = Coord { x: 97.95, y: 109.49 };
        assert_eq!(elevation_at(GameMap::Flat, me), None);
    }

    #[test]
    fn test_out_of_bounds_returns_none() {
        let oob = Coord { x: 200.0, y: 50.0 };
        assert_eq!(elevation_at(GameMap::Bakurani, oob), None);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test elevation::tests`
Expected: FAIL (module or functions not defined).

- [ ] **Step 3: Implement `elevation.rs`**

Implement:
1. `GameMap` enum with `Serialize`, `Deserialize`, `Default` (defaulting to `Bakurani`).
2. Static byte arrays holding `include_bytes!("assets/terrain/bakurani.bin")` etc.
3. `elevation_at(map, coord)` using bilinear interpolation across the $512 \times 512$ grid.
4. `delta_z(map, me, target)` computing `target_z - me_z`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test elevation::tests`
Expected: PASS (all 3 tests pass).

- [ ] **Step 5: Commit**

```bash
git add src/elevation.rs src/main.rs
git commit -m "feat(elevation): implement GameMap and bilinear elevation sampling"
```

---

### Task 3: Extend `Solution` Calculation with 3D Slant Range (`src/coords.rs`)

**Files:**
- Modify: `src/coords.rs:139-170`

**Interfaces:**
- Consumes: `crate::elevation::{GameMap, elevation_at, delta_z}`
- Produces:
  - `Solution::between(me: Coord, target: Coord, metres_per_unit: f64, y_north: bool, map: GameMap) -> Self`
  - Fields on `Solution`: `range_m`, `slant_range_m`, `delta_z_m`, `me_alt_m`, `target_alt_m`, `bearing_deg`, `east_m`, `north_m`.

- [ ] **Step 1: Write failing test in `src/coords.rs` for 3D slant range**

```rust
#[test]
fn test_solution_with_elevation_slant_range() {
    let me = Coord { x: 97.95, y: 109.49 };
    let target = Coord { x: 76.78, y: 103.61 };
    let sol = Solution::between(me, target, 100.0, true, crate::elevation::GameMap::Bakurani);
    assert!((sol.range_m - 2197.14).abs() < 1.0);
    assert!((sol.slant_range_m - 2302.0).abs() < 5.0);
    assert!(sol.delta_z_m.is_some());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test coords::tests::test_solution_with_elevation_slant_range`
Expected: FAIL (signature mismatch / fields missing).

- [ ] **Step 3: Update `Solution` and `Solution::between`**

Update `Solution` struct and `between` function to incorporate `GameMap` and compute `me_alt_m`, `target_alt_m`, `delta_z_m`, and `slant_range_m = range_m.hypot(dz)`.

- [ ] **Step 4: Run tests across `coords.rs`**

Run: `cargo test coords::`
Expected: PASS (all unit tests in `coords.rs` pass).

- [ ] **Step 5: Commit**

```bash
git add src/coords.rs
git commit -m "feat(coords): extend Solution with 3D slant range and elevation deltas"
```

---

### Task 4: UI Integration (Map Selector, Coordinate Cards, Solution Card)

**Files:**
- Modify: `src/app.rs`

**Interfaces:**
- Consumes: `crate::elevation::GameMap`, `crate::coords::Solution`
- Updates:
  - `Saved`: add `map: GameMap` with `#[serde(default)]`.
  - Header: add map selector bar with 4 segmented buttons.
  - Coordinate Cards: show `ALT {alt} m` next to `E` and `N`.
  - Solution Card: show `3D SLANT: {slant} m · ΔZ: {dz} m ({UPHILL/DOWNHILL})`.
  - Copy action: format text to include 3D slant and $\Delta Z$.

- [ ] **Step 1: Add automated tests for Map selector and UI state in `src/app.rs`**

Add tests ensuring:
1. `Saved` deserializes default `GameMap::Bakurani` when `map` key is missing.
2. Copy string contains `3D:` and `ΔZ:` when map elevation is present.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test app::tests`
Expected: FAIL.

- [ ] **Step 3: Implement UI changes in `src/app.rs`**

1. Add `pub map: GameMap` to `Saved` struct.
2. In `ui()`, add `map_selector(ui)` between title bar and clipboard routing.
3. Update `coord_card` to show altitude.
4. Update `solution()` to display 3D slant range, $\Delta Z$, uphill/downhill badge, and update clipboard copy formatting.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test app::tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat(ui): add map selector bar and 3D slant range solution display"
```

---

### Task 5: End-to-End Verification and Build

**Files:**
- Check: `target/debug/wardogs-range-calc.exe`
- Check: `Cargo.lock`

- [ ] **Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass with 0 warnings/failures.

- [ ] **Step 2: Build release executable**

Run: `cargo build --release`
Expected: Clean compilation resulting in single self-contained binary at `target/release/wardogs-range-calc.exe`.

- [ ] **Step 3: Verify binary size & persistence**

Verify release executable size is compact (< 20 MB) and test that existing `app.ron` loads without error.

- [ ] **Step 4: Final commit**

```bash
git commit --allow-empty -m "chore: complete map selection and offline elevation support"
```
