# Map Selection & Offline Elevation Support Design

**Date:** 2026-10-17  
**Status:** Approved  
**Author:** Antigravity & User  

---

## 1. Problem Statement & Context

In *WARDOGS*, players operating indirect fire weapons (SPH-2 artillery and L81 mortar) calculate firing solutions using 2D grid coordinates $(X, Y)$ copied from the in-game map. 

However, terrain elevation varies significantly across maps like **Bakurani**, **Ozeti**, and **Zestafona** (with height deltas often exceeding $600\text{ m}$). This creates two problems:
1. **Discrepancy with In-Game Spotting:** In-game laser rangefinders, binocular reticles, and waypoint pings measure **3D slant range** ($\sqrt{\Delta X^2 + \Delta Y^2 + \Delta Z^2}$). For example, gun at `x97.95, y109.49` and target at `x76.78, y103.61` produces a flat 2D distance of $2,197\text{ m}$, but an in-game slant range of $2,302\text{ m}$ due to a $\approx +687\text{ m}$ uphill height difference.
2. **Artillery Trajectory Drop:** Ballistic firing solutions require awareness of the height differential ($\Delta Z$) between the battery and the target to compensate for uphill/downhill trajectories.

The calculator currently assumes flat ground on a single default scale. This design introduces map selection and offline terrain elevation lookups to provide instant $\Delta Z$ and 3D slant range context.

---

## 2. Architecture & Subsystems

### 2.1 GameMap Model
A `GameMap` enum defines supported maps and persists across restarts:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GameMap {
    #[default]
    Bakurani,
    Ozeti,
    Zestafona,
    Flat, // Disables elevation lookups; treats terrain as 0m
}
```

### 2.2 Elevation Grid & Interpolation (`src/elevation.rs`)
To maintain the app's single-binary portability with zero external assets, elevation grids are pre-baked into compact binary rasters and embedded via `include_bytes!`:

* **Grid Resolution:** $512 \times 512$ samples covering game coordinates $X \in [0.0, 163.2]$ and $Y \in [0.0, 163.2]$ ($\approx 31.9\text{ m}$ per sample cell).
* **Sample Format:** Signed 16-bit integers (`i16`) storing elevation in decimeters ($0.1\text{ m}$ vertical resolution).
* **Storage Footprint:** $512 \times 512 \times 2 = 524,288\text{ bytes}$ ($\approx 512\text{ KB}$) per map, totaling $\approx 1.5\text{ MB}$ uncompressed for all three maps embedded into the final `.exe`.
* **Sampling Algorithm:**
  1. Coordinate $(X, Y)$ is transformed into fractional cell coordinates:
     $$u = \frac{X - X_{\min}}{X_{\max} - X_{\min}} \times (W - 1)$$
     $$v = \frac{Y - Y_{\min}}{Y_{\max} - Y_{\min}} \times (H - 1)$$
  2. Fractional coordinates identify the four surrounding grid vertices: $Q_{00}, Q_{10}, Q_{01}, Q_{11}$.
  3. Bilinear interpolation computes a smooth, continuous terrain height:
     $$Z(u, v) = (1 - f_x)(1 - f_y) Q_{00} + f_x (1 - f_y) Q_{10} + (1 - f_x) f_y Q_{01} + f_x f_y Q_{11}$$
  4. Returns `None` if the point is outside the playable map bounds or if `GameMap::Flat` is selected.

### 2.3 Extended Firing Solution (`src/coords.rs`)
[`Solution`](file:///D:/PrivateRepos/wardogs-range-calc/src/coords.rs) is updated to carry 3D spatial context alongside planar distance:

```rust
pub struct Solution {
    /// Horizontal 2D distance in metres (used for community artillery firing tables).
    pub range_m: f64,
    /// 3D direct line-of-sight distance in metres: sqrt(range_m^2 + delta_z^2).
    pub slant_range_m: f64,
    /// Height difference (Target Z - Me Z) in metres, if map elevation is enabled.
    pub delta_z_m: Option<f64>,
    pub me_alt_m: Option<f64>,
    pub target_alt_m: Option<f64>,
    pub bearing_deg: f64,
    pub east_m: f64,
    pub north_m: f64,
}
```

When `delta_z_m` is `Some(dz)`:
$$\text{slant\_range\_m} = \sqrt{\text{range\_m}^2 + dz^2}$$
When `None`:
$$\text{slant\_range\_m} = \text{range\_m}$$

---

## 3. User Interface & Display

### 3.1 Map Selector Bar
Located immediately beneath the custom title bar and above the `CLIPBOARD` routing row:
* Four segmented HUD buttons: `[ BAKURANI ]  [ OZETI ]  [ ZESTAFONA ]  [ FLAT ]`.
* The currently active map is highlighted with `theme::ACCENT` (`#f0b24a`).
* Clicking another map instantly recalculates the solution and updates the elevation context.

### 3.2 Position Cards (ME and TARGET)
When valid coordinates are locked and a terrain map is active, the coordinate card footer displays the sampled elevation:
* **ME:** `E 97.95  ·  N 109.49  ·  ALT 147 m  ·  ✔ LOCKED`
* **TARGET:** `E 76.78  ·  N 103.61  ·  ALT 360 m  ·  ✔ LOCKED`
*(Omitted if `FLAT` is selected).*

### 3.3 Solution Card
* **Primary 2D Range:** Displayed in large font ($56\text{ pt}$) as the authoritative value for dialing artillery/mortar sights.
* **Secondary 3D Slant & Delta Z:**
  ```text
  3D SLANT: 2207 m  ·  ΔZ: +213 m (UPHILL)
  ```
  * Uphill indicated in accent/warning color (`theme::ACCENT`), downhill in good color (`theme::GOOD`), level in neutral (`theme::MUTED`).
* **COPY Button:** Copies formatted summary including 3D slant range:
  `2197 m (3D: 2207 m, ΔZ: +213 m), 254.5° (4524 mil)`

---

## 4. Pre-Baking Tooling & Data Artifacts

A build-time or offline generation script (`scripts/generate_elevation_grids.py`) constructs the $512 \times 512$ grids from community terrain contours and manifests:
* Target outputs: `src/assets/terrain/bakurani.bin`, `ozeti.bin`, `zestafona.bin`.
* Stored directly in the repository so compilation requires no network access and no Python runtime for normal `cargo build`.

---

## 5. Verification & Testing

1. **Exact Reproduction Test Case:**
   * Coordinates: ME `x97.95, y109.49`, TARGET `x76.78, y103.61` on **Bakurani**.
   * Ground-Truth Values: Extracted from `contours.json` triangulated raster:
     - ME Elevation $= 147\text{ m} \pm 2\text{ m}$
     - TARGET Elevation $= 360\text{ m} \pm 2\text{ m}$
     - $\Delta Z = +213\text{ m} \pm 2\text{ m}$
     - 2D Range $= 2,197\text{ m} \pm 1\text{ m}$
     - 3D Slant Range $= 2,207\text{ m} \pm 2\text{ m}$
   *(Note: Earlier hypothetical estimates assumed $+687\text{ m}$ $\Delta Z$ prior to obtaining community contour dataset).*
2. **Flat Mode Test Case:**
   * Same coordinates with `Flat` selected: $\Delta Z = \text{None}$, Slant Range $= 2,197\text{ m}$.
3. **Bilinear Interpolation & Boundary Tests:**
   * Test interpolation continuity between adjacent cells.
   * Test out-of-bounds coordinates return `None` safely without panic.
4. **UI Snapshot / Interaction Tests:**
   * Verify switching maps persists to `Saved` and updates calculations.
