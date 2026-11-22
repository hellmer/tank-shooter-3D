"""Exports a 3dixel model to GLB files the game can load with raylib.

Usage:  python3 tools/export_glb.py art/tanque/model.py assets/models/tank --layout src/model_layout.rs

The model file says what to write: `exports(parts)` maps each .glb name to its parts and
the origin (pivot) of that file, so the game can turn each piece around its own pivot, and
`layout(parts, scale, vec, points)` returns the Rust constants (pivots, lengths) for the game. Every face texture is
packed into one atlas per file. Texels the model marks as glowing (alpha > 1.5) also go into
an emissive texture, so the game can draw them unlit. A Rust file with the pivots and wheel
positions is written too, so the game and the model never disagree.
"""
import argparse
import json
import os
import struct
import sys
from io import BytesIO

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "3dixel", "core"))
import render  # noqa: E402

FACES = ("front", "back", "right", "left", "top", "bottom")


def pack(images, pad=1):
    """Shelf packing. Returns atlas size and the top-left corner of each image."""
    order = sorted(range(len(images)), key=lambda i: -images[i].shape[0])
    width = 256
    while True:
        x = y = shelf = 0
        spots = [None] * len(images)
        ok = True
        for i in order:
            h, w = images[i].shape[:2]
            if w + 2 * pad > width:
                ok = False
                break
            if x + w + 2 * pad > width:
                x, y, shelf = 0, y + shelf, 0
            spots[i] = (x + pad, y + pad)
            x += w + 2 * pad
            shelf = max(shelf, h + 2 * pad)
        height = y + shelf
        if ok and height <= width:
            return (width, height), spots
        width *= 2


def group_mesh(parts, origin):
    """Quads for every textured face, in the group's frame (origin moved to its pivot)."""
    faces = []
    origin = np.asarray(origin, float)
    for p in parts:
        geo = render.box_faces(p["from"], p["to"])
        center = (np.asarray(p["from"], float) + np.asarray(p["to"], float)) / 2
        for face in FACES:
            img = p["tex"].get(face)
            if img is None:
                continue
            o, u, v = (np.asarray(a, float) for a in geo[face])
            corners = [o, o + u, o + u + v, o + v]
            outward = (o + (u + v) / 2) - center
            world = [render.apply_transforms(c, p["tr"]) - origin for c in corners]
            c_world = render.apply_transforms(center, p["tr"]) - origin
            fc_world = render.apply_transforms(center + outward, p["tr"]) - origin
            n = fc_world - c_world
            n = n / np.linalg.norm(n)
            if img.shape[-1] == 3:
                img = np.concatenate([img, np.ones(img.shape[:2] + (1,))], -1)
            faces.append((world, n, img))
    return faces


