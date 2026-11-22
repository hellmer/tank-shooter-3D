"""Pinta la textura del suelo: tierra seca con manchas, piedrecitas y matas de hierba.

64x64 texels que se repiten sin costuras; a 0.1 m por texel coincide con la escala de los
modelos de 3dixel (1 unidad = 1 texel = 0.1 m).

    python3 art/terreno/ground.py assets/textures/ground.png
"""
import sys

import numpy as np
from PIL import Image

N = 64
DIRT = ["#2a2519", "#332d1e", "#3d3523", "#473e29", "#52482f", "#5e5336", "#6b5f3e", "#7b6e49"]
GRASS = ["#2b3318", "#38421d", "#475424", "#58662c", "#6c7a37"]
PEBBLE = ["#3a3836", "#55524d", "#77736b", "#9b958a"]


def hexc(h):
    h = h.lstrip("#")
    return np.array([int(h[i:i + 2], 16) for i in (0, 2, 4)], np.uint8)


def periodic_noise(rng, waves=6):
    """Smooth noise that tiles: a sum of waves with whole-number frequencies."""
    y, x = np.mgrid[0:N, 0:N] / N * 2 * np.pi
    f = np.zeros((N, N))
    for _ in range(waves):
        kx, ky = rng.integers(-3, 4, 2)
        if kx == 0 and ky == 0:
            continue
        f += np.sin(kx * x + ky * y + rng.uniform(0, 2 * np.pi)) / np.hypot(kx, ky)
    f -= f.min()
    return f / max(f.max(), 1e-6)


def main(out):
    rng = np.random.default_rng(42)
    big = periodic_noise(rng, 8)
    small = periodic_noise(rng, 10)
    # Dirt in broad patches, with a narrow dithered seam only where two tones meet.
    level = big * 3.2 + small * 1.6
    idx = np.floor(level).astype(int) + 1
    frac = level - np.floor(level)
    y, x = np.mgrid[0:N, 0:N]
    dither = (frac > 0.82) & ((x + y) % 2 == 0)
    idx = np.clip(idx + dither, 0, len(DIRT) - 1)
    img = np.array([hexc(c) for c in DIRT])[idx]

    # Pebbles: 1 or 2 texels, light on the upper left, a shadow texel under the right side.
    taken = np.zeros((N, N), bool)
    for _ in range(26):
        py, px = rng.integers(0, N, 2)
        if taken[py, px]:
            continue
        size = 2 if rng.random() < 0.4 else 1
        tone = 1 + int(rng.random() < 0.5)
        for dy in range(size):
            for dx in range(size):
                img[(py + dy) % N, (px + dx) % N] = hexc(PEBBLE[tone + (1 if dy == 0 and dx == 0 else 0)])
                taken[(py + dy) % N, (px + dx) % N] = True
        img[(py + size) % N, (px + size - 1) % N] = hexc(PEBBLE[0])

    # Dry grass in a few clumps where the dirt is dark: blades 1 texel wide, 2-4 long, light tips.
    grassy = big > 0.62
    for _ in range(120):
        py, px = rng.integers(0, N, 2)
        if not grassy[py, px] or taken[py, px]:
            continue
        length = int(rng.integers(2, 5))
        for k in range(length):
            yy = (py - k) % N
            tone = min(len(GRASS) - 1, 1 + k)
            img[yy, px] = hexc(GRASS[tone])
            taken[yy, px] = True

    Image.fromarray(img, "RGB").save(out)
    print(f"wrote {out}")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "assets/textures/ground.png")
