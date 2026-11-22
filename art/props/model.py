"""Escenario: caja militar (destructible), roca y barrera de hormigón (cobertura fija).

Cada objeto se construye con su base en el suelo y centrado en el origen; build_parts() los
pone uno al lado del otro solo para la vista previa.
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "common"))
from paint import *  # noqa: E402,F401,F403
from paint import np  # noqa: E402
from kit import light_map  # noqa: E402

OLIVE = [hexc(c) for c in ("#14170f", "#1f2416", "#2b321d", "#384126", "#465130", "#56613b", "#6b7549", "#858c5d", "#a3a679")]
STONE = [hexc(c) for c in ("#17181d", "#22232a", "#2e3038", "#3b3d45", "#4a4b52", "#5a5a5f", "#6d6b6d", "#84807c", "#a09a91", "#bdb5a8")]
CONCRETE = [hexc(c) for c in ("#2a2b2f", "#38393d", "#47484b", "#57585a", "#68686a", "#7a7a79", "#8e8c89", "#a39f9a", "#b9b4ac")]
STENCIL = hexc("#d8d2bd")


# ---------------------------------------------------------------- crate

def crate_body(face, h, w):
    """Olive planks with a darker steel band and a stenciled mark on the big sides."""
    y, x = np.mgrid[0:h, 0:w]
    lit = np.round(0.7 - 1.2 * y / max(h - 1, 1) - 0.6 * x / max(w - 1, 1) + 0.4).astype(int)
    idx = 4 + FACE_LIGHT[face] + lit
    if face in ("top", "bottom"):
        plank = (x // 4) % 2
        seam = x % 4 == 0
    else:
        plank = (y // 4) % 2
        seam = y % 4 == 0
    idx = idx + plank * 0 - (plank == 1) * 1 * (x % 7 == 3) * 0
    idx = np.where(seam, idx - 2, idx)
    # Wood grain: short darker streaks along each plank, placed on purpose every few texels.
    grain = ((x * 3 + y * 7) % 11 == 0) & ~seam
    idx = np.where(grain, idx - 1, idx)
    img = rgba(idx, OLIVE)
    if face in ("front", "back", "left", "right") and h >= 12:
        # Stencil: a hollow square with a cross, kept clear of the edges.
        cy, cx = h // 2, w // 2
        r = 3
        box_ = (np.abs(y - cy) <= r) & (np.abs(x - cx) <= r) & ((np.abs(y - cy) == r) | (np.abs(x - cx) == r))
        cross = ((y == cy) | (x == cx)) & (np.abs(y - cy) <= 1) & (np.abs(x - cx) <= 1)
        m = box_ | cross
        img[m, :3] = STENCIL[None, :] * (0.85 + 0.15 * (y[m] < cy))[:, None]
    return img


def crate_post(face, h, w):
    y, x = np.mgrid[0:h, 0:w]
    idx = 3 + FACE_LIGHT[face] + np.round(0.5 - 1.2 * y / max(h - 1, 1)).astype(int)
    idx = np.where(y % 5 == 2, idx + 2, idx)  # bolt heads
    return rgba(idx, STEEL)


def crate_parts(size=24):
    h = size / 2
    P = [box((-h, 0, -h), (h, size, h), "crate", crate_body)]
    for sx in (-1, 1):
        for sz in (-1, 1):
            cx, cz = sx * (h - 0.5), sz * (h - 0.5)
            P.append(box((cx - 1.5, 0, cz - 1.5), (cx + 1.5, size + 1, cz + 1.5), "crate", crate_post))
    return P


# ---------------------------------------------------------------- rock

def rock_painter(seed):
    """Weathered stone: a soft light field, tilted strata every few rows with a lit lip under
    each one, and a couple of cracks. Low contrast so it does not turn into gravel."""
    def paint(face, h, w):
        rng = np.random.default_rng(seed * 7 + FACES.index(face))
        y, x = np.mgrid[0:h, 0:w]
        L = light_map(h, w, seed + FACES.index(face), lx=0.6, ly=0.8)
        base = 3 + {"top": 1, "bottom": -2, "back": -1, "right": -1}.get(face, 0)
        idx = base + np.round(L * 3).astype(int)
        lean = int(rng.integers(3, 6))
        strata = (y + x // lean) % 6
        idx = np.where(strata == 0, idx - 1, idx)
        idx = np.where(strata == 1, idx + 1, idx)
        for _ in range(2 if h * w > 80 else 1):
            cy, cx = int(rng.integers(1, max(2, h - 1))), int(rng.integers(0, w))
            for k in range(min(6, h - cy)):
                xx = min(w - 1, max(0, cx + (k // 2) * (1 if seed % 2 else -1)))
                idx[cy + k, xx] -= 2
        return rgba(idx, STONE)
    return paint


def shift(p, dx, dy, dz):
    """Moves a part (box and every pivot) without changing its shape."""
    p["from"] = (p["from"][0] + dx, p["from"][1] + dy, p["from"][2] + dz)
    p["to"] = (p["to"][0] + dx, p["to"][1] + dy, p["to"][2] + dz)
    p["tr"] = [((pv[0] + dx, pv[1] + dy, pv[2] + dz), r) for pv, r in p["tr"]]
    return p


def grounded(p, sink=0.0):
    """Moves a tilted part up or down so its lowest corner sits `sink` below the ground."""
    lowest = min(c[1] for c in render_corners(p))
    return shift(p, 0, -lowest - sink, 0)


def render_corners(p):
    from render import corners
    return corners(p)


def rock_parts():
    g = "rock"
    # A big tilted boulder with two smaller ones leaning on it, each sunk a little into the ground.
    big = box((-12, 0, -10), (12, 16, 10), g, rock_painter(3), [((0, 8, 0), (8, 25, -6))])
    mid = box((4, 0, 0), (16, 10, 12), g, rock_painter(5), [((10, 5, 6), (-12, 40, 14))])
    small = box((-17, 0, 4), (-9, 7, 12), g, rock_painter(7), [((-13, 3.5, 8), (10, -30, -8))])
    return [grounded(p, 1.0) for p in (big, mid, small)]


# ---------------------------------------------------------------- barrier

def concrete_painter(seed, stripe=False):
    def paint(face, h, w):
        rng = np.random.default_rng(seed + FACES.index(face))
        y, x = np.mgrid[0:h, 0:w]
        lit = np.round(0.8 - 1.4 * y / max(h - 1, 1) - 0.6 * x / max(w - 1, 1) + 0.5).astype(int)
        idx = 4 + FACE_LIGHT[face] + lit
        # Formwork lines and a few chipped spots, placed on purpose.
        idx = np.where(x % 10 == 0, idx - 1, idx)
        for _ in range(max(1, h * w // 60)):
            cy, cx = int(rng.integers(0, h)), int(rng.integers(0, w))
            idx[cy, cx] -= 2
            if cx + 1 < w:
                idx[cy, cx + 1] += 1
        img = rgba(idx, CONCRETE)
        if stripe and face in ("front", "back") and h >= 6:
            # Yellow and black hazard stripes along the top band.
            band = (y >= 1) & (y <= 3)
            yellow = ((x + y) // 3) % 2 == 0
            img[band & yellow, :3] = hexc("#d8a52a")
            img[band & ~yellow, :3] = hexc("#1b1b1d")
        return img
    return paint


def barrier_parts():
    g = "barrier"
    P = []
    # Jersey barrier: a wide foot, two sloped faces and a narrow top, 40 long.
    P.append(box((-20, 0, -5), (20, 4, 5), g, concrete_painter(11)))
    slope = box((-20, 3, 0), (20, 12, 3), g, concrete_painter(13, stripe=True), [((0, 3, 3), (-14, 0, 0))])
    P.append(slope)
    back = box((-20, 3, -3), (20, 12, 0), g, concrete_painter(17, stripe=True), [((0, 3, -3), (14, 0, 0))])
    P.append(back)
    P.append(box((-20, 11, -2), (20, 14, 2), g, concrete_painter(19)))
    return P


BUILDERS = {"crate": crate_parts, "rock": rock_parts, "barrier": barrier_parts}


def build_one(name):
    return BUILDERS[name]()


def build_parts(pose=None):
    out = []
    for name, dx in (("crate", -40.0), ("rock", 0.0), ("barrier", 45.0)):
        for p in build_one(name):
            p = dict(p)
            p["tr"] = list(p["tr"]) + [((0, 0, 0), (0, 0, 0))]
            p["tr"] = [(pv, r) for pv, r in p["tr"]]
            # Shift by moving the box and every pivot.
            p["from"] = (p["from"][0] + dx, p["from"][1], p["from"][2])
            p["to"] = (p["to"][0] + dx, p["to"][1], p["to"][2])
            p["tr"] = [((pv[0] + dx, pv[1], pv[2]), r) for pv, r in p["tr"]]
            out.append(p)
    return out


# ---------------------------------------------------------------- export

def exports(parts):
    return {name: (build_one(name), (0, 0, 0)) for name in BUILDERS}


def layout(parts, s, vec, points):
    lines = []
    for name in BUILDERS:
        pts = points(build_one(name))
        lo, hi = pts.min(0), pts.max(0)
        c = (lo + hi) / 2
        half = (hi - lo) / 2
        lines.append(f"/// Bounding box of the {name}: center and half size, from its base origin.")
        lines.append(f"pub const {name.upper()}_CENTER: Vector3 = {vec((c[0], c[1], c[2]))};")
        lines.append(f"pub const {name.upper()}_HALF: Vector3 = {vec((half[0], half[1], half[2]))};")
    return "\n".join(lines) + "\n"
