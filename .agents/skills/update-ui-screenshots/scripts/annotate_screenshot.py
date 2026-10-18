"""
Generates assets/screenshot_annotated.png by compositing assets/screenshot.png
onto a dark tactical canvas with numbered callout lines and descriptions.
"""

from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

REPO_ROOT = Path(__file__).resolve().parents[4]
SCREENSHOT_PATH = REPO_ROOT / "assets" / "screenshot.png"
ANNOTATED_PATH = REPO_ROOT / "assets" / "screenshot_annotated.png"

AMBER = (242, 166, 30, 255)
GREEN = (84, 214, 110, 255)
CYAN = (56, 208, 214, 255)
RED = (237, 85, 85, 255)
WHITE = (232, 236, 233, 255)
MUTED = (145, 156, 149, 255)
DARK = (12, 14, 13, 255)

CALLOUTS = [
    (1, 381, 63, AMBER, "MAP SELECTOR", "2048x2048 8m terrain grid or FLAT"),
    (2, 541, 109, GREEN, "CLIPBOARD STATUS", "Toggle OS event listener"),
    (3, 541, 205, CYAN, "ME (GUN POSITION)", "Coords, East/North & ASL altitude"),
    (4, 541, 350, RED, "TARGET (IMPACT PT)", "Auto-routed on next copy"),
    (5, 541, 456, AMBER, "SAVED TARGETS", "Preset dropdown, SAVE & DEL"),
    (6, 541, 565, AMBER, "GROUND DIST & AZIMUTH", "Ground m, Azimuth°, MIL & COPY"),
    (7, 495, 626, WHITE, "TERRAIN TELEMETRY", "GUN/TGT ASL, RISE slope, 3D & ΔE/ΔN"),
    (8, 541, 720, GREEN, "WEAPON SOLUTIONS", "L81 & SPH-2 MIL, Flight s, HUD RNG & LOS"),
    (9, 326, 792, AMBER, "QUICK ACTIONS & LOG", "Clear, Swap & Last Copy audit"),
    (10, 150, 862, WHITE, "MAP SETTINGS", "Coordinate scale (100m/unit)"),
]


def main() -> None:
    base = Image.open(SCREENSHOT_PATH).convert("RGBA")
    _, bh = base.size

    pad_left = 16
    pad_top = 16
    pad_bottom = 16
    canvas_w = 940
    canvas_h = bh + pad_top + pad_bottom

    out = Image.new("RGBA", (canvas_w, canvas_h), DARK)
    out.paste(base, (pad_left, pad_top))

    draw = ImageDraw.Draw(out)
    font_bold = ImageFont.truetype("C:/Windows/Fonts/consolab.ttf", 14)
    font_reg = ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 12)
    font_num = ImageFont.truetype("C:/Windows/Fonts/consolab.ttf", 14)

    box_x = 602
    box_w = 26
    box_h = 22

    for num, sx, sy, col, title, desc in CALLOUTS:
        ax = pad_left + sx
        ay = pad_top + sy
        r = 3
        draw.ellipse((ax - r, ay - r, ax + r, ay + r), fill=col)
        draw.line([(ax, ay), (box_x, ay)], fill=col, width=2)
        bx0, by0 = box_x, ay - box_h // 2
        bx1, by1 = box_x + box_w, by0 + box_h
        draw.rectangle([bx0, by0, bx1, by1], fill=col)
        num_str = str(num)
        bbox = draw.textbbox((0, 0), num_str, font=font_num)
        tw, th = bbox[2] - bbox[0], bbox[3] - bbox[1]
        draw.text(
            (bx0 + (box_w - tw) / 2, by0 + (box_h - th) / 2 - 2),
            num_str,
            fill=DARK,
            font=font_num,
        )
        tx = bx1 + 10
        draw.text((tx, ay - 14), title, fill=col, font=font_bold)
        draw.text((tx, ay + 2), desc, fill=MUTED, font=font_reg)

    out.save(ANNOTATED_PATH)
    print(f"Saved {ANNOTATED_PATH}")


if __name__ == "__main__":
    main()
