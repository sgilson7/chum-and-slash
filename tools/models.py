#!/usr/bin/env python3
"""Writes data/models.json: the hero and the dog, built from primitives.

Whole millimetres, degrees and 0-255 colours, so the core parses no float
(D2). Forward is -z; y is up; a positive x angle swings a hanging limb
forward. Run: python3 tools/models.py
"""

import json
from pathlib import Path

COLOURS = {
    # the hero (CS-20)
    "black": [24, 24, 28],
    "coat_green": [56, 112, 60],
    "coat_lining": [36, 72, 40],
    "cord_white": [236, 230, 214],
    "cord_brown": [124, 86, 54],
    "skin": [206, 158, 120],
    "hair": [42, 30, 24],
    "shoe": [44, 42, 46],
    "tool_grey": [74, 78, 84],
    "metal": [176, 184, 192],
    "glow_cyan": [130, 255, 228],
    # the fauna: bright, long-limbed, plant-like
    "dog_magenta": [214, 64, 152],
    "dog_belly": [250, 168, 96],
    "leaf_lime": [152, 226, 64],
    "leaf_teal": [40, 190, 170],
    "eye_glow": [255, 238, 120],
    "wound": [120, 20, 60],
}


def J(name, parent, at):
    return {"name": name, "parent": parent, "at": at}


def P(joint, mesh, at, size, colour, role="", style="", turn=(0, 0, 0)):
    p = {"joint": joint, "mesh": mesh, "at": at, "size": size, "colour": colour, "turn": list(turn)}
    if role:
        p["role"] = role
    if style:
        p["style"] = style
    return p


def hero():
    joints = [
        J("hips", None, [0, 960, 0]),
        J("chest", "hips", [0, 120, 0]),
        J("head", "chest", [0, 430, 0]),
        J("r_shoulder", "chest", [215, 340, 0]),
        J("r_elbow", "r_shoulder", [0, -300, 0]),
        J("tool", "r_elbow", [0, -290, 0]),
        J("l_shoulder", "chest", [-215, 340, 0]),
        J("l_elbow", "l_shoulder", [0, -300, 0]),
        J("r_hip", "hips", [100, -40, 0]),
        J("r_knee", "r_hip", [0, -440, 0]),
        J("l_hip", "hips", [-100, -40, 0]),
        J("l_knee", "l_hip", [0, -440, 0]),
    ]
    parts = [
        # torso: a black shirt under an open green coat
        P("chest", "cube", [0, 190, 0], [330, 400, 200], "black", "shirt"),
        P("hips", "cube", [0, 40, 0], [300, 160, 190], "black", "shirt"),
        P("chest", "cube", [0, 150, 115], [390, 560, 40], "coat_green", "coat"),
        P("chest", "cube", [-125, 150, -112], [140, 560, 36], "coat_green", "coat", turn=(0, -8, 0)),
        P("chest", "cube", [125, 150, -112], [140, 560, 36], "coat_green", "coat", turn=(0, 8, 0)),
        P("chest", "cube", [-205, 170, 0], [40, 520, 220], "coat_green", "coat"),
        P("chest", "cube", [205, 170, 0], [40, 520, 220], "coat_green", "coat"),
        P("hips", "cube", [0, -60, 112], [380, 300, 36], "coat_green", "coat", turn=(-6, 0, 0)),
        P("chest", "cube", [0, 395, 0], [300, 70, 230], "coat_lining", "collar"),
        # head
        P("head", "cyl", [0, -40, 0], [110, 90, 110], "skin"),
        P("head", "sphere", [0, 90, 0], [210, 240, 220], "skin"),
        P("head", "blob", [0, 170, 20], [230, 110, 240], "hair"),
        # arms: green sleeves, bare hands
        P("r_shoulder", "cyl", [0, -150, 0], [120, 310, 120], "coat_green", "coat"),
        P("r_elbow", "cyl", [0, -130, 0], [100, 270, 100], "coat_green", "coat"),
        P("r_elbow", "sphere", [0, -285, 0], [95, 95, 95], "skin"),
        P("l_shoulder", "cyl", [0, -150, 0], [120, 310, 120], "coat_green", "coat"),
        P("l_elbow", "cyl", [0, -130, 0], [100, 270, 100], "coat_green", "coat"),
        P("l_elbow", "sphere", [0, -285, 0], [95, 95, 95], "skin"),
        # legs: corduroy trousers, white wales with brown between, cuffed
        P("r_hip", "cyl", [0, -220, 0], [155, 450, 155], "cord_white", "trousers", "stripes"),
        P("r_knee", "cyl", [0, -200, 0], [138, 400, 138], "cord_white", "trousers", "stripes"),
        P("r_knee", "cyl", [0, -385, 0], [160, 70, 160], "cord_brown", "cuff"),
        P("r_knee", "cube", [0, -450, -45], [115, 70, 250], "shoe"),
        P("l_hip", "cyl", [0, -220, 0], [155, 450, 155], "cord_white", "trousers", "stripes"),
        P("l_knee", "cyl", [0, -200, 0], [138, 400, 138], "cord_white", "trousers", "stripes"),
        P("l_knee", "cyl", [0, -385, 0], [160, 70, 160], "cord_brown", "cuff"),
        P("l_knee", "cube", [0, -450, -45], [115, 70, 250], "shoe"),
        # the utility tool: a haft held forward, a blade on one side, a pick
        # on the other, and a glowing edge (cues 33-37)
        P("tool", "cyl", [0, -200, 0], [38, 520, 38], "tool_grey", "haft"),
        P("tool", "wedge", [0, -430, -95], [36, 200, 170], "metal", "blade", turn=(0, 0, 0)),
        P("tool", "cube", [0, -430, -182], [16, 200, 14], "glow_cyan", "tool_edge", "glow"),
        P("tool", "cone", [0, -430, 75], [40, 150, 40], "metal", "pick", turn=(90, 0, 0)),
    ]
    return {"joints": joints, "parts": parts}


