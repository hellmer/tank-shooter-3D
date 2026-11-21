"""Torreta enemiga fija, con el mismo acero que el tanque pero luces rojas para leerse como hostil.

Frente = +Z. Grupos: base (fija), head (gira en yaw) y gun (sube y baja), para que el juego
mueva cada parte alrededor de su pivote.
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "common"))
from paint import *  # noqa: E402,F401,F403
from paint import np  # noqa: E402

HEAD_PIVOT = (0.0, 11.0, 0.0)
GUN_PIVOT = (0.0, 16.0, 9.0)
MUZZLE_X = 3.0


def pedestal_painter(seed, turned=False):
    """Heavy plinth: armor plates, a red light slit on every side, darker toward the ground."""
    base = plain_armor(seed, base=4, panel=(6, 9), hseam=3, glow_pal=RED,
                       glows={f: [(0.30, 0.42, 0.30, 0.70)] for f in ("front", "back", "left", "right")})
    if not turned:
        return base

    def paint(face, h, w):
        img = base(face, h, w)
        if face in ("front", "back", "left", "right"):
            img[-2:, :, :3] *= 0.7
        return img
    return paint


def ring_painter(face, h, w):
    y, x = np.mgrid[0:h, 0:w]
    idx = np.full((h, w), 3 + FACE_LIGHT[face])
    idx = np.where(x % 4 == 0, idx - 1, idx)
    g = np.full((h, w), -1)
    if face in ("front", "back", "left", "right"):
        g[0, :] = np.where(x[0] % 4 == 2, 3, -1)
    return rgba(idx, STEEL, (g, RED) if (g >= 0).any() else None)


def eye_painter(face, h, w):
    """Targeting eye: a dark visor with a slit of red light, brightest in the middle."""
    y, x = np.mgrid[0:h, 0:w]
    idx = armor_idx(h, w, face, 41, 4, (30, 40))
    if face == "front":
        g = np.full((h, w), -1)
        r = h // 2
        g[r, 1:w - 1] = 2
        g[r, 2:w - 2] = 3
        g[r, w // 2 - 1:w // 2 + 1] = 4
        return rgba(idx, STEEL, (g, RED))
    return rgba(idx, STEEL)


def base_parts():
    P = []
    # Plinth with four buttress feet turned 45 degrees at the corners.
    P.append(box((-9, 0, -9), (9, 8, 9), "base", pedestal_painter(3)))
    for sx in (-1, 1):
        for sz in (-1, 1):
            c = (sx * 9.0, 0.0, sz * 9.0)
            yaw = 45 if sx * sz > 0 else -45
            P.append(box((c[0] - 2, 0, c[2] - 4), (c[0] + 2, 5, c[2] + 4), "base", plain_armor(7 + sx * 3 + sz, base=4, panel=(30, 40)), [(c, (0, yaw, 0))]))
    # Bearing ring the head turns on.
    P.append(box((-7, 8, -7), (7, 11, 7), "base", ring_painter))
    return P


def head_parts():
    P = []
    hg = "head"
    side_glow = {"right": [(0.35, 0.5, 0.2, 0.6)], "left": [(0.35, 0.5, 0.4, 0.8)]}
    P.append(box((-8, 11, -11), (8, 19, 7), hg, plain_armor(21, base=5, glows=side_glow, panel=(7, 10), glow_pal=RED)))
    # Cheek plates turned toward the middle, like the player's turret but sharper.
    cheek = box((0, 11, 1), (9, 19, 13), hg, plain_armor(23, base=5, glow_pal=RED, glows={"top": [(0.4, 0.55, 0.3, 0.8)]}), [((0, 15, 1), (0, 30, 0))])
    P.append(cheek)
    P.append(mirror(cheek))
    # Roof sensor with the red eye looking forward.
    P.append(box((-4, 19, -2), (4, 22, 4), hg, eye_painter))
    # Counterweight at the back.
    P.append(box((-7, 12, -17), (7, 18, -11), hg, plain_armor(27, base=4, panel=(5, 8), glow_pal=RED, glows={"back": [(0.4, 0.55, 0.15, 0.85)]})))
    return P


def gun_parts():
    gg = "gun"
    P = [box((-5, 13, 8), (5, 19, 12), gg, plain_armor(31, base=5, panel=(30, 40)))]  # mantlet
    for x in (-MUZZLE_X, MUZZLE_X):
        P.append(box((x - 1, 15, 12), (x + 1, 17, 30), gg, barrel_painter(33)))
        P.append(box((x - 1.5, 14.5, 27), (x + 1.5, 17.5, 31), gg, barrel_painter(35)))
    return P


def build_parts(pose=None):
    pose = pose or {}
    yaw = pose.get("head_yaw", 0.0)
    pitch = pose.get("gun_pitch", 0.0)
    base, head, gun = base_parts(), head_parts(), gun_parts()
    for p in gun:
        p["tr"] = p["tr"] + [(GUN_PIVOT, (-pitch, 0, 0))]
    for p in head + gun:
        p["tr"] = p["tr"] + [(HEAD_PIVOT, (0, yaw, 0))]
    return base + head + gun


# ---------------------------------------------------------------- export

def exports(parts):
    by = {}
    for p in parts:
        by.setdefault(p["group"], []).append(p)
    return {
        "base": (by["base"], (0, 0, 0)),
        "head": (by["head"], HEAD_PIVOT),
        "gun": (by["gun"], GUN_PIVOT),
    }


def layout(parts, s, vec, points):
    gun = [p for p in parts if p["group"] == "gun"]
    tip = max(points(gun)[:, 2])
    hp = np.asarray(HEAD_PIVOT, float)
    gp = np.asarray(GUN_PIVOT, float)
    top = max(points(parts)[:, 1])
    return f"""/// Head pivot relative to the turret's base.
pub const HEAD_PIVOT: Vector3 = {vec(hp)};
/// Gun pivot relative to the head pivot.
pub const GUN_PIVOT: Vector3 = {vec(gp - hp)};
/// From the gun pivot to the muzzles.
pub const GUN_LENGTH: f32 = {(tip - gp[2]) * s:.4f};
/// Sideways offset of each of the two barrels.
pub const MUZZLE_X: f32 = {MUZZLE_X * s:.4f};
/// Height of the whole turret, for hit boxes.
pub const HEIGHT: f32 = {top * s:.4f};
"""
