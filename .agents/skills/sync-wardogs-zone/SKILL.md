---
name: sync-wardogs-zone
description: >-
  Use this skill when WARDOGS updates map terrain or artillery ballistics, when adding a new map or weapon, or when verifying numerical parity against https://wardogs.zone/calculators/artillery.
---

# Sync with `wardogs.zone/calculators/artillery`

This skill extracts the live terrain heightfield metadata and weapon ballistic models from `https://wardogs.zone/calculators/artillery` and updates the offline grids and solver constants in this repository.

## Steps

### 1. Inspect Live `wardogs.zone` Bundles

Run the inspection helper script to fetch the current Next.js chunks from `wardogs.zone` and print the map elevation parameters and weapon ballistic constants:

```powershell
python .agents/skills/sync-wardogs-zone/scripts/inspect_wardogs_zone.py
```

### 2. Compare Terrain Parameters (`src/elevation.rs` & `scripts/generate_elevation_grids.py`)

Check the output from Step 1 against:
- [`scripts/generate_elevation_grids.py`](../../../scripts/generate_elevation_grids.py):
  - Verify the `.webp` heightfield URLs (`kavkazi`, `ozeti`, `zestafona`) and `GRID_SIZE = 2048`.
  - If any `.webp` URL changed, update `MAPS` in `scripts/generate_elevation_grids.py` and regenerate the `WDE2` compressed terrain binaries:
    ```powershell
    python scripts/generate_elevation_grids.py
    ```
- [`src/elevation.rs`](../../../src/elevation.rs):
  - Verify `min_m`, `step_m`, and `hud_plane_m` in `MapMeta`:
    - **Bakurani (`kavkazi`)**: `min_m = -1008.0`, `step_m = 0.25`, `hud_plane_m = Some(-863.0)`
    - **Ozeti (`ozeti`)**: `min_m = -1037.0`, `step_m = 0.25`, `hud_plane_m = None`
    - **Zestafona (`zestafona`)**: `min_m = 25.0`, `step_m = 0.25`, `hud_plane_m = None`
  - Ensure `sample_grid` uses half-pixel cell centers (`u * 2048.0 - 0.5`) and `js_round` (`(x + 0.5).floor() as i64`) for integer ASL / RISE formatting.

### 3. Compare Weapon Ballistic Models (`src/ballistics.rs`)

Check the weapon parameters from Step 1 against [`src/ballistics.rs`](../../../src/ballistics.rs):
- **L81 Mortar (`l81`)**:
  - `v0 = 84.0`, `g = 9.8`, `drag = 148e-5`, `dt = 0.05`, `deg_per_mil = 0.05625`, `offset = -45.0`, `max_range_m = 684.0`
- **SPH-2 (`sph2`)**:
  - `v0 = 300.0`, `g = 9.8`, `drag = 659e-6`, `dt = 0.02`, `deg_per_mil = 0.0625`, `offset = 0.0`, `max_range_m = 2827.0`
  - 21-point Mach velocity drag curve (`SPH2_DRAG_CURVE`)
  - HUD Range model (`compute_hud_range`): `v0 = 222.3`, `g = 9.84`, `drag = 2743e-7`, `deg_per_mil = 0.053`, `offset = 140.6`, `height_scale = 0.879`
- **Terrain Collision (`trace_terrain`)**:
  - Muzzle height `gun_m + 2.0`, `step_m = 8.0` (`16384 / 2048`), `from_m = 16.0` (`2 * step_m`), `to_m = dist - 24.0` (`dist - 3 * step_m`).

### 4. Validate Parity

Run the test suite and linter to confirm 1:1 parity:

```powershell
cargo test --locked
cargo clippy --all-targets -- -D warnings
```
