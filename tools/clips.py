#!/usr/bin/env python3
"""Writes data/clips.json: the hero's and the dog's keyed clips.

Each key is [tick, [x, y, z] degrees, ease into this key]. Positive x swings
a hanging limb forward, positive y turns left (counter-clockwise from above),
z rolls. Root curves are [tick, forward mm, up mm]. Timings start from the
plan's numbers (PLAN.md "Starting numbers": light 5/3/6/10, heavy 14/4/10/16)
and change in the feel rounds. Strike segments are linear so no tick turns
a joint more than 45 degrees (clip.rs MAX_STEP_DEG). Run: python3 tools/clips.py
"""

import json
from pathlib import Path


def K(t, x=0, y=0, z=0, e="inout"):
    return [t, [x, y, z], e]


# The hero's guard: tool held forward and low in the right hand.
GUARD = {
    "r_shoulder": (25, 0, 8),
    "r_elbow": (45, 0, 0),
    "l_shoulder": (15, 0, -8),
    "l_elbow": (30, 0, 0),
    "chest": (6, 0, 0),
    "r_hip": (8, 0, 0),
    "r_knee": (-12, 0, 0),
    "l_hip": (8, 0, 0),
    "l_knee": (-12, 0, 0),
}


def guard_at(t, e="inout", but=()):
    return {j: K(t, *a, e=e) for j, a in GUARD.items() if j not in but}


def clip(length, keys, loop=False, root=None, events=None):
    c = {"len": length, "keys": keys}
    if loop:
        c["loop"] = True
    if root:
        c["root"] = root
    if events:
        c["events"] = events
    return c


def merge(*frames):
    """frames: lists of {joint: key}; returns {joint: [keys]}."""
    out = {}
    for f in frames:
        for j, k in f.items():
            out.setdefault(j, []).append(k)
    return out


def g(t, e="inout", **over):
    """The guard at tick t, with some joints overridden: name=(x, y, z)."""
    f = guard_at(t, e, but=over.keys())
    for j, a in over.items():
        f[j] = K(t, *a, e=e)
    return f


