#!/usr/bin/env python3
"""Builds the journal's "Moving parts" animations from the harness's output
(CS-15). Every step drawn comes from the core: the harness's --json files and
the tutorial trace in ../hack-and-slash-journal/anim/. Nothing is timed by
hand. The player follows the SnapClass Guide journal's request lifecycles
(steps lighting parts of an SVG, a key, Play, Pause, Step, Restart, and every
step as text) and the MerryQuery journal's step players.

    python3 tools/journal_anim.py      writes anim/moving.html and anim/player.js
"""

import html
import json
import pathlib

J = pathlib.Path(__file__).resolve().parents[2] / "hack-and-slash-journal"
A = J / "anim"
esc = html.escape


def load(name):
    return json.loads((A / f"{name}.json").read_text())


def svg(w, h, body, label):
    return (f'<div class="mv-scroll"><svg class="mv-svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" '
            f'role="img" aria-label="{esc(label)}">{body}</svg></div>')


def box(id_, x, y, w, h, text, cls="mv-node"):
    return (f'<g id="{id_}" class="{cls}"><rect x="{x}" y="{y}" width="{w}" height="{h}" rx="6"/>'
            f'<text x="{x + w / 2}" y="{y + h / 2 + 5}" text-anchor="middle">{esc(text)}</text></g>')


def figure(key, title, made_by, svg_html, steps, legend):
    data = json.dumps(steps, separators=(",", ":"))
    items = "".join(f"<li>{esc(s['text'])}</li>" for s in steps)
    leg = "".join(f'<li><span class="mv-key {c}"></span>{esc(t)}</li>' for c, t in legend)
    return f'''<figure class="mv" id="mv-{key}" data-steps='{esc(data)}'>
  <figcaption><b>{esc(title)}</b> Made by <code>{esc(made_by)}</code>.</figcaption>
  {svg_html}
  <p class="mv-now" aria-live="polite"></p>
  <div class="mv-ctl"><button type="button" data-do="play">Play</button><button type="button" data-do="pause">Pause</button><button type="button" data-do="step">Step</button><button type="button" data-do="restart">Restart</button><span class="mv-count"></span></div>
  <ul class="mv-legend">{leg}</ul>
  <details class="mv-list"><summary>Every step as text ({len(steps)})</summary><ol>{items}</ol></details>
</figure>'''


# ---------------------------------------------------------------- 1 combo flow

