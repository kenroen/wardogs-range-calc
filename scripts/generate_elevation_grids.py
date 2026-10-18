#!/usr/bin/env python3
"""
Downloads and encodes the 2048x2048 terrain heightfields for WARDOGS maps
(Bakurani, Ozeti, Zestafona) from wardogs.zone.

Each output file is a 2D-delta + byte-transposed + zlib-compressed binary stream
of the 2048x2048 uint16 heightfield cells, matching wardogs.zone's exact
terrain heightfields cell-for-cell.
"""

import io
import os
import time
import urllib.request
import zlib
import numpy as np
from PIL import Image

GRID_SIZE = 2048

MAPS = {
    "bakurani": "https://wardogs.zone/game/maps/height/kavkazi.e92f4871.webp",
    "ozeti": "https://wardogs.zone/game/maps/height/europe.a1ac244c.webp",
    "zestafona": "https://wardogs.zone/game/maps/height/northamerica.c5373db8.webp",
}

OUT_DIR = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
    "src",
    "assets",
    "terrain",
)


def generate_map(map_name: str, url: str) -> None:
    print(f"Fetching 2048x2048 heightfield for '{map_name}' from {url}...")
    t0 = time.time()
    req = urllib.request.Request(url, headers={"User-Agent": "wardogs-range-calc/2.0"})
    with urllib.request.urlopen(req, timeout=30) as resp:
        raw = resp.read()

    img = Image.open(io.BytesIO(raw)).convert("RGBA")
    assert img.size == (GRID_SIZE, GRID_SIZE), f"Unexpected size: {img.size}"

    arr = np.array(img, dtype=np.uint16)
    cells = ((arr[:, :, 0] << 8) | arr[:, :, 1]).astype("<i2")

    # 2D delta encoding (horizontal then vertical)
    d1 = np.empty_like(cells)
    d1[:, 0] = cells[:, 0]
    d1[:, 1:] = cells[:, 1:] - cells[:, :-1]

    d2 = np.empty_like(d1)
    d2[0, :] = d1[0, :]
    d2[1:, :] = d1[1:, :] - d1[:-1, :]

    u_d2 = d2.view("<u2").ravel()
    transposed = np.concatenate([u_d2 & 0xFF, u_d2 >> 8]).astype(np.uint8)
    compressed = zlib.compress(transposed.tobytes(), level=9)

    os.makedirs(OUT_DIR, exist_ok=True)
    out_path = os.path.join(OUT_DIR, f"{map_name}.bin")
    with open(out_path, "wb") as f:
        f.write(compressed)

    print(
        f"  Wrote {len(compressed):,} bytes to {out_path} in {time.time() - t0:.2f}s"
    )


def main() -> None:
    for name, url in MAPS.items():
        generate_map(name, url)
    print("\nAll 2048x2048 elevation rasters generated successfully!")


if __name__ == "__main__":
    main()
