# WARDOGS // Artillery Range Calculator

A lightweight desktop range and bearing calculator for **WARDOGS** artillery and mortar crews, built in Rust with [`egui`](https://github.com/emilk/egui) / [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe).

It watches your system clipboard for in-game coordinates (such as `x95.62, y104.98`) and automatically populates your gun position (**ME**) and impact point (**TARGET**), computing the firing solution immediately.

---

## Features

- **Automatic Clipboard Listener**:
  - Monitors the system clipboard in the background (`150 ms` polling) and ignores unrelated copied text.
  - **Routing Modes**:
    - `AUTO`: Fills **ME** when empty, then routes all subsequent coordinate copies to **TARGET**.
    - `ME`: Always routes copied coordinates to your gun position.
    - `TARGET`: Always routes copied coordinates to the target position.
  - Toggle listening on/off at any time with the `● LISTENING` / `○ PAUSED` button.
- **Instant Firing Solution**:
  - **Horizontal Range** in metres (`1` game grid unit = `100 m` by default, configurable in Map Settings).
  - **Compass Bearing** in degrees (`000.0°–360.0°`), 16-point cardinal direction, and **NATO mils** (`6400` mil circle) with an interactive compass rose.
  - **East / North Offsets** (`ΔE` / `ΔN`) in metres.
  - **Weapon Reach Envelopes**:
    - **L81 Mortar**: `132 m – 684 m` (High Arc)
    - **SPH-2**: `780 m – 2,629 m` (High Arc from `780 m`, Low Arc from `1,181 m`)
- **Quick Controls**:
  - Individual `CLEAR` buttons on both **ME** and **TARGET** cards, plus `CLEAR ALL`, `CLEAR TARGET`, and `⇅ SWAP`.
  - `PIN` toggle inside the **ME** card to keep the calculator window always-on-top while playing.
  - `COPY` button on the firing solution to copy range, degrees, and mils to the clipboard.
  - Persistent window position, coordinates, and map settings across restarts.

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

- [Rust](https://rustup.rs/) `1.90` or newer

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