def build_glb(faces, name):
    images = [f[2] for f in faces]
    (aw, ah), spots = pack(images)
    base = np.zeros((ah, aw, 4), np.uint8)
    emis = np.zeros((ah, aw, 3), np.uint8)
    pos, nor, uv, idx = [], [], [], []
    for (world, n, img), (sx, sy) in zip(faces, spots):
        h, w = img.shape[:2]
        rgb = (np.clip(img[..., :3], 0, 1) * 255).round().astype(np.uint8)
        glow = img[..., 3] > 1.5
        # Copy with a 1-texel border of the edge colors so sampling never bleeds.
        padded = np.pad(rgb, ((1, 1), (1, 1), (0, 0)), mode="edge")
        base[sy - 1:sy + h + 1, sx - 1:sx + w + 1, :3] = padded
        base[sy - 1:sy + h + 1, sx - 1:sx + w + 1, 3] = 255
        e = np.where(glow[..., None], rgb, 0).astype(np.uint8)
        emis[sy - 1:sy + h + 1, sx - 1:sx + w + 1] = np.pad(e, ((1, 1), (1, 1), (0, 0)), mode="edge")
        uvs = [(sx, sy), (sx + w, sy), (sx + w, sy + h), (sx, sy + h)]
        k = len(pos)
        for c, (tu, tv) in zip(world, uvs):
            pos.append(c)
            nor.append(n)
            uv.append((tu / aw, tv / ah))
        # Wind both triangles counter-clockwise seen from outside.
        tri_n = np.cross(world[1] - world[0], world[2] - world[0])
        if np.dot(tri_n, n) >= 0:
            idx += [k, k + 1, k + 2, k, k + 2, k + 3]
        else:
            idx += [k, k + 2, k + 1, k, k + 3, k + 2]

    pos = np.array(pos, np.float32)
    nor = np.array(nor, np.float32)
    uv = np.array(uv, np.float32)
    idx = np.array(idx, np.uint32)
    # raylib keeps indices as 16-bit, so write them that way when they fit.
    idx_type = 5123 if len(pos) < 65536 else 5125
    if idx_type == 5123:
        idx = idx.astype(np.uint16)

    def png(arr, mode):
        b = BytesIO()
        Image.fromarray(arr, mode).save(b, "PNG")
        return b.getvalue()

    blobs = [pos.tobytes(), nor.tobytes(), uv.tobytes(), idx.tobytes(), png(base, "RGBA"), png(emis, "RGB")]
    views, offset, binary = [], 0, b""
    for i, blob in enumerate(blobs):
        while len(binary) % 4:
            binary += b"\0"
        offset = len(binary)
        view = {"buffer": 0, "byteOffset": offset, "byteLength": len(blob)}
        if i < 3:
            view["target"] = 34962
        elif i == 3:
            view["target"] = 34963
        views.append(view)
        binary += blob
    while len(binary) % 4:
        binary += b"\0"

    gltf = {
        "asset": {"version": "2.0", "generator": "tank-shooter-3d export_glb.py"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0, "name": name}],
        "meshes": [{"name": name, "primitives": [{
            "attributes": {"POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2},
            "indices": 3, "material": 0}]}],
        "materials": [{
            "name": name,
            "pbrMetallicRoughness": {"baseColorTexture": {"index": 0}, "metallicFactor": 0.0, "roughnessFactor": 1.0},
            "emissiveTexture": {"index": 1},
            "emissiveFactor": [1.0, 1.0, 1.0],
        }],
        "samplers": [{"magFilter": 9728, "minFilter": 9728, "wrapS": 33071, "wrapT": 33071}],
        "images": [{"bufferView": 4, "mimeType": "image/png"}, {"bufferView": 5, "mimeType": "image/png"}],
        "textures": [{"sampler": 0, "source": 0}, {"sampler": 0, "source": 1}],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": len(pos), "type": "VEC3",
             "min": pos.min(0).tolist(), "max": pos.max(0).tolist()},
            {"bufferView": 1, "componentType": 5126, "count": len(nor), "type": "VEC3"},
            {"bufferView": 2, "componentType": 5126, "count": len(uv), "type": "VEC2"},
            {"bufferView": 3, "componentType": idx_type, "count": len(idx), "type": "SCALAR"},
        ],
        "bufferViews": views,
        "buffers": [{"byteLength": len(binary)}],
    }
    js = json.dumps(gltf, separators=(",", ":")).encode()
    while len(js) % 4:
        js += b" "
    out = struct.pack("<III", 0x46546C67, 2, 12 + 8 + len(js) + 8 + len(binary))
    out += struct.pack("<II", len(js), 0x4E4F534A) + js
    out += struct.pack("<II", len(binary), 0x004E4942) + binary
    return out, (aw, ah), len(idx) // 3


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("model")
    ap.add_argument("out")
    ap.add_argument("--layout")
    ap.add_argument("--scale", type=float, default=0.1)
    a = ap.parse_args()

    mod = render.load_model(a.model)
    parts = mod.build_parts()
    os.makedirs(a.out, exist_ok=True)
    for name, (ps, origin) in mod.exports(parts).items():
        data, size, tris = build_glb(group_mesh(ps, origin), name)
        with open(os.path.join(a.out, f"{name}.glb"), "wb") as f:
            f.write(data)
        print(f"{name}.glb: {len(ps)} cubes, {tris} triangles, atlas {size[0]}x{size[1]}")

    if a.layout:
        s = a.scale

        def vec(v):
            return f"Vector3 {{ x: {v[0] * s:.4f}, y: {v[1] * s:.4f}, z: {v[2] * s:.4f} }}"

        src = os.path.relpath(os.path.abspath(a.model), os.path.join(HERE, ".."))
        text = f"// Generated by tools/export_glb.py from {src}. Do not edit by hand.\n"
        text += "use raylib::prelude::Vector3;\n\n" + mod.layout(parts, s, vec, render.points)
        with open(a.layout, "w") as f:
            f.write(text)
        print(f"wrote {a.layout}")


if __name__ == "__main__":
    main()
