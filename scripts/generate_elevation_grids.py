#!/usr/bin/env python3
"""
Pre-bakes 512x512 elevation rasters for WARDOGS maps (Bakurani, Ozeti, Zestafona)
from open-source community contour models.

Each output file is a raw binary array of 512x512 little-endian signed 16-bit integers
(524,288 bytes) representing elevation in decimeters (0.1 m resolution), spanning
game coordinates X in [0.0, 163.2] and Y in [0.0, 163.2].
"""

import json
import os
import sys
import time
import urllib.request
import numpy as np
import matplotlib.tri as mtri
from scipy.ndimage import distance_transform_edt

GRID_SIZE = 512
MAX_COORD = 163.2

MAPS = {
    "bakurani": "https://raw.githubusercontent.com/apollyon-sys/wardogs-calculator/main/data/terrain/bakurani/contours.json",
    "ozeti": "https://raw.githubusercontent.com/apollyon-sys/wardogs-calculator/main/data/terrain/ozeti/contours.json",
    "zestafona": "https://raw.githubusercontent.com/apollyon-sys/wardogs-calculator/main/data/terrain/zestafona/contours.json",
}

OUT_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "src", "assets", "terrain")

def generate_map(map_name, url):
    print(f"Generating elevation grid for '{map_name}'...")
    t0 = time.time()
    
    # 1. Download and parse contours.json
    req = urllib.request.Request(url, headers={"User-Agent": "wardogs-calc-generator/1.0"})
    with urllib.request.urlopen(req) as resp:
        data = json.loads(resp.read().decode("utf-8"))
    
    grid = data["grid"]
    origin_x = float(grid["originX"])
    origin_y = float(grid["originY"])
    step_x = float(grid["stepX"])
    step_y = float(grid["stepY"])
    quantisation = float(data.get("quantisation", 10))
    
    # 2. Extract contour vertices (px, py, z)
    xs, ys, zs = [], [], []
    for level in data["levels"]:
        z = float(level["relativeMeters"])
        for flat in level.get("lines", []):
            x = 0
            y = 0
            for i in range(0, len(flat), 2):
                x += flat[i]
                y += flat[i + 1]
                px = origin_x + (x / quantisation) * step_x
                py = origin_y - (y / quantisation) * step_y
                xs.append(px)
                ys.append(py)
                zs.append(z)
                
    xs = np.array(xs, dtype=np.float32)
    ys = np.array(ys, dtype=np.float32)
    zs = np.array(zs, dtype=np.float32)
    print(f"  Parsed {len(xs)} contour points in {time.time() - t0:.2f}s")
    
    # 3. Deduplicate points to clean up triangulation
    coords = np.round(np.column_stack([xs, ys]), 2)
    _, u_idx = np.unique(coords, axis=0, return_index=True)
    xs_u, ys_u, zs_u = xs[u_idx], ys[u_idx], zs[u_idx]
    
    # 4. Triangulate and interpolate
    tri = mtri.Triangulation(xs_u, ys_u)
    interp = mtri.LinearTriInterpolator(tri, zs_u)
    
    gx = np.linspace(0.0, MAX_COORD, GRID_SIZE, dtype=np.float32)
    gy = np.linspace(0.0, MAX_COORD, GRID_SIZE, dtype=np.float32)
    grid_x, grid_y = np.meshgrid(gx, gy)
    
    z_grid = interp(grid_x, grid_y)
    
    # 5. Fill masked values (outside contour bounds) using nearest-neighbor
    if np.ma.is_masked(z_grid):
        mask = z_grid.mask
        z_data = z_grid.data.copy()
        z_data[mask] = np.nan
        indices = distance_transform_edt(mask, return_distances=False, return_indices=True)
        z_filled = z_data[tuple(indices)]
    else:
        z_filled = np.array(z_grid, dtype=np.float32)
        
    # Check for any remaining NaNs
    if np.isnan(z_filled).any():
        nan_mask = np.isnan(z_filled)
        indices = distance_transform_edt(nan_mask, return_distances=False, return_indices=True)
        z_filled = z_filled[tuple(indices)]
        
    print(f"  Elevation range: min={z_filled.min():.1f}m, max={z_filled.max():.1f}m")
    
    # 6. Convert to decimeters (i16) and write binary
    z_decimeters = np.clip(np.round(z_filled * 10.0), -32768, 32767).astype("<i2")
    
    os.makedirs(OUT_DIR, exist_ok=True)
    out_path = os.path.join(OUT_DIR, f"{map_name}.bin")
    with open(out_path, "wb") as f:
        f.write(z_decimeters.tobytes())
        
    file_size = os.path.getsize(out_path)
    print(f"  Wrote {file_size} bytes to {out_path} (total time: {time.time() - t0:.2f}s)")
    assert file_size == GRID_SIZE * GRID_SIZE * 2, f"Unexpected file size: {file_size}"

def main():
    for name, url in MAPS.items():
        generate_map(name, url)
    print("\nAll elevation rasters generated successfully!")

if __name__ == "__main__":
    main()