def combo_flow():
    d = load("first_combo")
    c = d["combo"]
    moves = ["light_1", "light_2", "light_3", "light_4", "heavy_1", "heavy_2", "heavy_3", "launcher"]
    body = '<text class="mv-h" x="20" y="26">press</text><text class="mv-h" x="250" y="26">move</text><text class="mv-h" x="520" y="26">string</text>'
    for i, p in enumerate(["light", "heavy", "launcher"]):
        body += box(f"cf-p-{p}", 20, 44 + i * 60, 150, 40, p)
    for i, m in enumerate(moves):
        body += box(f"cf-m-{m}", 250, 40 + i * 30, 160, 26, m)
    body += box("cf-rec", 250, 290, 160, 30, f"recovery {c['recovery']} ticks", "mv-node mv-warn")
    for k in range(c["cap_total"]):
        body += box(f"cf-s-{k}", 470 + (k % 3) * 70, 44 + (k // 3) * 50, 60, 36, f"{k + 1}")
    body += f'<text class="mv-note" x="470" y="170">caps: light {c["cap_light"]}, heavy {c["cap_heavy"]}, all {c["cap_total"]}</text>'
    for i, cb in enumerate(d["combos"]):
        body += box(f"cf-c-{cb['name']}", 470, 190 + i * 44, 200, 34, f"combo: {cb['name']}", "mv-node mv-good")
    steps, prev, string = [], None, []
    for s in d["steps"]:
        h = s["hero"]
        names = [e["name"] for e in s["events"]]
        kinds = [e["kind"] for e in s["events"]]
        lit = []
        text = None
        if h["clip"] != prev and h["clip"] in moves:
            press = "launcher" if h["clip"] == "launcher" else h["clip"].split("_")[0]
            lit = [f"cf-p-{press}", f"cf-m-{h['clip']}"] + [f"cf-s-{k}" for k in range(len(h["string"]))]
            text = f"Tick {s['t']}: {press} starts {h['clip']}; the string is {len(h['string'])} long."
        if "combo" in kinds:
            name = s["events"][kinds.index("combo")]["name"]
            lit = lit + [f"cf-c-{name}"] + [f"cf-s-{k}" for k in range(len(h["string"]))]
            text = (text or f"Tick {s['t']}:") + f" The string matches the combo '{name}'."
        if h["recovery"] and prev != "recover" and h["clip"] == "recover":
            lit = ["cf-rec"]
            text = f"Tick {s['t']}: the cap is reached; {h['recovery']} ticks of recovery, no attack starts."
        if "die" in kinds:
            lit = ["cf-c-first_launch"]
            text = f"Tick {s['t']}: the launcher lands the combo's last hit and the injured dog dies."
        if text:
            steps.append({"lit": lit, "text": text})
        prev = h["clip"]
    return figure("combo", "The first combos, press by press.", d["made_by"], svg(700, 340, body, "Presses, moves, the string and the combos"),
                  steps, [("mv-on", "lit: what this step did"), ("mv-good", "a combo the core recognised"), ("mv-warn", "the recovery after a cap")])


# ---------------------------------------------------------------- 2 clip states and blends

def clip_states():
    d = load("first_combo")
    seen = []
    for s in d["steps"]:
        c = s["hero"]["clip"]
        if not seen or seen[-1] != c:
            seen.append(c)
    order = []
    for c in seen:
        if c not in order:
            order.append(c)
    body = ""
    pos = {}
    for i, c in enumerate(order):
        x, y = 30 + (i % 5) * 150, 40 + (i // 5) * 110
        pos[c] = (x, y)
        body += box(f"cs-{c}", x, y, 120, 36, c)
    steps, prev = [], None
    for s in d["steps"]:
        c = s["hero"]["clip"]
        if c != prev and prev is not None:
            b = d["blends"][c]
            (x1, y1), (x2, y2) = pos[prev], pos[c]
            eid = f"cs-e-{prev}-{c}"
            if eid not in body:
                body = (f'<g id="{eid}" class="mv-edge"><line x1="{x1 + 60}" y1="{y1 + 36}" x2="{x2 + 60}" y2="{y2}"/>'
                        f'<text x="{(x1 + x2) / 2 + 64}" y="{(y1 + y2) / 2 + 22}">{b}</text></g>') + body
            kind = "a cut" if b == 0 else f"a {b}-tick blend"
            steps.append({"lit": [f"cs-{prev}", f"cs-{c}", eid], "text": f"Tick {s['t']}: {prev} gives way to {c}, {kind} that fades to exactly zero (CS-11)."})
        prev = c
    rows = (len(order) + 4) // 5
    return figure("clips", "The hero's animation states, and the blend into each.", d["made_by"],
                  svg(780, 40 + rows * 110, body, "The hero's clips and the transitions between them"), steps,
                  [("mv-on", "the clip now playing and the one before"), ("mv-edge-key", "a transition, labelled with its blend in ticks")])


# ---------------------------------------------------------------- 3 the dog's tree, lit

def tree_lit():
    d = load("dog_fight")
    t = d["trees"]["dog"]
    body = box("tr-0", 20, 20, 150, 34, "selector")
    y = 70
    for r in t["rules"]:
        x = 60
        if r["seq"] is not None:
            body += box(f"tr-{r['seq']}", x, y, 90, 30, "sequence")
            x += 100
        for c in r["conds"]:
            label = c["cond"].replace("GapAbove", "gap >").replace("GapBelow", "gap <").replace("(", " ").replace(")", "")
            body += box(f"tr-{c['id']}", x, y, 150, 30, label, "mv-node mv-cond")
            x += 160
        body += box(f"tr-{r['act_id']}", x, y, 130, 30, r["act"] + (" (interrupt)" if r["interrupt"] else ""), "mv-node mv-act")
        y += 42
    steps, prev = [], None
    for s in d["steps"]:
        e = s["enemies"][0]
        key = (tuple(e["active"]), e["clip"], e["state"])
        if key != prev:
            lit = [f"tr-{i}" for i in e["active"]]
            failed = [f"tr-{i}" for i in e["failed"]]
            steps.append({"lit": lit, "dim": failed, "text": f"Tick {s['t']}: the dog is {e['state']}, playing {e['clip']}; its tree runs '{e['node']}'."})
        prev = key
    return figure("tree", "The dog's behaviour tree, lit as a fight replays.", d["made_by"], svg(760, y + 10, body, "The dog's behaviour tree"),
                  steps, [("mv-on", "the running branch"), ("mv-dim", "a condition checked this tick that failed")])


# ---------------------------------------------------------------- 4 stun, launch and air

def lane_runs(values):
    runs, start = [], 0
    for i in range(1, len(values) + 1):
        if i == len(values) or values[i] != values[start]:
            runs.append((start, i, values[start]))
            start = i
    return runs


def air_states():
    d = load("launch_juggle")
    st = d["steps"]
    n = len(st)
    W, X0 = 4, 120
    body = '<text class="mv-h" x="10" y="40">dog state</text><text class="mv-h" x="10" y="100">heights</text><text class="mv-h" x="10" y="210">hero</text>'
    cls = {"idle": "mv-idle", "stunned": "mv-stun", "airborne": "mv-air", "down": "mv-down", "dead": "mv-dead"}
    for a, b, v in lane_runs([s["enemies"][0]["state"] for s in st]):
        body += f'<rect class="{cls[v]}" x="{X0 + a * W}" y="24" width="{(b - a) * W}" height="24"><title>{v}</title></rect>'
        if b - a > 10:
            body += f'<text class="mv-note" x="{X0 + a * W + 3}" y="64">{v}</text>'
    top = max(1, max(max(s["enemies"][0]["y_mm"], s["hero"]["y_mm"]) for s in st))
    pts = " ".join(f"{X0 + i * W},{180 - s['enemies'][0]['y_mm'] * 80 // top}" for i, s in enumerate(st))
    hp = " ".join(f"{X0 + i * W},{180 - s['hero']['y_mm'] * 80 // top}" for i, s in enumerate(st))
    body += f'<polyline class="mv-line-dog" points="{pts}"/><polyline class="mv-line-hero" points="{hp}"/>'
    for a, b, v in lane_runs([s["hero"]["clip"] for s in st]):
        body += f'<rect class="mv-clip" x="{X0 + a * W}" y="196" width="{(b - a) * W}" height="20"><title>{v}</title></rect>'
        if b - a > 12:
            body += f'<text class="mv-note" x="{X0 + a * W + 2}" y="232">{v}</text>'
    body += f'<line id="air-cursor" class="mv-cursor" x1="{X0}" y1="16" x2="{X0}" y2="240"/>'
    steps, prev = [], None
    for i, s in enumerate(st):
        e = s["enemies"][0]
        ev = ", ".join(f"{x['kind']} {x['name']}" for x in s["events"] if x["kind"] in ("hit", "land", "jump", "die"))
        key = (e["state"], s["hero"]["clip"], ev)
        if key != prev or ev:
            steps.append({"x": X0 + i * W, "cursor": "air-cursor", "lit": [], "text": f"Tick {s['t']}: dog {e['state']} at {e['y_mm']} mm, hero {s['hero']['clip']} at {s['hero']['y_mm']} mm{'; ' + ev if ev else ''}."})
        prev = key
    return figure("air", "Stun, launch and the air, over time.", d["made_by"], svg(X0 + n * W + 20, 250, body, "The dog's state and height, and the hero's clips, tick by tick"),
                  steps, [("mv-stun", "stunned"), ("mv-air", "airborne"), ("mv-down", "down"), ("mv-idle", "idle"), ("mv-line-dog-key", "dog's height"), ("mv-line-hero-key", "hero's height")])


# ---------------------------------------------------------------- 5 hit-stop and reaction

def hitstop():
    d = load("first_combo")
    st = [s for s in d["steps"] if s["t"] <= 50]
    W, X0 = 14, 130
    body = '<text class="mv-h" x="10" y="40">hero phase</text><text class="mv-h" x="10" y="80">hit-stop</text><text class="mv-h" x="10" y="120">dog</text><text class="mv-h" x="10" y="160">heard, felt</text>'
    pc = {"anticipation": "mv-ant", "strike": "mv-strike", "follow-through": "mv-follow", "recovery": "mv-rec", "": "mv-idle"}
    for i, s in enumerate(st):
        x = X0 + i * W
        body += f'<rect class="{pc.get(s["hero"]["phase"], "mv-idle")}" x="{x}" y="26" width="{W - 1}" height="22"><title>{s["hero"]["clip"]} {s["hero"]["phase"]}</title></rect>'
        if s["hero"]["hitstop"]:
            body += f'<rect class="mv-stop" x="{x}" y="66" width="{W - 1}" height="22"><title>hit-stop {s["hero"]["hitstop"]}</title></rect>'
        e = s["enemies"][0]
        if e["shake"]:
            body += f'<rect class="mv-shake" x="{x}" y="106" width="{W - 1}" height="22"><title>shake</title></rect>'
        for ev in s["events"]:
            if ev["kind"] in ("sound", "rumble"):
                y = 146 if ev["kind"] == "sound" else 166
                body += f'<circle class="mv-{ev["kind"]}" cx="{x + W / 2}" cy="{y}" r="5"><title>{ev["name"]}</title></circle>'
        if s["t"] % 5 == 0:
            body += f'<text class="mv-tick" x="{x}" y="196">{s["t"]}</text>'
    body += f'<line id="hs-cursor" class="mv-cursor" x1="{X0}" y1="16" x2="{X0}" y2="182"/>'
    steps = []
    for i, s in enumerate(st):
        h = s["hero"]
        e = s["enemies"][0]
        ev = ", ".join(f"{x['kind']} {x['name']}" for x in s["events"])
        steps.append({"x": X0 + i * W + W / 2, "cursor": "hs-cursor", "lit": [],
                      "text": f"Tick {s['t']}: {h['clip']} frame {h['frame']} ({h['phase'] or 'idle'}), hit-stop {h['hitstop']}; dog {e['reaction'] or 'untouched'}{', shake ' + str(e['shake']) if e['shake'] else ''}{'; ' + ev if ev else ''}."})
    return figure("hitstop", "Hit-stop and the hit's reaction, tick by tick.", d["made_by"], svg(X0 + len(st) * W + 20, 206, body, "Phases, hit-stop, the dog's reaction, sounds and rumbles over the first fifty ticks"),
                  steps, [("mv-ant", "anticipation"), ("mv-strike", "strike"), ("mv-follow", "follow-through"), ("mv-rec", "recovery"), ("mv-stop", "hit-stop"), ("mv-shake", "the dog shakes"), ("mv-sound", "a sound"), ("mv-rumble", "a rumble")])


# ---------------------------------------------------------------- 6 the tutorial

def tutorial():
    d = load("tutorial")
    st = d["steps"]
    W, X0 = 3, 110
    body = '<text class="mv-h" x="10" y="40">game speed</text><text class="mv-h" x="10" y="150">tutorial</text>'
    pts = " ".join(f"{X0 + i * W},{110 - s['speed_pct'] * 80 // 100}" for i, s in enumerate(st))
    body += f'<polyline class="mv-line-dog" points="{pts}"/>'
    pcl = {"off": "mv-idle", "slowing": "mv-ant", "holding": "mv-follow", "paused": "mv-stop"}
    for a, b, v in lane_runs([s["phase"] for s in st]):
        body += f'<rect class="{pcl[v]}" x="{X0 + a * W}" y="136" width="{(b - a) * W}" height="22"><title>{v}</title></rect>'
        if b - a > 8:
            body += f'<text class="mv-note" x="{X0 + a * W + 2}" y="176">{v}</text>'
    for i, s in enumerate(st):
        if s["pressed"]:
            body += f'<g class="mv-good"><circle cx="{X0 + i * W}" cy="196" r="7"/></g><text class="mv-note" x="{X0 + i * W + 10}" y="200">{s["asking"]} pressed</text>'
    body += f'<line id="tu-cursor" class="mv-cursor" x1="{X0}" y1="16" x2="{X0}" y2="206"/>'
    steps, prev = [], None
    for i, s in enumerate(st):
        key = (s["phase"], s["speed_pct"], s["pressed"], s["hero_clip"])
        if key != prev:
            ask = f", asking for {s['asking']}" if s["asking"] else ""
            steps.append({"x": X0 + i * W, "cursor": "tu-cursor", "lit": [],
                          "text": f"Frame {s['t']}: {s['phase']}, speed {s['speed_pct']}%{ask}; the hero plays {s['hero_clip']}, the dog {s['dog_clip']}{' (the press)' if s['pressed'] else ''}."})
        prev = key
    return figure("tutorial", "The tutorial: slow, hold, pause, prompt, and the press that does the move.", d["made_by"],
                  svg(X0 + len(st) * W + 120, 216, body, "Game speed and the tutorial's phase, frame by frame"),
                  steps, [("mv-ant", "slowing"), ("mv-follow", "holding at a fifth"), ("mv-stop", "paused, waiting"), ("mv-good", "the asked-for press")])


def main():
    parts = [combo_flow(), clip_states(), tree_lit(), air_states(), hitstop(), tutorial()]
    (A / "moving.html").write_text("\n".join(parts) + "\n")
    print("wrote", A / "moving.html", sum(p.count("<li>") for p in parts), "steps")


if __name__ == "__main__":
    main()
