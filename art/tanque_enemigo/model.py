"""Tanque enemigo: un cazador ligero, más bajo y corto que el del jugador, con acero oxidado
(OXIDE) y luces rojas para que se lea como hostil de un vistazo.

Frente = +Z. 1 unidad = 1 pixel de textura. Mismos grupos que el tanque del jugador
(hull, turret, barrel, wheel_*) para que el juego anime cada pieza igual.
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "common"))
from paint import *  # noqa: E402,F401,F403
from paint import np  # noqa: E402

TURRET_PIVOT = (0.0, 11.5, -3.0)
BARREL_PIVOT = (0.0, 14.0, 9.0)
ROAD_WHEEL_Z = (13.0, 4.5, -4.5, -13.0)
ROAD_WHEEL_Y = 4.6
IDLER_Z = 20.0
IDLER_Y = 5.0
TRACK_X = (10, 16)


def armor(seed, base=5, glows=None, panel=(8, 12), hseam=None):
    return plain_armor(seed, base=base, glows=glows, panel=panel, hseam=hseam, glow_pal=RED, pal=OXIDE)


def eye_painter(face, h, w):
    """Hunter's sight: a dark visor with a red slit, brightest in the middle."""
    idx = armor_idx(h, w, face, 71, 4, (30, 40))
    if face != "front":
        return rgba(idx, OXIDE)
    g = np.full((h, w), -1)
    r = h // 2
    g[r, 1:w - 1] = 2
    g[r, 2:w - 2] = 3
    g[r, w // 2 - 1:w // 2 + 1] = 4
    return rgba(idx, OXIDE, (g, RED))


def exhaust_painter(face, h, w):
    """Rear engine grille: dark slats with a dull red glow between them."""
    y, x = np.mgrid[0:h, 0:w]
    idx = armor_idx(h, w, face, 93, 3, (30, 40))
    if face != "back":
        return rgba(idx, OXIDE)
    g = np.where((x % 3 == 1) & (y > 0) & (y < h - 1), 1, -1)
    g = np.where((g >= 0) & (y == h // 2), 2, g)
    idx = np.where(x % 3 == 1, 0, idx)
    return rgba(idx, OXIDE, (g, RED))


# ---------------------------------------------------------------- geometry

def wheel(zc, yc, size, name):
    """Round wheel like the player's: a straight box plus one turned 45 degrees."""
    h = size / 2
    x0, x1 = TRACK_X
    pv = ((x0 + x1) / 2, yc, zc)
    a = box((x0, yc - h, zc - h), (x1, yc + h, zc + h), name, idler_painter(int(zc * 5) + size))
    b = box((x0 - 0.5, yc - h, zc - h), (x1 + 0.5, yc + h, zc + h), name, idler_painter(int(zc * 5) + size + 1, turned=True), [(pv, (45, 0, 0))])
    for p in (a, b):
        p["pivot"] = pv
    return [a, b]


def hull_side():
    P = []
    x0, x1 = TRACK_X
    xm = (x0 + x1) / 2
    tp = track_painter(5, glow_pal=RED)
    P.append(box((x0, 0, -16), (x1, 1, 16), "hull", tp))
    P.append(box((x0, 0, 16), (x1, 1, 22), "hull", tp, [((xm, 0, 16), (-40, 0, 0))]))
    P.append(box((x0, 0, -22), (x1, 1, -16), "hull", tp, [((xm, 0, -16), (40, 0, 0))]))
    P.append(box((x0, 8, -21), (x1, 9, 21), "hull", tp))
    for k, zc in enumerate(ROAD_WHEEL_Z):
        P += wheel(zc, ROAD_WHEEL_Y, 6, f"wheel_{k + 1}")
    P += wheel(IDLER_Z, IDLER_Y, 7, "wheel_0")
    P += wheel(-IDLER_Z, IDLER_Y, 7, f"wheel_{len(ROAD_WHEEL_Z) + 1}")
    # Side skirt, tilted outward at the bottom, with a row of red slits.
    slits = {"right": [(0.55, 0.68, f, f + 0.07) for f in (0.12, 0.30, 0.48, 0.66)]}
    P.append(box((x0 - 1, 7, -24), (x1 + 1, 12, 20), "hull", armor(11, glows=slits, hseam=2), [((xm, 12, 0), (0, 0, 8))]))
    return P


def hull_center():
    P = []
    deck = {"top": [(0.12, 0.17, 0.2, 0.8)]}
    P.append(box((-9, 4, -26), (9, 11.5, 22), "hull", armor(23, glows=deck, panel=(9, 13))))
    # Wedge nose: the upper glacis slopes down, the lower plate slopes up to meet it.
    glow = {"top": [(0.70, 0.80, 0.10, 0.26), (0.70, 0.80, 0.74, 0.90)]}
    P.append(box((-9, 8.5, 22), (9, 11.5, 31), "hull", armor(29, glows=glow), [((0, 11.5, 22), (22, 0, 0))]))
    P.append(box((-9, 4, 22), (9, 6, 30), "hull", armor(31, base=4), [((0, 4, 22), (-30, 0, 0))]))
    # Engine deck and grille at the back, with a red tail light on each corner.
    P.append(box((-7, 11.5, -25), (7, 12.5, -16), "hull", armor(33, base=4, panel=(3, 4))))
    P.append(box((-6, 5, -28), (6, 10, -26), "hull", exhaust_painter))
    tail = {"back": [(0.3, 0.7, 0.15, 0.85)]}
    P.append(box((-9, 9, -27), (-7, 11, -26), "hull", armor(35, base=4, glows=tail)))
    P.append(box((7, 9, -27), (9, 11, -26), "hull", armor(37, base=4, glows=tail)))
    return P


def turret_parts():
    P = []
    tg = "turret"
    side = {"right": [(0.30, 0.45, 0.30, 0.60)], "left": [(0.30, 0.45, 0.40, 0.70)]}
    P.append(box((-8, 11.5, -14), (8, 16.5, 5), tg, armor(41, glows=side, panel=(7, 10))))
    # Cheeks turned in hard so the front ends in a sharp wedge.
    cheek = box((0, 11.5, 0), (9, 16.5, 12), tg, armor(43, glows={"top": [(0.3, 0.42, 0.3, 0.8)]}), [((0, 14, 0), (0, 32, 0))])
    P.append(cheek)
    P.append(mirror(cheek))
    # Bustle with a red stripe on the back.
    P.append(box((-7, 12, -19), (7, 16, -14), tg, armor(47, base=4, glows={"back": [(0.4, 0.55, 0.1, 0.9)]})))
    # Roof sight with the red eye, and a short whip antenna.
    P.append(box((-6, 16.5, -6), (0, 19, 0), tg, eye_painter))
    P.append(box((5, 16.5, -13), (6, 27, -12), tg, antenna_painter))
    return P


def barrel_parts():
    bg = "barrel"
    return [
        box((-3.5, 12, 8), (3.5, 16, 12), bg, armor(61, base=4, panel=(30, 40))),  # mantlet
        box((-1.5, 12.5, 12), (1.5, 15.5, 16), bg, armor(63, base=3, panel=(30, 40))),  # collar
        box((-1, 13, 16), (1, 15, 40), bg, barrel_painter(65)),
        box((-1.5, 12.5, 37), (1.5, 15.5, 42), bg, barrel_painter(67)),  # muzzle brake
    ]


def build_parts(pose=None):
    pose = pose or {}
    turret_yaw = pose.get("turret_yaw", 0.0)
    barrel_pitch = pose.get("barrel_pitch", 0.0)
    side = hull_side()
    other = []
    for p in side:
        m = mirror(p)
        if p["group"].startswith("wheel_"):
            m["group"] = p["group"].replace("wheel_", "wheel_r")
            m["pivot"] = (-p["pivot"][0], p["pivot"][1], p["pivot"][2])
            p["group"] = p["group"].replace("wheel_", "wheel_l")
        other.append(m)
    turret = turret_parts()
    barrel = barrel_parts()
    for p in barrel:
        p["tr"] = p["tr"] + [(BARREL_PIVOT, (-barrel_pitch, 0, 0))]
    for p in turret + barrel:
        p["tr"] = p["tr"] + [(TURRET_PIVOT, (0, turret_yaw, 0))]
    return side + other + hull_center() + turret + barrel


# ---------------------------------------------------------------- export

def exports(parts):
    by = {}
    for p in parts:
        by.setdefault(p["group"], []).append(p)
    return {
        "hull": (by["hull"], (0, 0, 0)),
        "turret": (by["turret"], TURRET_PIVOT),
        "barrel": (by["barrel"], BARREL_PIVOT),
        "wheel_road": (by["wheel_l1"], by["wheel_l1"][0]["pivot"]),
        "wheel_end": (by["wheel_l0"], by["wheel_l0"][0]["pivot"]),
    }


def layout(parts, s, vec, points):
    by = {}
    for p in parts:
        by.setdefault(p["group"], []).append(p)
    barrel_tip = max(points(by["barrel"])[:, 2])
    last = len(ROAD_WHEEL_Z) + 1
    wheels = []
    for g, ps in sorted(by.items()):
        if not g.startswith("wheel_"):
            continue
        pv = ps[0]["pivot"]
        kind = "End" if g[-1] in ("0", str(last)) else "Road"
        wheels.append(f"    Wheel {{ pos: {vec(pv)}, radius: {(pv[1] - 1.0) * s:.4f}, kind: WheelKind::{kind} }},")
    # Hit box: hull and turret, leaving out the thin antenna.
    solid = [p for p in by["hull"] + by["turret"] if p["to"][0] - p["from"][0] > 1.5]
    body = points(solid)
    lo, hi = body.min(0), body.max(0)
    tp = np.asarray(TURRET_PIVOT, float)
    bp = np.asarray(BARREL_PIVOT, float)
    return f"""pub use crate::model_layout::{{Wheel, WheelKind}};

/// Turret pivot relative to the hull origin.
pub const TURRET_PIVOT: Vector3 = {vec(tp)};
/// Barrel pivot relative to the turret pivot.
pub const BARREL_PIVOT: Vector3 = {vec(bp - tp)};
/// From the barrel pivot to the muzzle.
pub const BARREL_LENGTH: f32 = {(barrel_tip - bp[2]) * s:.4f};
/// Hit box of the hull and turret, relative to the hull origin.
pub const BOX_CENTER: Vector3 = {vec((lo + hi) / 2)};
pub const BOX_HALF: Vector3 = {vec((hi - lo) / 2)};

pub const WHEELS: [Wheel; {len(wheels)}] = [
{chr(10).join(wheels)}
];
"""
