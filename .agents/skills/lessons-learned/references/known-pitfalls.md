# Known Pitfalls & Lessons Learned

## 1. Terrain & Elevation Math (`src/elevation.rs`)

### Downsampling `2048x2048` Terrain Grids Breaks Local Slopes
- **Symptom / Mistake**: Downsampling the `2048x2048` (`8 m` per cell) heightfields to `512x512` (`32 m` per cell) caused `TGT ASL` and `RISE` on steep hills to drift by several metres compared to `wardogs.zone`.
- **Root Cause**: A `32 m` grid averages out sharp terrain ridges and local slopes.
- **Rule / Fix**: Always keep the full `2048x2048` `u16` heightfield resolution compressed via `WDE2` (2D delta predictor + byte-plane transpose + `zlib`).

### Half-Pixel Cell-Center Bilinear Interpolation
- **Symptom / Mistake**: Standard grid indexing `u * (size - 1)` produces ~0.25–0.5 m elevation differences against `wardogs.zone`.
- **Root Cause**: `wardogs.zone` maps normalized coordinates `u, v in [0, 1]` to pixel cell centers via `o = u * 2048.0 - 0.5` and `n = v * 2048.0 - 0.5`.
- **Rule / Fix**: Always use `u * (GRID_SIZE as f64) - 0.5` in `sample_grid`.

### JavaScript `Math.round` vs. Rust `f64::round` on Negative ASL
- **Symptom / Mistake**: At `-786.5 m` (`TGT ASL`), Rust's `(-786.5_f64).round()` yields `-787`, whereas `wardogs.zone` displays `-786 m`.
- **Root Cause**: Rust's `f64::round` rounds half-way cases away from zero (`-786.5 -> -787`), while JavaScript's `Math.round(x)` is `Math.floor(x + 0.5)` which rounds `-0.5` toward `+∞` (`-786.5 -> -786`).
- **Rule / Fix**: Always use `elevation::js_round(x)` (`(x + 0.5).floor() as i64`) when formatting integer `ASL` or `RISE` values for display.

### Negative `ASL` Values on Bakurani and Ozeti
- **Symptom / Mistake**: Assuming `min_m = 0.0` for all maps or assuming negative `ASL` is a bug.
- **Root Cause**: In CryEngine / `wardogs.zone`, Bakurani (`min_m = -1008.0`, shooting range `-863.0`) and Ozeti (`min_m = -1037.0`) sit below world `Z = 0`, whereas Zestafona sits above (`min_m = +25.0`).
- **Rule / Fix**: Preserve each map's exact `min_m`, `step_m = 0.25`, and `hud_plane_m` (`Some(-863.0)` on Bakurani).

---

## 2. Ballistics & Firing Solutions (`src/ballistics.rs`)

### `DIST` vs. `HUD RNG` vs. `Sight Range`
- **Symptom / Mistake**: Displaying an invented "flat-ground equivalent range" (`1416 m`) as the main range readout when firing uphill at a `517 m` target.
- **Root Cause**: `wardogs.zone` displays planar ground distance (`517 m`) as the primary `DIST` readout, solves the exact `mil` elevation angle using the weapon's aerodynamic drag model, and separately projects the SPH-2's in-game `HUD RNG` readout (`compute_hud_range`) relative to `hud_plane_m`.
- **Rule / Fix**: Keep `DIST (GROUND)` as the primary distance readout (`s.range_m`), and show `HUD RNG <m>m` on the `SPH-2 HIGH` and `SPH-2 LOW` weapon rows.

### SPH-2 Uses a 21-Point Mach Drag Curve (Not Constant Scalar Drag)
- **Symptom / Mistake**: Using a single constant drag coefficient for the SPH-2 causes significant mil and flight-time errors.
- **Root Cause**: `wardogs.zone` models the SPH-2 with `v0 = 300.0 m/s`, `g = 9.8 m/s²`, `k = 659e-6`, `dt = 0.02 s`, `0.0625 deg/mil`, multiplied by a 21-point velocity-dependent drag curve (`SPH2_DRAG_CURVE`).
- **Rule / Fix**: Always interpolate `SPH2_DRAG_CURVE` at current projectile speed `hypot(vx, vy)` on each Euler step (`dt = 0.02 s`).

### Coordinate Y-Axis Orientation (`y_north = true`)
- **Symptom / Mistake**: Persisted `y_north = false` in `%APPDATA%` inverted North/South bearings (`332.7°` instead of `207.3°`).
- **Root Cause**: In *WARDOGS*, `Y` always increases northward (`v = (163.84 - y) / 163.84` in image row space).
- **Rule / Fix**: Enforce `s.y_north = true` on startup in `App::new`.

---

## 3. PowerShell (`pwsh`) & Git Tooling on Windows

### Backticks in Double-Quoted PowerShell Strings
- **Symptom / Mistake**: Running `git commit -m "... (`u * 2048 - 0.5`) ..."` in double quotes fails in `pwsh` with `The Unicode escape sequence is not valid. A valid sequence is `u{...}`.
- **Root Cause**: PowerShell (`pwsh`) uses the backtick `` ` `` as its escape character inside double-quoted strings (`"..."`), so `` `u `` is parsed as a Unicode escape sequence.
- **Rule / Fix**: Always wrap `git commit -m '...'` and `gh pr create --title '...' --body '...'` arguments in **single quotes** (`'...'`) in PowerShell so backticks and `$` are treated literally.

### GitHub Ruleset Required Status Checks & CI Validation Workflow
- **Symptom / Mistake**: PRs targeting `main` could be merged while `CI Validation` was still running or failing, and the workflow unnecessarily built a release binary and uploaded an artifact.
- **Root Cause**: The `Protect main branch` repository ruleset (`25298684`) had `required_status_checks` enabled with an empty `required_status_checks: []` array, and `.github/workflows/validate.yml` had an unnecessary `if: github.head_ref == 'develop'` guard plus release build/artifact upload steps.
- **Rule / Fix**: Keep `.github/workflows/validate.yml` focused strictly on validation (`cargo clippy --all-targets -- -D warnings` and `cargo test --locked --verbose`) with job name `Validate & Test (Windows)` on `pull_request` to `main`, and ensure the `Protect main branch` ruleset explicitly lists `{"context": "Validate & Test (Windows)", "integration_id": 15368}` under `required_status_checks`.