def dog():
    joints = [
        J("hips", None, [0, 640, 160]),
        J("chest", "hips", [0, 40, -360]),
        J("neck", "chest", [0, 110, -170]),
        J("head", "neck", [0, 270, -110]),
        J("jaw", "head", [0, -60, -60]),
        J("fl_hip", "chest", [130, -60, -40]),
        J("fl_knee", "fl_hip", [0, -340, 0]),
        J("fr_hip", "chest", [-130, -60, -40]),
        J("fr_knee", "fr_hip", [0, -340, 0]),
        J("bl_hip", "hips", [130, -40, 40]),
        J("bl_knee", "bl_hip", [0, -340, 0]),
        J("br_hip", "hips", [-130, -40, 40]),
        J("br_knee", "br_hip", [0, -340, 0]),
        J("tail", "hips", [0, 60, 200]),
    ]
    parts = [
        P("hips", "facet", [0, 0, 0], [300, 260, 420], "dog_magenta", "body"),
        P("chest", "facet", [0, 10, 0], [330, 310, 380], "dog_magenta", "body"),
        P("chest", "facet", [0, -90, 20], [240, 140, 300], "dog_belly", "body"),
        P("neck", "facetcyl", [0, 120, -50], [95, 300, 95], "dog_magenta", "body", turn=(-25, 0, 0)),
        P("head", "facet", [0, 0, -80], [190, 170, 290], "dog_magenta", "head"),
        P("jaw", "wedge", [0, -20, -110], [150, 60, 230], "dog_belly", "jaw"),
        P("head", "sphere", [60, 40, -170], [45, 45, 45], "eye_glow", "eye", "glow"),
        P("head", "sphere", [-60, 40, -170], [45, 45, 45], "eye_glow", "eye", "glow"),
        # fronds along the back and on the head: the plant-like bent
        P("head", "facetcone", [70, 120, 20], [60, 220, 60], "leaf_lime", "frond", turn=(-30, 0, -25)),
        P("head", "facetcone", [-70, 120, 20], [60, 220, 60], "leaf_lime", "frond", turn=(-30, 0, 25)),
        P("chest", "facetcone", [0, 190, 40], [90, 260, 90], "leaf_teal", "frond", turn=(-35, 0, 0)),
        P("hips", "facetcone", [0, 170, -60], [80, 230, 80], "leaf_teal", "frond", turn=(-35, 0, 0)),
        P("hips", "facetcone", [0, 150, 120], [70, 200, 70], "leaf_lime", "frond", turn=(-40, 0, 0)),
        P("tail", "facetcone", [0, 40, 160], [70, 380, 70], "leaf_lime", "tail", turn=(-70, 0, 0)),
    ]
    for side in ("fl", "fr", "bl", "br"):
        parts += [
            P(f"{side}_hip", "facetcyl", [0, -170, 0], [62, 340, 62], "dog_magenta", "leg"),
            P(f"{side}_knee", "facetcone", [0, -150, 0], [56, 300, 56], "leaf_teal", "leg", turn=(180, 0, 0)),
            P(f"{side}_knee", "facet", [0, -300, -20], [80, 50, 110], "leaf_lime", "foot"),
        ]
    return {"joints": joints, "parts": parts}


def main():
    out = {"colours": COLOURS, "models": {"hero": hero(), "dog": dog()}}
    path = Path(__file__).resolve().parents[1] / "data" / "models.json"
    path.write_text(json.dumps(out, indent=1) + "\n")
    print(f"wrote {path}")


if __name__ == "__main__":
    main()
