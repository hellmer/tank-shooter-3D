"""Tanque del jugador, construido con 3dixel a partir de la hoja de referencia de 4 vistas.

Frente del tanque = +Z. 1 unidad = 1 pixel de textura. Cada pieza lleva "group"
(hull, turret, barrel, wheel_*) para que el exportador pueda separar las piezas que se mueven.
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "common"))
from paint import *  # noqa: E402,F401,F403
from paint import np  # noqa: E402

# Pivots shared with the game (model units; the game scales by 0.1).
TURRET_PIVOT = (0.0, 14.0, -4.0)
BARREL_PIVOT = (0.0, 18.5, 12.0)
ROAD_WHEEL_Z = (18.0, 9.0, 0.0, -9.0, -18.0)
ROAD_WHEEL_Y = 5.3
IDLER_Z = 27.0
IDLER_Y = 6.2


# ---------------------------------------------------------------- geometry

def wheel(zc, yc, size, name, spin=0.0):
    """A round wheel: a straight box plus one turned 45 degrees (an octagon from the side).
    The turned box is one unit wider so its sides never share a plane with the straight one.
    Both parts carry the wheel's axle as "pivot" and its own group name, for spinning."""
    h = size / 2
    pv = (15.5, yc, zc)
    a = box((12, yc - h, zc - h), (19, yc + h, zc + h), name, idler_painter(int(zc * 3) + size), [(pv, (spin, 0, 0))])
    b = box((11.5, yc - h, zc - h), (19.5, yc + h, zc + h), name, idler_painter(int(zc * 3) + size + 1, turned=True), [(pv, (45 + spin, 0, 0))])
    for p in (a, b):
        p["pivot"] = pv
    return [a, b]


def hull_parts():
    P = []
    # Tracks (x 12..19 on this side), with sloped ends that lift off the ground.
    tp = track_painter(3)
    # Track belt: ground run, two angled runs up to the end wheels, and the return run under the skirt.
    P.append(box((12, 0, -23), (19, 1, 23), "hull", tp))
    P.append(box((12, 0, 23), (19, 1, 29), "hull", tp, [((15.5, 0, 23), (-42, 0, 0))]))
    P.append(box((12, 0, -29), (19, 1, -23), "hull", tp, [((15.5, 0, -23), (42, 0, 0))]))
    P.append(box((12, 11, -29), (19, 12, 29), "hull", tp))
    # Every wheel is its own round part (see wheel()), so the game can spin each one.
    for k, zc in enumerate(ROAD_WHEEL_Z):
        P += wheel(zc, ROAD_WHEEL_Y, 6, f"wheel_{k + 1}")
    P += wheel(IDLER_Z, IDLER_Y, 8, "wheel_0")
    P += wheel(-IDLER_Z, IDLER_Y, 8, f"wheel_{len(ROAD_WHEEL_Z) + 1}")
    # Side skirt over the upper track, with glowing vents along the bottom edge.
    skirt = plain_armor(11, base=5, glows={"right": [(0.55, 0.72, 0.06, 0.14), (0.55, 0.72, 0.24, 0.32), (0.55, 0.72, 0.42, 0.50), (0.55, 0.72, 0.60, 0.68), (0.55, 0.72, 0.78, 0.86)]}, hseam=2)
    P.append(box((11, 9, -33), (20, 15, 31), "hull", skirt))
    # Skirt nose, angled down toward the front.
    P.append(box((11, 9, 31), (20, 15, 37), "hull", plain_armor(17, base=5), [((15.5, 15, 31), (-30, 0, 0))]))
    # Rear vent block on the skirt (the stacked cyan slats of the reference).
    vents = plain_armor(19, base=4, glows={"right": [(0.15, 0.25, 0.15, 0.85), (0.40, 0.50, 0.15, 0.85), (0.65, 0.75, 0.15, 0.85)]})
    P.append(box((20, 9, -30), (21, 15, -20), "hull", vents))
    return P


def hull_center():
    P = []
    deck_glow = {"top": [(0.10, 0.16, 0.25, 0.75), (0.26, 0.32, 0.25, 0.75)]}
    P.append(box((-11, 5, -34), (11, 14, 30), "hull", plain_armor(23, base=5, glows=deck_glow, panel=(10, 14))))
    # Glacis plate sloping down to the nose.
    glacis_glow = {"top": [(0.70, 0.80, 0.12, 0.30), (0.70, 0.80, 0.70, 0.88)]}
    P.append(box((-11, 11, 30), (11, 14, 42), "hull", plain_armor(29, base=5, glows=glacis_glow), [((0, 14, 30), (28, 0, 0))]))
    # Lower front plate under the glacis.
    P.append(box((-11, 4, 30), (11, 9, 38), "hull", plain_armor(31, base=4), [((0, 9, 30), (-20, 0, 0))]))
    # Rear plate with the two thrusters.
    P.append(box((-6, 7, -38), (-2, 11, -34), "hull", thruster_painter))
    P.append(box((2, 7, -38), (6, 11, -34), "hull", thruster_painter))
    return P


