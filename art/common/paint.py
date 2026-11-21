"""Shared palettes and texture painters for the 3dixel models of this game.

1 unit = 1 texture pixel. Painters return RGBA images; alpha 2.0 marks glowing texels,
which the renderer and the game draw unlit.
"""
import os
import sys
import numpy as np

CORE = os.environ.get("DIXEL_CORE", os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "tools", "3dixel", "core"))
if CORE not in sys.path:
    sys.path.insert(0, CORE)
from kit import FACES, hexc, face_size, part, mirror  # noqa: E402,F401

# Gunmetal with cool shadows and slightly warm lights (dark to light).
STEEL = [hexc(c) for c in ("#07090d", "#0e1217", "#151a21", "#1c222a", "#242b34", "#2e3640", "#39424d", "#48525e", "#5c6672", "#79828c")]
CYAN = [hexc(c) for c in ("#0a3440", "#0f6878", "#17a8bd", "#3fdcef", "#a6faff")]
ORANGE = [hexc(c) for c in ("#4a1d08", "#9a420f", "#e8802a", "#ffbe70", "#fff1cc")]
RUBBER = [hexc(c) for c in ("#060607", "#0d0d0f", "#151518", "#1e1e22", "#2a2a2f", "#38383e")]
WHEEL = [hexc(c) for c in ("#14140f", "#232318", "#33331f", "#46452a", "#5a5838", "#706d48")]

FACE_LIGHT = {"top": 1, "front": 0, "left": 0, "right": -1, "back": -1, "bottom": -3}
RED = [hexc(c) for c in ("#3a0a0c", "#7a1216", "#d0231f", "#ff5a3c", "#ffc2a8")]

# ---------------------------------------------------------------- textures

def rgba(idx, pal, glow=None):
    idx = np.clip(idx, 0, len(pal) - 1)
    img = np.ones(idx.shape + (4,))
    img[..., :3] = np.array(pal)[idx]
    if glow is not None:
        gi, gpal = glow
        m = gi >= 0
        img[m, :3] = np.array(gpal)[np.clip(gi[m], 0, len(gpal) - 1)]
        img[m, 3] = 2.0
    return img


