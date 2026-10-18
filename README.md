# WARDOGS // Artillery Range Calculator

A lightweight, standalone desktop range, bearing, and ballistic sight calculator for **WARDOGS** artillery and mortar crews, built in Rust with [`egui`](https://github.com/emilk/egui) / [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe).

It monitors your system clipboard for in-game coordinates (such as `x95.62, y104.98`), populating your gun position (**ME**) and impact point (**TARGET**), and automatically computes the exact elevation-corrected sight range, bearing, and weapon mil solutions using embedded offline terrain models.

<p align="center">
  <img src="assets/screenshot.png" alt="WARDOGS Artillery Range Calculator Screenshot" width="480" />
</p>

---

## Interface & Field Reference

<p align="center">
  <img src="assets/screenshot_annotated.png" alt="WARDOGS Artillery Range Calculator Annotated Interface Guide" width="760" />
</p>

### 1. Map Selector (`MAP`)
Selects the active terrain elevation model used to sample altitudes and compute elevation-compensated ballistic trajectories:
- **`BAKURANI` / `OZETI` / `ZESTAFONA`**: Uses the embedded $512 \times 512$ offline terrain heightfield ($16.384 \times 16.384\text{ km}$, $X, Y \in [0.0, 163.84]$) with bilinear interpolation to look up exact ground elevation at both coordinates.
- **`FLAT`**: Disables terrain elevation (`ΔZ = 0 m`), computing pure 2D horizontal distance and level-ground firing solutions.

### 2. Clipboard Status (`CLIPBOARD`)
Controls the OS event-driven background clipboard listener ([`clipboard-rs`](https://crates.io/crates/clipboard-rs)):
- **`● LISTENING`** *(Green)*: Listens for OS clipboard change events (zero polling) and immediately captures valid in-game coordinates (e.g. `x97.83, y109.32`) while ignoring unrelated text. Click to pause.
- **`○ PAUSED`** *(Muted)*: Clipboard event handling is suspended; coordinates can still be entered or edited manually.

### 3. Gun Position Card (`■ ME — GUN POSITION`)
Represents your artillery piece or mortar position (cyan accent):
- **Coordinate Input Field (`x97.83, y109.32`)**: Displays your current position. When empty, the very first coordinate copied to the clipboard is routed here automatically. You can also type or paste coordinates manually.
- **`CLEAR` Button**: Empties the **ME** field so the next copied coordinate will populate your gun position again.
- **`◀ NEXT PASTE` Badge** *(shown when `ME` is empty)*: Indicates that the next coordinate copied in-game will land in this card.
- **Telemetry & Validation Bar (`E 97.83 N 109.32 ALT 147 m ✔ LOCKED`)**:
  - **`E` / `N`**: Parsed Easting ($X$) and Northing ($Y$) map coordinates.
  - **`ALT {z} m`**: Interpolated terrain altitude in metres above sea level (hidden in `FLAT` mode or if out of bounds).
  - **Status Indicator**: `✔ LOCKED` when valid coordinates are parsed, `AWAITING COORDINATES` when blank, or `✖ UNREADABLE — USE x95.62, y104.98` if the text cannot be parsed.

### 4. Target Position Card (`■ TARGET — IMPACT POINT`)
Represents the enemy impact point (red accent):
- **`◀ NEXT PASTE` Badge**: Once **ME** is populated, this badge appears on **TARGET** to show that all subsequent clipboard copies automatically update the target without disturbing your gun position.
- **Coordinate Input Field (`x75.28, y102.90`)**: The target's map coordinates, updated via clipboard or manual typing.
- **`CLEAR` Button**: Clears only the target coordinates and active target name.
- **Telemetry & Validation Bar (`E 75.28 N 102.90 ALT 398 m ✔ LOCKED`)**: Shows the target's parsed Easting (`E`), Northing (`N`), terrain elevation (`ALT`), and coordinate lock state.

### 5. Saved Targets (`SAVED`)
Built into the bottom of the **TARGET** card for storing and recalling frequent fire missions (persisted across restarts):
- **`SAVED` Dropdown (`High Outpost (x75.28, y102.90)`)**: Select any previously stored target to immediately load its coordinates into **TARGET**.
- **`DEL` Button**: Appears whenever the active target coordinates match a saved entry; click to delete that preset.
- **Target Name Input (`High Outpost`)**: Enter a custom callsign or landmark name for the current target coordinates.
- **`SAVE` Button**: Saves the current target coordinates under the entered name (or overwrites an existing preset with the same name). Pressing `Enter` in the name box also saves.

### 6. Primary Sight & Azimuth Readout (`DIAL RANGE (SIGHT)` / `RANGE`)
The main firing solution header at the top of the solution panel:
- **`DIAL RANGE (SIGHT)` (`2615 m`)**: The primary elevation-compensated sight distance in metres to dial on your weapon's sight drum. Because firing uphill (`+ΔZ`) requires aiming higher and firing downhill (`-ΔZ`) requires aiming lower, this value solves the full aerodynamic drag + gravity ballistic equations so your shell lands on the target's altitude.
  - Priority order: **SPH-2 Low Arc** $\rightarrow$ **SPH-2 High Arc** $\rightarrow$ **L81 Mortar** $\rightarrow$ planar ground distance fallback.
  - When `FLAT` map mode is selected, the header changes to **`RANGE`** and displays planar ground distance.
- **`COPY` Button**: Copies a formatted one-line firing solution to your clipboard for pasting into squad chat:
  - With terrain elevation: `Sight: 2615 m (Ground: 2345 m, ΔZ: +251 m), 254.1° (4517 mil)`
  - Flat mode: `2345 m, 254.1° (4517 mil)`
- **Compass Bearing (`254.1° WSW`)**: Clockwise azimuth from true north (`000.0°–360.0°`) alongside the 16-point cardinal compass direction (`N`, `NNE`, `NE`, …, `WSW`, etc.).
- **Azimuth in Mils (`4517 MIL`)**: Bearing converted to the standard NATO `6400` mil circle.
- **Compass Rose**: Visual azimuth indicator with a North (`N`) marker and amber needle pointing along the firing bearing.

### 7. Spatial & Terrain Telemetry
Detailed geometric breakdown below the main sight readout:
- **`GROUND: 2345 m`**: Horizontal 2D planar distance between **ME** and **TARGET**.
- **`3D: 2358 m`**: Direct line-of-sight slant distance ($\sqrt{\text{GROUND}^2 + \Delta Z^2}$).
- **`ΔZ: +251 m (UPHILL)`**: Vertical height difference ($\text{Target ALT} - \text{Me ALT}$) in metres, paired with a slope badge:
  - **`(UPHILL)`** *(Amber)*: Target is $> 0.5\text{ m}$ above your gun.
  - **`(DOWNHILL)`** *(Green)*: Target is $> 0.5\text{ m}$ below your gun.
  - **`(LEVEL)`** *(Muted)*: Height difference is within $\pm 0.5\text{ m}$.
  - **`(FLAT / NO TERRAIN)`**: Shown when `FLAT` mode is active or coordinates lie outside the terrain grid.
- **`ΔE -2255 m ΔN -642 m`**: Signed East/West ($\Delta E$) and North/South ($\Delta N$) distance components in metres.

### 8. Weapon Ballistic Solutions (`L81 MORTAR` & `SPH-2`)
Weapon-specific firing table solutions accounting for both horizontal range and altitude difference ($\Delta Z$):
- **`L81 MORTAR`** *(Engagement envelope: `132 m – 684 m`)*:
  - When in range: Displays `<Sight Range> m · <Elevation> MIL · <Flight Time>s`.
  - When out of envelope: Displays `TOO CLOSE · MIN 132 m`, `OUT OF RANGE · MAX 684 m`, or `UNREACHABLE (ELEVATION)`.
- **`SPH-2`** *(155mm Howitzer — Engagement envelope: `780 m – 2629 m`, Low Arc from `1181 m`)*:
  - When both arcs can reach: Displays `LOW <Sight>m (<mil>mil) / HIGH <Sight>m (<mil>mil)` (e.g. `LOW 2615m (504mil) / HIGH 2467m (856mil)`).
  - When only one arc can reach: Displays `<ARC> <Sight> m · <Elevation> MIL · <Flight Time>s`.
  - When out of envelope: Displays `TOO CLOSE · MIN 780 m`, `OUT OF RANGE · MAX 2629 m`, or `UNREACHABLE (ELEVATION)`.

### 9. Quick Actions & Clipboard Audit Log
- **`CLEAR ALL`**: Clears both **ME** and **TARGET** cards and resets the clipboard routing state.
- **`CLEAR TARGET`**: Clears only **TARGET** while keeping **ME** locked so you can immediately copy a new target.
- **`⇅ SWAP`**: Swaps the **ME** and **TARGET** coordinates (useful for reverse-azimuth or counter-battery checks).
- **`LAST COPY` Status Line (`LAST COPY x75.28, y102.90 → TARGET`)**: Confirms the last coordinate read from the clipboard and which card (`→ ME` or `→ TARGET`) received it.

### 10. Map Settings (`▸ MAP SETTINGS`)
A collapsible footer for custom map calibration:
- **`SCALE` (`100 m / unit`)**: Drag or type to adjust how many metres equal `1.0` coordinate unit (in *WARDOGS*, `1` unit = `100 m`, so `0.01` = `1 m`).
- **`RESET` Button**: Restores the coordinate scale to the standard `100 m / unit`.

---

## Features

- **Ballistic Sight Range & Weapon Solutions**:
  - **Dynamic Sight Dial (Large Readout)**: In *WARDOGS*, artillery sights are dialed to sight range (or mils). When firing uphill or downhill, the required sight range differs substantially from the planar ground distance. The calculator simulates aerodynamic drag, projectile velocity, and gravity to compute the exact range to dial on your gun's sight drum.
    - *Example (SPH-2 on Bakurani)*: At 2,345 m ground distance with target +251 m uphill, the calculator dials **2,615 m** (504 mil) on Low Arc. On Flat ground, it dials **2,345 m** (325 mil).
  - **SPH-2 (155mm HE)**: Provides both **Low Arc** and **High Arc** solutions with sight meters, mil elevation, and flight time in seconds.
  - **L81 Mortar (81mm)**: Provides RNG scale sight range, mils, and flight time within mortar engagement envelope (`132 m – 684 m`).
- **Map Selection & Offline Elevation Grids**:
  - Segmented map selector supporting **BAKURANI**, **OZETI**, **ZESTAFONA**, and **FLAT** (flat-earth / 2D-only mode).
  - Pre-baked $512 \times 512$ decimeter elevation rasters embedded directly into the binary via `include_bytes!`. Completely offline, zero runtime I/O, no network calls, and single-file portable distribution (~7.3 MB executable).
  - Sub-grid $O(1)$ bilinear interpolation for smooth altitude sampling across the full $16.384 \times 16.384\text{ km}$ map space ($X, Y \in [0.0, 163.84]$).
  - Live altitude readout (`ALT {z} m`) on both **ME** and **TARGET** cards when a terrain map is active.
- **Spatial Firing Context**:
  - **Horizontal Ground Distance** in metres.
  - **3D Slant Range** ($\sqrt{\text{Range}^2 + \Delta Z^2}$) and **Height Difference** ($\Delta Z$) with tactical trajectory indicators: `(UPHILL)`, `(DOWNHILL)`, or `(LEVEL)`.
  - **Compass Bearing** in degrees (`000.0°–360.0°`), 16-point cardinal direction, and **NATO mils** (`6400` mil circle) with an interactive compass rose.
  - **East / North Offsets** (`ΔE` / `ΔN`) in metres.
- **Automatic Clipboard Listener**:
  - Watches the system clipboard via native OS change events ([`clipboard-rs`](https://crates.io/crates/clipboard-rs)) with zero background polling, ignoring unrelated copied text.
  - Automatically fills **ME** when empty, then routes all subsequent coordinate copies to **TARGET**.
  - Toggle listening on/off at any time with the `● LISTENING` / `○ PAUSED` button.
- **Target Management**:
  - Save, name, and recall targets via a quick-access dropdown on the target card.
  - Delete or overwrite targets with a single click.
- **Quick Controls & UI**:
  - Individual `CLEAR` buttons on both **ME** and **TARGET** cards, plus `CLEAR ALL`, `CLEAR TARGET`, and `⇅ SWAP`.
  - One-click `COPY` button formats a complete tactical firing report to clipboard (e.g. `Sight: 2615 m (Ground: 2345 m, ΔZ: +251 m), 254.1° (4517 mil)`).
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

To rebuild or update the pre-baked $512 \times 512$ elevation binary grids from the 8 m resolution terrain heightfield blocks ($2049 \times 2049$ samples), an offline Python generator script is included:

```powershell
python scripts/generate_elevation_grids.py
```

Outputs 524,288-byte little-endian `i16` decimeter rasters directly into `src/assets/terrain/`.
