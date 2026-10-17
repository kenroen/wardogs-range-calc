#!/usr/bin/env python3
"""
Pre-bakes 512x512 elevation rasters for WARDOGS maps (Bakurani, Ozeti, Zestafona)
from the full 8 m resolution terrain heightfield blocks (2049x2049 samples over
[0.0, 163.84] coordinate units).

Each output file is a raw binary array of 512x512 little-endian signed 16-bit integers
(524,288 bytes) representing elevation in decimeters (0.1 m resolution), spanning
game coordinates X in [0.0, 163.84] and Y in [0.0, 163.84].
"""

import gzip
import os
import time
import urllib.request
from concurrent.futures import ThreadPoolExecutor
import numpy as np
from scipy.interpolate import RegularGridInterpolator
from scipy.ndimage import distance_transform_edt

GRID_SIZE = 512
MAX_COORD = 163.84
BLOCKS_PER_AXIS = 16
BLOCK_SAMPLES = 129
FULL_GRID_SAMPLES = BLOCKS_PER_AXIS * (BLOCK_SAMPLES - 1) + 1  # 2049

MAPS = {
    "bakurani": "bakurani",
    "ozeti": "europe",
    "zestafona": "north-america",
}

OUT_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "src", "assets", "terrain")


def fetch_block(args):
    cdn_id, bx, by = args
    url = f"https://cdn.wardogshub.gg/terrain/{cdn_id}/v1/{bx}_{by}.bin"
    req = urllib.request.Request(url, headers={"User-Agent": "wardogs-calc-generator/2.0"})
    for attempt in range(3):
        try:
            with urllib.request.urlopen(req, timeout=15) as resp:
                raw = resp.read()
            if raw[:2] == b"\x1f\x8b":
                raw = gzip.decompress(raw)
            arr = np.frombuffer(raw, dtype="<u2").reshape((BLOCK_SAMPLES, BLOCK_SAMPLES))
            return bx, by, arr
        except Exception as e:
            if attempt == 2:
                raise RuntimeError(f"Failed to fetch {url}: {e}") from e
            time.sleep(0.5 * (attempt + 1))


def generate_map(map_name, cdn_id):
    print(f"Generating elevation grid for '{map_name}' (CDN '{cdn_id}')...")
    t0 = time.time()

    tasks = [(cdn_id, bx, by) for by in range(BLOCKS_PER_AXIS) for bx in range(BLOCKS_PER_AXIS)]
    raw_grid = np.zeros((FULL_GRID_SAMPLES, FULL_GRID_SAMPLES), dtype=np.uint16)

    with ThreadPoolExecutor(max_workers=16) as pool:
        for bx, by, block in pool.map(fetch_block, tasks):
            x0 = bx * (BLOCK_SAMPLES - 1)
            y0 = by * (BLOCK_SAMPLES - 1)
            raw_grid[y0 : y0 + BLOCK_SAMPLES, x0 : x0 + BLOCK_SAMPLES] = block

    print(f"  Downloaded 256 blocks ({FULL_GRID_SAMPLES}x{FULL_GRID_SAMPLES}) in {time.time() - t0:.2f}s")

    # Convert encoded u16 to relative meters: 0 = no-data, >0 => (val - 1) * 0.25
    # Both block index `by` and local row index inside each block grow northwards (+Y),
    # so row 0 corresponds to Y = 0.0 (south) and row 2048 to Y = 163.84 (north).
    valid_mask = raw_grid > 0
    z_full = np.where(valid_mask, (raw_grid.astype(np.float32) - 1.0) * 0.25, np.nan)

    if not valid_mask.all():
        invalid_mask = ~valid_mask
        indices = distance_transform_edt(invalid_mask, return_distances=False, return_indices=True)
        z_full = z_full[tuple(indices)]

    # Resample onto 512x512 grid over [0.0, 163.84]
    src_axis = np.linspace(0.0, MAX_COORD, FULL_GRID_SAMPLES, dtype=np.float64)
    interp = RegularGridInterpolator((src_axis, src_axis), z_full, method="linear", bounds_error=True)

    dst_axis = np.linspace(0.0, MAX_COORD, GRID_SIZE, dtype=np.float64)
    gy, gx = np.meshgrid(dst_axis, dst_axis, indexing="ij")
    z_512 = interp((gy, gx)).astype(np.float32)

    print(f"  Elevation range: min={z_512.min():.1f}m, max={z_512.max():.1f}m")

    z_decimeters = np.clip(np.round(z_512 * 10.0), -32768, 32767).astype("<i2")

    os.makedirs(OUT_DIR, exist_ok=True)
    out_path = os.path.join(OUT_DIR, f"{map_name}.bin")
    with open(out_path, "wb") as f:
        f.write(z_decimeters.tobytes())

    file_size = os.path.getsize(out_path)
    print(f"  Wrote {file_size} bytes to {out_path} (total time: {time.time() - t0:.2f}s)")
    assert file_size == GRID_SIZE * GRID_SIZE * 2, f"Unexpected file size: {file_size}"


def main():
    for name, cdn_id in MAPS.items():
        generate_map(name, cdn_id)
    print("\nAll elevation rasters generated successfully!")


if __name__ == "__main__":
    main()
