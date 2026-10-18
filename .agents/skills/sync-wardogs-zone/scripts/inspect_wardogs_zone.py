"""
Fetches https://wardogs.zone/calculators/artillery, discovers all active Next.js
script chunks, and extracts the map elevation configs and artillery weapon models.
"""

import re
import urllib.request

BASE_URL = "https://wardogs.zone"
CALC_URL = f"{BASE_URL}/calculators/artillery"
HEADERS = {"User-Agent": "Mozilla/5.0"}


def fetch_text(url: str) -> str:
    req = urllib.request.Request(url, headers=HEADERS)
    with urllib.request.urlopen(req, timeout=20) as resp:
        return resp.read().decode("utf-8", errors="replace")


def main() -> None:
    print(f"Fetching {CALC_URL} ...")
    html = fetch_text(CALC_URL)
    chunks = sorted(set(re.findall(r"/_next/static/chunks/[^\"'<>]+\.js", html)))
    print(f"Found {len(chunks)} JS chunks.")

    for chunk_path in chunks:
        js = fetch_text(f"{BASE_URL}{chunk_path}")

        # 1. Check for map heightfield configs
        if "heightField" in js or "hudPlaneM" in js:
            print(f"\n=== Map Configs in {chunk_path} ===")
            for m in re.finditer(r"heightField:\s*\{[^}]+\}(?:,hudPlaneM:[-\d.]+)?", js):
                start = max(0, m.start() - 120)
                end = min(len(js), m.end() + 60)
                print(" ", js[start:end])

        # 2. Check for ballistic weapon definitions
        if "degPerMil" in js or "659e-6" in js or "148e-5" in js:
            print(f"\n=== Weapon Ballistics in {chunk_path} ===")
            idx = js.find("degPerMil")
            if idx != -1:
                start = max(0, idx - 600)
                end = min(len(js), idx + 1800)
                print(js[start:end])


if __name__ == "__main__":
    main()