def armor_idx(h, w, face, seed, base=5, panel=(9, 13), hseam=None):
    """Armor plates: light falling from the top-left, panel seams with a lit bevel,
    one tone of variation per panel, bolts at a few panel corners and short wear marks."""
    rng = np.random.default_rng(seed)
    y, x = np.mgrid[0:h, 0:w]
    b = base + FACE_LIGHT[face]
    lit = np.round(0.6 - 1.3 * y / max(h - 1, 1) - 0.7 * x / max(w - 1, 1) + 0.5).astype(int)
    if face in ("front", "back", "left", "right"):
        lit = lit + np.round(-1.2 * y / max(h - 1, 1) + 0.4).astype(int)
    idx = b + lit

    # Vertical panel seams.
    cuts, pos = [], int(rng.integers(panel[0] // 2, panel[1]))
    while pos < w - 3:
        cuts.append(pos)
        pos += int(rng.integers(panel[0], panel[1] + 1))
    hs = hseam if hseam is not None else (h // 2 if h >= 10 else None)
    edges = [0] + cuts + [w]
    for k in range(len(edges) - 1):
        a, c = edges[k], edges[k + 1]
        shift = int(rng.choice([0, 0, -1, 1]))
        idx[:, a:c] += shift
        # A bolt near the top-left corner of some panels, on the lit side.
        if c - a >= 5 and h >= 5 and rng.random() < 0.6:
            by, bx = 1, a + 2
            idx[by, bx] += 2
            idx[by + 1, bx] -= 1
        # One short wear scratch per wide panel, along the panel.
        if c - a >= 7 and h >= 6:
            sy = int(rng.integers(2, h - 2))
            sx = int(rng.integers(a + 1, c - 4))
            idx[sy, sx:sx + 3] += 1
    for cpos in cuts:
        idx[:, cpos] -= 2
        if cpos + 1 < w:
            idx[:, cpos + 1] += 1
    if hs is not None and 2 <= hs < h - 2:
        idx[hs, :] -= 2
        idx[hs + 1, :] += 1
    return idx


def no_glow(h, w):
    return np.full((h, w), -1)


def glow_bar(g, y0, y1, x0, x1):
    """Cyan light strip: bright core, dimmer ends."""
    y0, y1, x0, x1 = int(y0), int(y1), int(x0), int(x1)
    if y1 <= y0 or x1 <= x0:
        return g
    g[y0:y1, x0:x1] = 2
    if x1 - x0 >= 3:
        g[y0:y1, x0 + 1:x1 - 1] = 3
    if x1 - x0 >= 5 and y1 - y0 >= 1:
        g[y0:y1, (x0 + x1) // 2] = 4
    return g


def box(f, t, group, painter, tr=()):
    tex = {}
    for face in FACES:
        h, w = face_size(f, t, face)
        tex[face] = painter(face, h, w)
    p = part(f, t, tex, tr)
    p["group"] = group
    return p


def plain_armor(seed, base=5, glows=None, panel=(9, 13), hseam=None, glow_pal=None):
    """Painter: armor on every face; `glows` maps face -> list of strips in face fractions."""
    def paint(face, h, w):
        idx = armor_idx(h, w, face, seed + FACES.index(face) * 7, base, panel, hseam)
        g = no_glow(h, w)
        for (fy0, fy1, fx0, fx1) in (glows or {}).get(face, []):
            glow_bar(g, round(fy0 * h), max(round(fy0 * h) + 1, round(fy1 * h)), round(fx0 * w), round(fx1 * w))
        return rgba(idx, STEEL, (g, glow_pal or CYAN) if (g >= 0).any() else None)
    return paint


def track_painter(seed):
    """Rubber track: tread links on the outside, road wheels painted on the outer side."""
    def paint(face, h, w):
        y, x = np.mgrid[0:h, 0:w]
        if face in ("top", "bottom"):
            # Tread bars across the track, two pixels apart, with a cyan pad now and then on top.
            along = y  # rows run along z
            idx = 3 + (along % 3 == 0) * 1 - (along % 3 == 2) * 1 + FACE_LIGHT[face] // 2
            idx[:, 0] -= 1
            idx[:, -1] -= 1
            g = no_glow(h, w)
            if face == "top":
                for r in range(2, h - 2, 6):
                    g[r, 2:w - 2] = 2
                    g[r, w // 2] = 3
            return rgba(idx, RUBBER, (g, CYAN) if face == "top" else None)
        if face in ("front", "back"):
            idx = 3 - (y * 2 // max(h, 1)) + (y % 2)
            return rgba(idx, RUBBER)
        # Outer and inner sides: track belt top and bottom, road wheels in between.
        idx = np.full((h, w), 2)
        idx[:2, :] = 3 + (x[:2] % 3 == 0)
        idx[-2:, :] = 2 + (x[-2:] % 3 == 0)
        widx = np.full((h, w), -1)
        cy = h / 2 - 0.5
        n = 6
        r = min((h - 2.5) / 2, (w / n) / 2 - 0.3)
        for k in range(n):
            cx = (k + 0.5) * w / n - 0.5
            d = np.hypot(y - cy, x - cx)
            rim = (d <= r) & (d > r - 1.2)
            disc = d <= r - 1.2
            hub = d <= 1.3
            lit = np.round(0.8 - 0.6 * (y - cy) / r - 0.4 * (x - cx) / r).astype(int)
            widx = np.where(disc, 2 + lit, widx)
            widx = np.where(rim, 1 + (lit > 0), widx)
            widx = np.where(hub, 4, widx)
        img = rgba(idx, RUBBER)
        m = widx >= 0
        img[m, :3] = np.array(WHEEL)[np.clip(widx[m], 0, len(WHEEL) - 1)]
        return img
    return paint


def idler_painter(seed, turned=False):
    """End wheel of the track: tread on the rim faces, a spoked wheel inside a rubber ring on the sides."""
    def paint(face, h, w):
        y, x = np.mgrid[0:h, 0:w]
        if face in ("left", "right"):
            cy, cx = (h - 1) / 2, (w - 1) / 2
            d = np.hypot(y - cy, x - cx)
            idx = np.full((h, w), 2)
            idx = np.where((x + y) % 3 == 0, 3, idx)  # tread links seen at the corners
            img = rgba(idx, RUBBER)
            r = h / 2 - 1.0
            lit = np.round(0.8 - 0.6 * (y - cy) / r - 0.4 * (x - cx) / r).astype(int)
            widx = np.full((h, w), -1)
            widx = np.where(d <= r, 2 + lit, widx)
            widx = np.where((d <= r) & (d > r - 1.1), 1 + (lit > 0), widx)
            ang = np.arctan2(y - cy, x - cx) + (np.pi / 4 if turned else 0)
            spoke = (np.abs(((ang * 3 / np.pi) % 1) - 0.5) > 0.38) & (d < r - 1.1) & (d > 1.4)
            widx = np.where(spoke, 1, widx)
            widx = np.where(d <= 1.4, 4, widx)
            m = widx >= 0
            img[m, :3] = np.array(WHEEL)[np.clip(widx[m], 0, len(WHEEL) - 1)]
            return img
        # Rim faces: tread bars across the belt.
        along = y if face in ("top", "bottom") else y
        idx = 3 + (along % 3 == 0) * 1 - (along % 3 == 2) * 1 + FACE_LIGHT[face] // 2
        idx[:, 0] -= 1
        idx[:, -1] -= 1
        return rgba(idx, RUBBER)
    return paint


def thruster_painter(face, h, w):
    y, x = np.mgrid[0:h, 0:w]
    idx = armor_idx(h, w, face, 91, 4, (20, 30))
    g = no_glow(h, w)
    if face == "back":
        cy, cx = (h - 1) / 2, (w - 1) / 2
        d = np.hypot(y - cy, x - cx)
        o = np.full((h, w), -1)
        o = np.where(d <= min(h, w) / 2 - 0.4, 2, o)
        o = np.where(d <= min(h, w) / 2 - 1.2, 3, o)
        o = np.where(d <= 0.8, 4, o)
        return rgba(idx, STEEL, (o, ORANGE))
    if face in ("left", "right", "top", "bottom"):
        idx[:, ::3] -= 1
    return rgba(idx, STEEL, None if not (g >= 0).any() else (g, CYAN))


def barrel_painter(seed):
    def paint(face, h, w):
        y, x = np.mgrid[0:h, 0:w]
        b = 4 + FACE_LIGHT[face]
        if face in ("front", "back"):
            d = np.hypot(y - (h - 1) / 2, x - (w - 1) / 2)
            idx = np.where(d < 1.0, 0, b + 1)
            return rgba(idx, STEEL)
        # Long faces: lit line along one edge, darker other edge, faint rings every few units.
        long_axis_cols = face in ("left", "right")  # z runs along columns on side faces
        idx = np.full((h, w), b)
        if long_axis_cols:
            idx[0, :] += 2
            idx[-1, :] -= 1
            ring = (x % 7 == 3)
            idx = np.where(ring, idx - 1, idx)
        else:
            idx[:, 0] += 1
            idx[:, -1] -= 1
            ring = (y % 7 == 3)
            idx = np.where(ring, idx - 1, idx)
        return rgba(idx, STEEL)
    return paint


def coil_painter(face, h, w):
    """Thermal coil around the barrel root: steel with copper-orange windings (unlit, dim)."""
    y, x = np.mgrid[0:h, 0:w]
    b = 3 + FACE_LIGHT[face]
    idx = np.full((h, w), b)
    g = no_glow(h, w)
    if face in ("left", "right"):
        g = np.where(x % 2 == 0, 1, -1)
        g[0, :] = np.where(x[0] % 2 == 0, 2, -1)
    elif face in ("top", "bottom"):
        g = np.where(y % 2 == 0, 1 + (face == "top"), -1)
    else:
        idx[1:-1, 1:-1] = 0
    return rgba(idx, STEEL, (g, ORANGE) if (g >= 0).any() else None)


def sensor_painter(face, h, w):
    y, x = np.mgrid[0:h, 0:w]
    idx = armor_idx(h, w, face, 55, 4, (30, 40))
    if face == "front":
        g = np.full((h, w), -1)
        # Two lenses: a red one and a cyan one, each a 2x2 window.
        img = rgba(idx, STEEL)
        red = [hexc("#5a0d12"), hexc("#b0161f"), hexc("#ff3b3b"), hexc("#ffb0a0")]
        cy = h // 2 - 1
        for (cx, pal) in ((1, red), (w - 3, CYAN)):
            img[cy:cy + 2, cx:cx + 2, :3] = pal[2]
            img[cy, cx, :3] = pal[3]
            img[cy:cy + 2, cx:cx + 2, 3] = 2.0
        return img
    return rgba(idx, STEEL)


def antenna_painter(face, h, w):
    y, x = np.mgrid[0:h, 0:w]
    idx = np.full((h, w), 3 + FACE_LIGHT[face])
    if face in ("front", "left"):
        idx[:, 0] += 1
    idx = idx - (y > h * 0.6)
    return rgba(idx, STEEL)
