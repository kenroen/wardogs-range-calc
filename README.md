# WARDOGS // Artillery Range Calculator

A lightweight desktop range, bearing, and 3D ballistic slant calculator for **WARDOGS** artillery and mortar crews, built in Rust with [`egui`](https://github.com/emilk/egui) / [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe).

It watches your system clipboard for in-game coordinates (such as `x95.62, y104.98`) and automatically populates your gun position (**ME**) and impact point (**TARGET**), computing the firing solution immediately with offline terrain elevation context.

<p align="center">
  <img src="assets/screenshot.png" alt="WARDOGS Artillery Range Calculator Screenshot" width="480" />
</p>

---

## Features

- **Map Selection & Offline Elevation Grids**:
  - Segmented map selector supporting **BAKURANI**, **OZETI**, **ZESTAFONA**, and **FLAT** (flat-earth / 2D-only mode).
  - Pre-baked $512 \times 512$ decimeter elevation rasters embedded directly into the binary via `include_bytes!`. Completely offline, zero runtime I/O, no network calls, and single-file portable distribution (~7.3 MB executable).
  - Sub-grid $O(1)$ bilinear interpolation for smooth altitude sampling across the full $16 \times 16\text{ km}$ map space ($X, Y \in [0.0, 163.2]$).
  - Live altitude readout (`ALT {z} m`) on both **ME** and **TARGET** cards when a terrain map is active.
- **Automatic Clipboard Listener**:
  - Monitors the system clipboard in the background (`150 ms` polling) and ignores unrelated copied text.
  - Automatically fills **ME** when empty, then routes all subsequent coordinate copies to **TARGET**.
  - Toggle listening on/off at any time with the `● LISTENING` / `○ PAUSED` button.
- **Instant Firing Solution**:
  - **Horizontal 2D Range** in metres (authoritative for community artillery/mortar sight elevation tables).
  - **3D Slant Range** ($\sqrt{\text{Range}^2 + \Delta Z^2}$) and **Height Difference** ($\Delta Z$) with tactical trajectory indicators: `(UPHILL)`, `(DOWNHILL)`, or `(LEVEL)`.
  - **Compass Bearing** in degrees (`000.0°–360.0°`), 16-point cardinal direction, and **NATO mils** (`6400` mil circle) with an interactive compass rose.
  - **East / North Offsets** (`ΔE` / `ΔN`) in metres.
  - **Weapon Reach Envelopes**:
    - **L81 Mortar**: `132 m – 684 m` (High Arc)
    - **SPH-2**: `780 m – 2,629 m` (High Arc from `780 m`, Low Arc from `1,181 m`)
- **Target Management**:
  - Save, name, and recall targets via a quick-access dropdown on the target card.
  - Delete or overwrite targets with a single click.
- **Quick Controls & UI**:
  - Individual `CLEAR` buttons on both **ME** and **TARGET** cards, plus `CLEAR ALL`, `CLEAR TARGET`, and `⇅ SWAP`.
  - Always-on-top toggle (`ON TOP`) configurable in settings.
  - One-click `COPY` button formats a complete tactical firing report to clipboard (e.g. `2197 m (3D: 2207 m, ΔZ: +213 m), 254.5° (4524 mil)`).
  - Persistent window position, active map, coordinates, and saved targets across restarts.

---

## Coordinate Format

The calculator accepts coordinates copied from the game or typed manually:

| Mode | Supported Examples |
| :--- | :--- |
| **Clipboard & Manual Input** | `x95.62, y104.98`, `X: 95.62 Y: 104.98`, `y104.98 x95.62` |
| **Manual Input Only (Lenient)** | `95.62, 104.98`, `95.62 104.98` |

---

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) `1.85` or newer (Rust 2024 edition compatible)

### Build & Run

```powershell
# Run in development mode
cargo run

# Build optimized release binary
cargo build --release
```

The compiled release executable will be located at:

```text
target/release/wardogs-range-calc.exe
```

### Run Tests

```powershell
cargo test
```

---

## Tooling & Offline Data Generation

To rebuild or update the pre-baked elevation binary grids from open-source community contour models (`contours.json`), an offline Python generator script is included:

```powershell
python scripts/generate_elevation_grids.py
```

Outputs 524,288-byte little-endian `i16` decimeter rasters directly into `src/assets/terrain/`.
