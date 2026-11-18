# State

The project's long-term memory, at most two pages. Rewrite it at each milestone (a stage boundary, a finished feature, a deploy) and before a handoff. After a compaction or in a new session, read this file and ask `tools/ask.py` for cards; do not re-read files it already summarises. `scripts/context_check.py` checks its size, its commit and the cards it cites.

Commit: bb178f8

## Where the build is

Stage 5, M0 in progress. Plan approved 2026-11-18; Sam said build through to publishing without stopping (DECISIONS.md). All 25 questions on their defaults; CS-Q23/24/25 answered A. Ported so far (MIT, credit owed in LICENSES.md): POTS `fx.rs` (+ kit's sqrt/length, `Fx` alias), `geom.rs`, `mesh.rs`, `render.rs` into crates/core/src; `web/gl.js`, `data/shaders/scene.{vert,frag}`; kit `rng.rs`, `ticks.rs`, `web/input.js`. 46 core lib tests pass. Not yet: everything else. Context passed 200k at this point; Sam asked for no stops, so no /compact request.

## Design settled (build to this)

- Units Q16.16 metres, angles 65536/turn, 60 ticks/s. Gameplay JSON (combat, models, clips, enemies, arenas) embedded in core with include_str!; presentation JSON (audio, haptics, display, strings, pads glyphs) fetched by page.
- Modules: model.rs (two-level joint rigs + parts from data/models.json; hero trousers = shader stripe style, ~44 vertical wales, white/brown, uniform-driven), clip.rs (keyed clips: per-joint keys with ease, events hit window/cancel_from, root curve; sample in Q16; transition offset decays with quintic smootherstep to exactly 0 at N ticks: 6 cancel, 12 locomotion), combat.rs (combo machine: 8-tick buffer, caps light4/heavy3/total6, 24-tick recovery, string reset 35; first combos LHLLH and LHLLH+Launcher recognised as data, ComboDone event; stamina only sprint/air moves/abilities; air dmg x3/2; launcher lifts only stunned to 3 m; air float gravity/4 for 20 ticks; stun 90 refreshed per hit, cut to a short tail when the player's string ends; push-back 0.3/0.8 m; hit-stop 4/7/10 cap 12; poise: stagger else shake), feel (camera knock ring from Chumworld Arena feel.rs: KNOCK_MAX 5 cm, RING_PCT 55, QUIET ONE/2000, test sequence [3276,-1801,990,-544,299,-164,90,-49,0]), pilot.rs (Vagrancy tree port, M4), pack.rs (tokens), tutorial.rs, arena.rs (rings per vistas review, checkpoints, cutscenes), world.rs (World::tick(input)->Frame: scene batch, sound events with Q16 rate, rumble events, harness lines), api/shim stateful Game like POTS shim (frame(now_ms), key, pad, act, ui, render, scene, draws, mats, uniforms, mesh, noise).
- Page: D3 bans numbers/Math in JS, so pitch ratios, gains and noise samples come from the core. Split depth: far pass first, then a clear-depth draw, then near (CS-Q24, vistas review).
- Harness: examples/harness.rs prints per tick clip/frame/phase/hitstop/reaction/node/stamina/sounds/rumbles; --json for journal animations.

## Decisions in force

Rules CS-1..CS-20 (CLAUDE.md "Rules agreed"). PLAN.md milestones M0-M8 and its starting-numbers table. Deploy grant: chum-and-slash after check.sh passes. Person row R1.4 (Playwright install) still open: install via scripts/setup.sh at M8.

## What is next

Write model.rs, clip.rs, combat.rs, world.rs, harness; then shim Game + page; then M4 (read Vagrancy lines named in ../refs/vagrancy.md Behaviour trees bullet), M5-M8.

## Cards the next session needs

`card:docs.context.how-to-read-builder`
`card:kits.realtime.drawing-less-on-a-software-renderer`
`card:kits.canvas.going-further-webgl`
`card:docs.checks.things-the-browser-check-cannot-see`