def attack(phases, hit, cancel, length, poses, forward_mm, strike_tick, up_mm=0):
    """poses: list of (tick, ease, overrides)."""
    frames = [g(t, e, **o) for t, e, o in poses]
    return clip(
        length,
        merge(*frames),
        root=[[0, 0, 0], [phases[0], forward_mm // 5, 0], [strike_tick, forward_mm, up_mm], [length, forward_mm, up_mm]],
        events={"hit": hit, "cancel": cancel, "phases": phases},
    )


def hero():
    c = {}
    c["idle"] = clip(90, merge(g(0), g(45, chest=(9, 0, 0), r_elbow=(50, 0, 0)), g(90)), loop=True)

    def run(n, amp, lean):
        h = n // 2
        q = n // 4
        return clip(
            n,
            merge(
                g(0, r_hip=(amp, 0, 0), l_hip=(-amp, 0, 0), r_knee=(-20, 0, 0), l_knee=(-70, 0, 0),
                  l_shoulder=(amp, 0, -8), r_shoulder=(25 - amp // 2, 0, 8), chest=(lean, 0, 0)),
                g(q, r_hip=(0, 0, 0), l_hip=(0, 0, 0), r_knee=(-60, 0, 0), l_knee=(-30, 0, 0),
                  l_shoulder=(15, 0, -8), r_shoulder=(25, 0, 8), chest=(lean, 0, 0)),
                g(h, r_hip=(-amp, 0, 0), l_hip=(amp, 0, 0), r_knee=(-70, 0, 0), l_knee=(-20, 0, 0),
                  l_shoulder=(-amp, 0, -8), r_shoulder=(25 + amp // 2, 0, 8), chest=(lean, 0, 0)),
                g(h + q, r_hip=(0, 0, 0), l_hip=(0, 0, 0), r_knee=(-30, 0, 0), l_knee=(-60, 0, 0),
                  l_shoulder=(15, 0, -8), r_shoulder=(25, 0, 8), chest=(lean, 0, 0)),
                g(n, r_hip=(amp, 0, 0), l_hip=(-amp, 0, 0), r_knee=(-20, 0, 0), l_knee=(-70, 0, 0),
                  l_shoulder=(amp, 0, -8), r_shoulder=(25 - amp // 2, 0, 8), chest=(lean, 0, 0)),
            ),
            loop=True,
        )

    c["run"] = run(36, 35, 12)
    c["sprint"] = run(28, 50, 22)
    c["jump"] = clip(16, merge(g(0), g(6, r_hip=(50, 0, 0), l_hip=(30, 0, 0), r_knee=(-90, 0, 0), l_knee=(-70, 0, 0)),
                                g(16, r_hip=(40, 0, 0), l_hip=(20, 0, 0), r_knee=(-60, 0, 0), l_knee=(-50, 0, 0))))
    c["fall"] = clip(30, merge(g(0, r_hip=(40, 0, 0), l_hip=(20, 0, 0), r_knee=(-60, 0, 0), l_knee=(-50, 0, 0)),
                               g(15, r_hip=(30, 0, 0), l_hip=(30, 0, 0), r_knee=(-50, 0, 0), l_knee=(-50, 0, 0)),
                               g(30, r_hip=(40, 0, 0), l_hip=(20, 0, 0), r_knee=(-60, 0, 0), l_knee=(-50, 0, 0))), loop=True)

    # Light string: 5 anticipation, 3 strike, 6 follow-through, 10 recovery.
    L = dict(phases=[5, 8, 14], hit=[6, 8], cancel=12, length=24)
    c["light_1"] = attack(**L, forward_mm=500, strike_tick=8, poses=[
        (0, "inout", {}),
        (5, "out", dict(r_shoulder=(80, 60, 10), r_elbow=(60, 0, 0), chest=(4, 25, 0))),
        (8, "linear", dict(r_shoulder=(85, -60, 10), r_elbow=(10, 0, 0), chest=(8, -30, 0))),
        (14, "out", dict(r_shoulder=(70, -85, 10), r_elbow=(25, 0, 0), chest=(8, -35, 0))),
        (24, "inout", {}),
    ])
    c["light_2"] = attack(**L, forward_mm=500, strike_tick=8, poses=[
        (0, "inout", {}),
        (5, "out", dict(r_shoulder=(75, -75, 10), r_elbow=(70, 0, 0), chest=(4, -30, 0))),
        (8, "linear", dict(r_shoulder=(85, 40, 10), r_elbow=(15, 0, 0), chest=(8, 25, 0))),
        (14, "out", dict(r_shoulder=(70, 60, 10), r_elbow=(25, 0, 0), chest=(8, 30, 0))),
        (24, "inout", {}),
    ])
    c["light_3"] = attack(**L, forward_mm=550, strike_tick=8, poses=[
        (0, "inout", {}),
        (5, "out", dict(r_shoulder=(140, 30, 20), r_elbow=(50, 0, 0), chest=(-6, 15, 0))),
        (8, "linear", dict(r_shoulder=(30, -30, 10), r_elbow=(10, 0, 0), chest=(20, -15, 0))),
        (14, "out", dict(r_shoulder=(20, -40, 10), r_elbow=(25, 0, 0), chest=(18, -15, 0))),
        (24, "inout", {}),
    ])
    c["light_4"] = attack(phases=[6, 9, 16], hit=[7, 9], cancel=14, length=28, forward_mm=750, strike_tick=9, poses=[
        (0, "inout", {}),
        (6, "out", dict(r_shoulder=(40, 10, 10), r_elbow=(110, 0, 0), chest=(0, 30, 0), l_shoulder=(40, 0, -10))),
        (9, "linear", dict(r_shoulder=(90, 0, 5), r_elbow=(0, 0, 0), chest=(14, -15, 0), l_shoulder=(-20, 0, -10))),
        (16, "out", dict(r_shoulder=(88, 0, 5), r_elbow=(5, 0, 0), chest=(14, -15, 0), l_shoulder=(-20, 0, -10))),
        (28, "inout", {}),
    ])

    # Heavy string: 14 anticipation, 4 strike, 10 follow-through, 16 recovery.
    H = dict(phases=[14, 18, 28], hit=[15, 18], cancel=30, length=44)
    c["heavy_1"] = attack(**H, forward_mm=1000, strike_tick=18, poses=[
        (0, "inout", {}),
        (14, "out", dict(r_shoulder=(165, 0, 5), r_elbow=(40, 0, 0), l_shoulder=(160, 0, -5), l_elbow=(40, 0, 0), chest=(-14, 0, 0))),
        (18, "linear", dict(r_shoulder=(40, 0, 5), r_elbow=(5, 0, 0), l_shoulder=(40, 0, -5), l_elbow=(10, 0, 0), chest=(26, 0, 0))),
        (28, "out", dict(r_shoulder=(25, 0, 5), r_elbow=(10, 0, 0), l_shoulder=(25, 0, -5), l_elbow=(15, 0, 0), chest=(28, 0, 0),
                         r_knee=(-35, 0, 0), l_knee=(-35, 0, 0))),
        (44, "inout", {}),
    ])
    c["heavy_2"] = attack(**H, forward_mm=900, strike_tick=18, poses=[
        (0, "inout", {}),
        (14, "out", dict(r_shoulder=(80, 90, 10), r_elbow=(40, 0, 0), l_shoulder=(70, 60, -5), chest=(0, 55, 0))),
        (18, "linear", dict(r_shoulder=(85, -60, 10), r_elbow=(5, 0, 0), l_shoulder=(70, -40, -5), chest=(6, -50, 0))),
        (28, "out", dict(r_shoulder=(70, -90, 10), r_elbow=(15, 0, 0), l_shoulder=(50, -60, -5), chest=(6, -60, 0))),
        (44, "inout", {}),
    ])
    c["heavy_3"] = attack(phases=[16, 20, 32], hit=[17, 20], cancel=34, length=50, forward_mm=1200, strike_tick=20, poses=[
        (0, "inout", {}),
        (16, "out", dict(r_shoulder=(170, 20, 10), r_elbow=(70, 0, 0), l_shoulder=(165, 20, -10), l_elbow=(60, 0, 0),
                         chest=(-18, 20, 0), r_knee=(-30, 0, 0), l_knee=(-30, 0, 0))),
        (20, "linear", dict(r_shoulder=(30, -10, 5), r_elbow=(0, 0, 0), l_shoulder=(30, -10, -5), l_elbow=(0, 0, 0),
                            chest=(32, -10, 0), r_knee=(-45, 0, 0), l_knee=(-45, 0, 0))),
        (32, "out", dict(r_shoulder=(20, -10, 5), r_elbow=(5, 0, 0), l_shoulder=(20, -10, -5), l_elbow=(5, 0, 0),
                         chest=(34, -10, 0), r_knee=(-45, 0, 0), l_knee=(-45, 0, 0))),
        (50, "inout", {}),
    ])

    # Launcher: low, then an upward sweep that lifts a stunned enemy.
    c["launcher"] = attack(phases=[10, 14, 22], hit=[11, 14], cancel=24, length=40, forward_mm=300, strike_tick=14, poses=[
        (0, "inout", {}),
        (10, "out", dict(r_shoulder=(-30, 10, 10), r_elbow=(20, 0, 0), chest=(22, 10, 0), r_knee=(-40, 0, 0), l_knee=(-40, 0, 0),
                         r_hip=(30, 0, 0), l_hip=(30, 0, 0))),
        (14, "linear", dict(r_shoulder=(140, 0, 10), r_elbow=(5, 0, 0), chest=(-10, 0, 0))),
        (22, "out", dict(r_shoulder=(160, 0, 10), r_elbow=(10, 0, 0), chest=(-14, 0, 0))),
        (40, "inout", {}),
    ])

    # In the air: shorter strings; they cost stamina (CS-3).
    c["air_light"] = attack(phases=[3, 6, 10], hit=[4, 6], cancel=9, length=18, forward_mm=150, strike_tick=6, poses=[
        (0, "inout", dict(r_hip=(40, 0, 0), l_hip=(20, 0, 0), r_knee=(-60, 0, 0), l_knee=(-50, 0, 0))),
        (3, "out", dict(r_shoulder=(80, 60, 10), r_elbow=(60, 0, 0), chest=(4, 20, 0), r_hip=(40, 0, 0), r_knee=(-60, 0, 0))),
        (6, "linear", dict(r_shoulder=(85, -50, 10), r_elbow=(10, 0, 0), chest=(8, -25, 0), r_hip=(40, 0, 0), r_knee=(-60, 0, 0))),
        (10, "out", dict(r_shoulder=(70, -70, 10), r_elbow=(25, 0, 0), chest=(8, -30, 0), r_hip=(40, 0, 0), r_knee=(-60, 0, 0))),
        (18, "inout", dict(r_hip=(40, 0, 0), l_hip=(20, 0, 0), r_knee=(-60, 0, 0), l_knee=(-50, 0, 0))),
    ])
    c["air_heavy"] = attack(phases=[8, 12, 20], hit=[9, 12], cancel=22, length=30, forward_mm=200, strike_tick=12, poses=[
        (0, "inout", dict(r_hip=(40, 0, 0), r_knee=(-60, 0, 0))),
        (8, "out", dict(r_shoulder=(165, 0, 5), r_elbow=(40, 0, 0), l_shoulder=(150, 0, -5), chest=(-12, 0, 0), r_hip=(40, 0, 0), r_knee=(-60, 0, 0))),
        (12, "linear", dict(r_shoulder=(40, 0, 5), r_elbow=(5, 0, 0), l_shoulder=(40, 0, -5), chest=(24, 0, 0), r_hip=(40, 0, 0), r_knee=(-60, 0, 0))),
        (20, "out", dict(r_shoulder=(25, 0, 5), r_elbow=(10, 0, 0), l_shoulder=(25, 0, -5), chest=(26, 0, 0), r_hip=(40, 0, 0), r_knee=(-60, 0, 0))),
        (30, "inout", dict(r_hip=(40, 0, 0), r_knee=(-60, 0, 0))),
    ])

    # T6: the air string. A hit on an airborne enemy opens the next one:
    # light, a rising cut, then an overhead chop; heavy, then a rising cut.
    # The slam (the launcher in the air): tool overhead, then down hard;
    # its hit window runs through the dive and the landing.
    tuck = dict(r_hip=(40, 0, 0), l_hip=(20, 0, 0), r_knee=(-60, 0, 0), l_knee=(-50, 0, 0))
    c["air_light_2"] = attack(phases=[4, 8, 12], hit=[5, 8], cancel=11, length=20, forward_mm=150, strike_tick=8, poses=[
        (0, "inout", tuck),
        (4, "out", dict(r_shoulder=(0, 10, 10), r_elbow=(50, 0, 0), chest=(14, 0, 0), **tuck)),
        (8, "linear", dict(r_shoulder=(130, 0, 10), r_elbow=(10, 0, 0), chest=(-8, 0, 0), **tuck)),
        (12, "out", dict(r_shoulder=(140, 0, 10), r_elbow=(20, 0, 0), chest=(-10, 0, 0), **tuck)),
        (20, "inout", tuck),
    ])
    c["air_light_3"] = attack(phases=[6, 9, 14], hit=[7, 9], cancel=13, length=22, forward_mm=200, strike_tick=9, poses=[
        (0, "inout", tuck),
        (6, "out", dict(r_shoulder=(150, 10, 10), r_elbow=(50, 0, 0), chest=(-10, 0, 0), **tuck)),
        (9, "linear", dict(r_shoulder=(50, 0, 10), r_elbow=(5, 0, 0), chest=(20, 0, 0), **tuck)),
        (14, "out", dict(r_shoulder=(35, 0, 10), r_elbow=(15, 0, 0), chest=(22, 0, 0), **tuck)),
        (22, "inout", tuck),
    ])
    c["air_heavy_2"] = attack(phases=[8, 12, 20], hit=[9, 12], cancel=22, length=30, forward_mm=200, strike_tick=12, poses=[
        (0, "inout", tuck),
        (8, "out", dict(r_shoulder=(10, 30, 10), r_elbow=(40, 0, 0), l_shoulder=(10, 20, -5), chest=(18, 20, 0), **tuck)),
        (12, "linear", dict(r_shoulder=(140, -10, 10), r_elbow=(5, 0, 0), l_shoulder=(120, -10, -5), chest=(-12, -10, 0), **tuck)),
        (20, "out", dict(r_shoulder=(155, -10, 10), r_elbow=(10, 0, 0), l_shoulder=(130, -10, -5), chest=(-14, -10, 0), **tuck)),
        (30, "inout", tuck),
    ])
    c["air_slam"] = attack(phases=[7, 11, 20], hit=[7, 20], cancel=30, length=38, forward_mm=100, strike_tick=11, poses=[
        (0, "inout", tuck),
        (7, "out", dict(r_shoulder=(170, 0, 5), r_elbow=(30, 0, 0), l_shoulder=(165, 0, -5), l_elbow=(30, 0, 0), chest=(-14, 0, 0),
                        r_hip=(20, 0, 0), l_hip=(20, 0, 0), r_knee=(-30, 0, 0), l_knee=(-30, 0, 0))),
        (11, "linear", dict(r_shoulder=(30, 0, 5), r_elbow=(0, 0, 0), l_shoulder=(30, 0, -5), l_elbow=(5, 0, 0), chest=(34, 0, 0),
                           r_hip=(10, 0, 0), l_hip=(10, 0, 0), r_knee=(-20, 0, 0), l_knee=(-20, 0, 0))),
        (20, "out", dict(r_shoulder=(20, 0, 5), r_elbow=(5, 0, 0), l_shoulder=(20, 0, -5), l_elbow=(10, 0, 0), chest=(36, 0, 0),
                         r_hip=(50, 0, 0), l_hip=(50, 0, 0), r_knee=(-70, 0, 0), l_knee=(-70, 0, 0))),
        (38, "inout", {}),
    ])

    # The Whirlwind Strike: arms out, two turns of the hips, 30 degrees a tick.
    out = dict(r_shoulder=(80, 0, 70), r_elbow=(5, 0, 0), l_shoulder=(70, 0, -60), l_elbow=(10, 0, 0))
    c["whirl"] = clip(42, merge(
        g(0),
        g(6, "out", **out, chest=(10, 30, 0), hips=(0, 0, 0)),
        g(30, "linear", **out, chest=(10, 30, 0), hips=(0, -720, 0)),
        g(36, "out", **out, chest=(10, 0, 0), hips=(0, -730, 0)),
        g(42, "inout", hips=(0, -720, 0)),
    ), root=[[0, 0, 0], [6, 0, 0], [30, 300, 0], [42, 300, 0]], events={"hit": [7, 30], "phases": [6, 30, 36]})

    # After a cap: the recovery, tool lowered, a breath (24 ticks).
    c["recover"] = clip(24, merge(
        g(0),
        g(8, r_shoulder=(5, 0, 8), r_elbow=(20, 0, 0), chest=(16, 0, 0), r_knee=(-20, 0, 0), l_knee=(-20, 0, 0)),
        g(24),
    ), events={"phases": [0, 0, 0]})
    c["hurt"] = clip(20, merge(g(0), g(4, chest=(-20, 0, 0)), g(20)))
    return c


def dog():
    D = {
        "neck": (0, 0, 0), "head": (0, 0, 0), "jaw": (0, 0, 0), "tail": (0, 0, 0),
        "fl_hip": (0, 0, 0), "fl_knee": (0, 0, 0), "fr_hip": (0, 0, 0), "fr_knee": (0, 0, 0),
        "bl_hip": (0, 0, 0), "bl_knee": (0, 0, 0), "br_hip": (0, 0, 0), "br_knee": (0, 0, 0),
        "hips": (0, 0, 0), "chest": (0, 0, 0),
    }

    def d(t, e="inout", **over):
        f = {j: K(t, *a, e=e) for j, a in D.items() if j not in over}
        for j, a in over.items():
            f[j] = K(t, *a, e=e)
        return f

    c = {}
    c["dog_idle"] = clip(60, merge(d(0), d(30, neck=(-6, 0, 0), tail=(10, 15, 0), head=(4, 8, 0)), d(60)), loop=True)

    def trot(t, a):
        return d(t, fl_hip=(a, 0, 0), br_hip=(a, 0, 0), fr_hip=(-a, 0, 0), bl_hip=(-a, 0, 0),
                 fl_knee=(-abs(a) // 2, 0, 0), br_knee=(-abs(a) // 2, 0, 0), tail=(10, a // 2, 0), neck=(-8, 0, 0))

    c["dog_strafe"] = clip(30, merge(trot(0, 28), trot(15, -28), trot(30, 28)), loop=True)
    crouch = dict(fl_hip=(30, 0, 0), fr_hip=(30, 0, 0), fl_knee=(-60, 0, 0), fr_knee=(-60, 0, 0),
                  bl_hip=(45, 0, 0), br_hip=(45, 0, 0), bl_knee=(-90, 0, 0), br_knee=(-90, 0, 0),
                  neck=(25, 0, 0), head=(-15, 0, 0), tail=(30, 0, 0))
    # The lunge: crouch 18 (the telegraph), leap 12, bite 6, recover 30 (trees review).
    c["dog_lunge"] = clip(66, merge(
        d(0),
        d(18, "out", **crouch),
        d(30, "linear", fl_hip=(70, 0, 0), fr_hip=(70, 0, 0), bl_hip=(-50, 0, 0), br_hip=(-50, 0, 0),
          neck=(-20, 0, 0), head=(-10, 0, 0), jaw=(35, 0, 0), tail=(-20, 0, 0)),
        d(33, "linear", jaw=(-10, 0, 0), neck=(10, 0, 0), fl_hip=(40, 0, 0), fr_hip=(40, 0, 0)),
        d(36, "out", jaw=(0, 0, 0), neck=(15, 0, 0), fl_hip=(30, 0, 0), fr_hip=(30, 0, 0)),
        d(66, "inout"),
    ), root=[[0, 0, 0], [18, -150, -120], [30, 2600, 300], [36, 3000, 0], [66, 3000, 0]],
        events={"hit": [30, 35], "phases": [18, 36, 46]})
    # Round 2 (the animator): the recoil is keyed on the tick of contact and
    # entered as a cut, so the hit-stop holds a dog that has taken the blow
    c["dog_hurt"] = clip(14, merge(d(0, "linear", neck=(-30, 0, 0), head=(-25, 0, 0), chest=(0, 0, 8)), d(14, "out")),
                         events={"cuts": [0, 1]})
    wob = dict(fl_hip=(-10, 0, 15), fr_hip=(-10, 0, -15), bl_hip=(10, 0, 15), br_hip=(10, 0, -15),
               fl_knee=(-30, 0, 0), fr_knee=(-30, 0, 0), bl_knee=(-40, 0, 0), br_knee=(-40, 0, 0), tail=(40, 0, 0))
    c["dog_stunned"] = clip(40, merge(
        d(0, **wob, neck=(30, 20, 0), head=(10, 0, 20)),
        d(20, **wob, neck=(30, -20, 0), head=(10, 0, -20)),
        d(40, **wob, neck=(30, 20, 0), head=(10, 0, 20)),
    ), loop=True)
    c["dog_tumble"] = clip(24, merge(
        d(0, **wob, neck=(-30, 0, 0)),
        d(12, "linear", **wob, neck=(-30, 0, 0), hips=(0, 0, 180)),
        d(24, "linear", **wob, neck=(-30, 0, 0), hips=(0, 0, 360)),
    ), loop=True)
    down = dict(fl_hip=(80, 0, 20), fr_hip=(80, 0, -20), bl_hip=(-80, 0, 20), br_hip=(-80, 0, -20),
                fl_knee=(-10, 0, 0), fr_knee=(-10, 0, 0), bl_knee=(10, 0, 0), br_knee=(10, 0, 0), neck=(40, 0, 0), tail=(-10, 0, 0))
    c["dog_down"] = clip(30, merge(d(0, **down), d(30, **down)), root=[[0, 0, -420], [30, 0, -420]])
    c["dog_dead"] = clip(40, merge(d(0), d(20, "in", **down), d(40, **down, head=(30, 0, 0))),
                         root=[[0, 0, 0], [20, 0, -420], [40, 0, -420]])
    # The large injured dog: lying, breathing, never attacking (cue 79).
    lie = dict(down, neck=(30, 0, 0))
    c["dog_injured"] = clip(80, merge(d(0, **lie), d(40, **dict(lie, chest=(4, 0, 0), neck=(36, 0, 0))), d(80, **lie)),
                            loop=True, root=[[0, 0, -420], [80, 0, -420]])
    return c


def main():
    out = {"clips": {**hero(), **dog()}}
    path = Path(__file__).resolve().parents[1] / "data" / "clips.json"
    path.write_text(json.dumps(out, separators=(",", ":")) + "\n")
    print(f"wrote {path} ({len(out['clips'])} clips)")


if __name__ == "__main__":
    main()
