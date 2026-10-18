---
name: update-ui-screenshots
description: >-
  Use this skill whenever UI fields, labels, layout, or styling in src/app.rs, src/chrome.rs, or src/theme.rs change and assets/screenshot.png or assets/screenshot_annotated.png need to be regenerated.
---

# Update UI & Annotated Interface Screenshots

Whenever the UI layout or displayed fields change, both `assets/screenshot.png` and `assets/screenshot_annotated.png` (referenced in `README.md`) must be kept in sync.

## Steps

### 1. Build Release Binary & Capture Base Screenshot (`assets/screenshot.png`)

1. Build the release executable:
   ```powershell
   cargo build --release
   ```
2. Seed `%APPDATA%\wardogs-range-calc\data\app.ron` with the reference fire mission (`ME = "x97.99, y109.57"`, `TARGET = "x95.62, y104.98"`, `active_map = Bakurani`, saved target `"High Outpost"`), launch `target/release/wardogs-range-calc.exe`, capture the window client area (`560x900`) to `assets/screenshot.png`, and stop the process.

### 2. Generate Annotated Callout Guide (`assets/screenshot_annotated.png`)

Run the annotation generator script to composite `assets/screenshot.png` onto a dark canvas with numbered callout lines (`1`–`10`) and descriptions:

```powershell
python .agents/skills/update-ui-screenshots/scripts/annotate_screenshot.py
```

### 3. Verify Visual Alignment

Use `view_file` to inspect both [`assets/screenshot.png`](../../../assets/screenshot.png) and [`assets/screenshot_annotated.png`](../../../assets/screenshot_annotated.png):
- Confirm the window is not clipped and shows the reference firing solution (`517 m`, `207.3° SSW`, `GUN ASL -863 m · TGT ASL -786 m · RISE +76 m (UPHILL)`).
- Confirm all 10 numbered callout dots in `assets/screenshot_annotated.png` point accurately to their corresponding UI elements. If window vertical spacing changed, adjust the `sy` coordinates in [`scripts/annotate_screenshot.py`](./scripts/annotate_screenshot.py) and re-run.
