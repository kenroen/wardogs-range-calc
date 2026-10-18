# AGENTS.md — WARDOGS Artillery Range Calculator

## Project Overview

A lightweight, standalone desktop artillery and mortar range/ballistics calculator for **WARDOGS**, built in Rust (`edition = "2024"`) using [`egui`](https://github.com/emilk/egui) / [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe). It listens to OS clipboard events for in-game coordinates (`x97.99, y109.57`) and computes ground distance, compass bearing, terrain elevation (`ASL` / `RISE`), 3D terrain collision (`BLOCKED`), and weapon firing solutions (`L81 Mortar` and `SPH-2` High/Low arcs with `HUD RNG`) with 1:1 parity against `https://wardogs.zone/calculators/artillery`.

---

## Build, Lint & Test Commands

Always run all three checks before considering any change complete:

```powershell
# 1. Lint with Clippy (CI enforces zero warnings across all targets)
cargo clippy --all-targets -- -D warnings

# 2. Run unit and integration test suite
cargo test --locked

# 3. Build optimized release executable (target/release/wardogs-range-calc.exe)
cargo build --release
```

### Additional Commands

```powershell
# Run the GUI application in development mode
cargo run

# Run a single test by name with stdout output
cargo test <test_name> -- --nocapture

# Regenerate offline 2048x2048 compressed terrain grids (requires Python 3 + Pillow + NumPy)
python scripts/generate_elevation_grids.py
```

---

## Architecture & Module Map

- **`src/main.rs`**: Application entry point (`eframe::run_native`), window size/icon configuration, and module declarations.
- **`src/app.rs`**: Main `egui` state (`App`), persistent storage (`Saved`), clipboard routing (`ME` $\rightarrow$ `TARGET`), saved target presets, and HUD panel rendering.
- **`src/ballistics.rs`**: Aerodynamic trajectory solver (`fly_arc`, `solve_arc`, `sample_arc`, `trace_terrain`, `compute_hud_range`).
  - **L81 Mortar**: `v0 = 84.0 m/s`, `g = 9.8 m/s²`, scalar drag `k = 148e-5`, `dt = 0.05 s`, `0.05625 deg/mil` (`6400 mil` circle), high arc `45°–88°`, max range `684 m`.
  - **SPH-2 (155mm)**: `v0 = 300.0 m/s`, `g = 9.8 m/s²`, base drag `k = 659e-6` with 21-point Mach drag curve, `dt = 0.02 s`, `0.0625 deg/mil`, low arc `0°–45°`, high arc `45°–85°`, max range `2827 m`.
  - **Terrain Line-of-Fire (`trace_terrain`)**: Samples the terrain heightfield every `8 m` (with 10-iteration binary search refinement) from a `2.0 m` muzzle offset above `GUN ASL` to detect intermediate terrain collisions (`BLOCKED`).
- **`src/elevation.rs`**: Offline terrain heightfield loader and sampler (`GameMap`: `Bakurani`, `Ozeti`, `Zestafona`, `Flat`).
  - Decodes embedded `WDE2` binary assets (`src/assets/terrain/*.bin`: `2048×2048` `u16` samples compressed via 2D delta predictor + byte-plane transpose + `zlib`).
  - Uses half-pixel cell-center bilinear interpolation (`u * 2048.0 - 0.5`) across the $16.384 \times 16.384\text{ km}$ map space (`MAX_COORD = 163.84`, `1` unit = `100 m`).
  - Provides `js_round` (`(x + 0.5).floor() as i64`) to match JavaScript `Math.round` tie-breaking on negative ASL elevations.
- **`src/coords.rs`**: Coordinate parser (`parse_clipboard`, `parse_lenient`), geometric solver (`Solution::between`), 16-point compass cardinal labels, and NATO `6400` mil azimuth conversion.
- **`src/clipboard.rs`**: Event-driven background OS clipboard listener (`clipboard-rs`) that wakes the `egui` event loop on valid coordinate copies without polling.
- **`src/chrome.rs` & `src/theme.rs`**: Custom frameless tactical HUD titlebar, corner brackets, typography, and color palette.

---

## Coding Guidelines & Conventions

1. **Rust Edition & Style**:
   - Uses **Rust 2024 edition** (`Cargo.toml`).
   - All code must pass `cargo clippy --all-targets -- -D warnings` with zero warnings. Avoid unnecessary allocations in the `egui` per-frame `update` path (`LazyLock` is used to decode terrain grids once on first access).
2. **Ballistics & Elevation Accuracy**:
   - Maintain 1:1 numerical parity with `wardogs.zone/calculators/artillery`.
   - Do not downsample the `2048×2048` (`8 m`) terrain heightfields or alter `min_m`, `step_m`, or `hud_plane_m` constants without verifying against reference coordinates (e.g. `x97.99, y109.57` $\rightarrow$ `x95.62, y104.98` on Bakurani).
   - Coordinate space uses Easting (`X`) growing East and Northing (`Y`) growing North (`y_north = true` is enforced; grid `v` maps to `(163.84 - y) / 163.84`).
3. **Testing**:
   - Every change to coordinate parsing, elevation sampling, or trajectory math must include unit tests in the corresponding module (`src/elevation.rs`, `src/ballistics.rs`, `src/coords.rs`, or `src/app.rs`).
4. **Git & PR Workflow**:
   - **Always switch to a new feature/fix branch off `develop` before making any changes** (`feat/`, `fix/`, `docs/`, `chore/`, `ci/`).
   - Use Conventional Commit messages (e.g., `fix(ballistics): ...`, `docs(readme): ...`). Wrap commit messages and PR bodies in single quotes (`'...'`) in PowerShell to prevent backtick interpolation.
   - **Always ask the user for confirmation before creating a Pull Request (`gh pr create`)**. Never open a PR automatically without asking first.
   - When approved by the user, open Pull Requests targeting `develop` (`main` is reserved for release merges from `develop`, which trigger `.github/workflows/validate.yml`).
   - Keep `README.md`, `assets/screenshot.png`, and `assets/screenshot_annotated.png` in sync whenever UI fields or layout change (use the `update-ui-screenshots` skill).
5. **Self-Learning Loop & Workspace Skills (`.agents/skills/`)**:
   - **`lessons-learned`** (`.agents/skills/lessons-learned/SKILL.md`): Activate this skill before debugging or modifying ballistics/elevation math, and **always append a new entry** to `.agents/skills/lessons-learned/references/known-pitfalls.md` whenever a mistake, command error, test failure, or user correction occurs.
   - **`sync-wardogs-zone`** (`.agents/skills/sync-wardogs-zone/SKILL.md`): Use when verifying or updating map heightfields and weapon ballistic models against `wardogs.zone/calculators/artillery`.
   - **`update-ui-screenshots`** (`.agents/skills/update-ui-screenshots/SKILL.md`): Use when regenerating `assets/screenshot.png` and `assets/screenshot_annotated.png`.