def turret_parts():
    P = []
    tg = "turret"
    side_glow = {"right": [(0.30, 0.42, 0.25, 0.55)], "left": [(0.30, 0.42, 0.45, 0.75)]}
    P.append(box((-10, 14, -18), (10, 21, 8), tg, plain_armor(41, base=5, glows=side_glow, panel=(8, 11))))
    # Cheek plates turned toward the middle so the front comes to a wedge.
    cheek_glow = {"top": [(0.35, 0.5, 0.25, 0.75)], "right": [(0.35, 0.5, 0.3, 0.7)]}
    cheek = box((0, 14, 2), (11, 21, 16), tg, plain_armor(43, base=5, glows=cheek_glow), [((0, 17.5, 2), (0, 22, 0))])
    P.append(cheek)
    P.append(mirror(cheek))
    # Rear bustle (storage box).
    P.append(box((-9, 15, -25), (9, 20, -18), tg, plain_armor(47, base=5, glows={"back": [(0.35, 0.5, 0.12, 0.32), (0.35, 0.5, 0.68, 0.88)]})))
    # Commander hatch and sensor mast with its pod.
    P.append(box((-7, 21, -11), (-1, 22, -5), tg, plain_armor(53, base=5, panel=(30, 40))))
    P.append(box((5, 21, -9), (7, 24, -7), tg, plain_armor(57, base=4, panel=(30, 40))))
    P.append(box((2, 24, -11), (10, 28, -5), tg, sensor_painter))
    # Antenna at the back left.
    P.append(box((-9, 21, -16), (-8, 34, -15), tg, antenna_painter))
    for p in P:
        p["tr"] = p["tr"]  # turret parts are posed later as a group
    return P


def barrel_parts():
    bg = "barrel"
    P = []
    P.append(box((-4, 15, 12), (4, 22, 17), bg, plain_armor(61, base=5, panel=(30, 40))))  # mantlet
    P.append(box((-2.5, 16, 17), (2.5, 21, 25), bg, coil_painter))
    P.append(box((-1.5, 17, 25), (1.5, 20, 58), bg, barrel_painter(63)))
    P.append(box((-2, 16.5, 54), (2, 20.5, 60), bg, barrel_painter(65)))  # muzzle brake
    return P


def build_parts(pose=None):
    pose = pose or {}
    turret_yaw = pose.get("turret_yaw", 0.0)
    barrel_pitch = pose.get("barrel_pitch", 0.0)
    side = hull_parts()
    spin = pose.get("wheel_spin", 0.0)
    if spin:
        for p in side:
            if p["group"].startswith("wheel_"):
                p["tr"] = [(p["pivot"], (spin, 0, 0))] + p["tr"]
    other = []
    for p in side:
        m = mirror(p)
        if p["group"].startswith("wheel_"):
            m["group"] = p["group"].replace("wheel_", "wheel_r")
            m["pivot"] = (-p["pivot"][0], p["pivot"][1], p["pivot"][2])
            p["group"] = p["group"].replace("wheel_", "wheel_l")
        other.append(m)
    parts = side + other + hull_center()
    turret = turret_parts()
    barrel = barrel_parts()
    for p in barrel:
        p["tr"] = p["tr"] + [(BARREL_PIVOT, (-barrel_pitch, 0, 0))]
    for p in turret + barrel:
        p["tr"] = p["tr"] + [(TURRET_PIVOT, (0, turret_yaw, 0))]
    return parts + turret + barrel


# ---------------------------------------------------------------- export

def exports(parts):
    """What tools/export_glb.py writes: file name -> (parts, origin of that file)."""
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
    """Rust constants the game needs so it matches the model."""
    by = {}
    for p in parts:
        by.setdefault(p["group"], []).append(p)
    barrel_tip = max(points(by["barrel"])[:, 2])
    wheels = []
    for g, ps in sorted(by.items()):
        if not g.startswith("wheel_"):
            continue
        pv = ps[0]["pivot"]
        kind = "End" if g in ("wheel_l0", "wheel_r0", "wheel_l6", "wheel_r6") else "Road"
        # Rolling radius: from the axle down to the ground the track runs on.
        wheels.append(f"    Wheel {{ pos: {vec(pv)}, radius: {(pv[1] - 1.0) * s:.4f}, kind: WheelKind::{kind} }},")
    tp = np.asarray(TURRET_PIVOT, float)
    bp = np.asarray(BARREL_PIVOT, float)
    return f"""/// Model units to game units.
pub const MODEL_SCALE: f32 = {s};
/// Turret pivot relative to the hull origin.
pub const TURRET_PIVOT: Vector3 = {vec(tp)};
/// Barrel pivot relative to the turret pivot.
pub const BARREL_PIVOT: Vector3 = {vec(bp - tp)};
/// From the barrel pivot to the muzzle.
pub const BARREL_LENGTH: f32 = {(barrel_tip - bp[2]) * s:.4f};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WheelKind {{
    Road,
    End,
}}

#[derive(Clone, Copy, Debug)]
pub struct Wheel {{
    pub pos: Vector3,
    pub radius: f32,
    pub kind: WheelKind,
}}

pub const WHEELS: [Wheel; {len(wheels)}] = [
{chr(10).join(wheels)}
];
"""
