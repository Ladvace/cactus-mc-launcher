#!/usr/bin/env python3
"""Slice a sprite sheet into individual elements.

The sheet has its elements on a transparent background, so each element is a
connected blob of opaque pixels. We build an alpha mask, dilate it slightly so a
sprite stays joined to its little shadow / detached bits, label connected
components, and crop each one's bounding box. Outputs PNGs + a manifest.json.

Usage: python3 scripts/extract_sprites.py [sheet.png] [out_dir]
"""
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image

SHEET = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("art/desert-sheet.png")
OUT = Path(sys.argv[2]) if len(sys.argv) > 2 else Path("static/decor/sprites")

ALPHA_THRESH = 24  # pixel is "ink" if alpha above this
DILATE = 5         # merge parts within ~this many px (sprite + shadow, sparkles…)
MIN_AREA = 500     # drop specks below this many ink pixels
PAD = 3            # transparent padding around each crop


def dilate(mask: np.ndarray, r: int) -> np.ndarray:
    m = mask
    for _ in range(r):
        out = m.copy()
        out[1:, :] |= m[:-1, :]
        out[:-1, :] |= m[1:, :]
        out[:, 1:] |= m[:, :-1]
        out[:, :-1] |= m[:, 1:]
        m = out
    return m


def label(mask: np.ndarray):
    """Return (ys, xs, root) for every ink pixel via union-find over 4-neighbours."""
    h, w = mask.shape
    idx = -np.ones((h, w), dtype=np.int64)
    ys, xs = np.nonzero(mask)
    n = len(ys)
    idx[ys, xs] = np.arange(n)
    parent = np.arange(n, dtype=np.int64)

    def find(a: int) -> int:
        while parent[a] != a:
            parent[a] = parent[parent[a]]
            a = parent[a]
        return a

    def unite(a: int, b: int):
        ra, rb = find(a), find(b)
        if ra != rb:
            parent[rb] = ra

    hy, hx = np.nonzero(mask[:, :-1] & mask[:, 1:])
    for a, b in zip(idx[hy, hx], idx[hy, hx + 1]):
        unite(int(a), int(b))
    vy, vx = np.nonzero(mask[:-1, :] & mask[1:, :])
    for a, b in zip(idx[vy, vx], idx[vy + 1, vx]):
        unite(int(a), int(b))

    root = np.array([find(i) for i in range(n)], dtype=np.int64)
    return ys, xs, root


def main():
    img = Image.open(SHEET).convert("RGBA")
    arr = np.asarray(img)
    h, w = arr.shape[:2]
    mask = arr[:, :, 3] > ALPHA_THRESH

    ys, xs, root = label(dilate(mask, DILATE))

    # Group pixel coords by component root.
    order = np.argsort(root)
    ys, xs, root = ys[order], xs[order], root[order]
    boundaries = np.nonzero(np.diff(root))[0] + 1
    groups = np.split(np.arange(len(root)), boundaries)

    boxes = []
    for g in groups:
        if len(g) < MIN_AREA:
            continue
        gy, gx = ys[g], xs[g]
        boxes.append((gy.min(), gx.min(), gy.max() + 1, gx.max() + 1, len(g)))

    # Stable order: top-to-bottom in rows, then left-to-right.
    boxes.sort(key=lambda b: (round(b[0] / 40), b[1]))

    OUT.mkdir(parents=True, exist_ok=True)
    for f in OUT.glob("*.png"):
        f.unlink()

    manifest = []
    for i, (y0, x0, y1, x1, area) in enumerate(boxes):
        y0, x0 = max(0, y0 - PAD), max(0, x0 - PAD)
        y1, x1 = min(h, y1 + PAD), min(w, x1 + PAD)
        crop = img.crop((x0, y0, x1, y1))
        name = f"{i:02d}.png"
        crop.save(OUT / name)
        manifest.append(
            {"id": i, "file": name, "w": int(x1 - x0), "h": int(y1 - y0), "area": int(area)}
        )

    (OUT / "manifest.json").write_text(json.dumps(manifest, indent=2))
    print(f"{len(manifest)} sprites → {OUT}")
    for m in manifest:
        print(f"  {m['file']}  {m['w']}x{m['h']}  area={m['area']}")


if __name__ == "__main__":
    main()
