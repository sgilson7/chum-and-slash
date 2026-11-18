# State

The project's long-term memory, at most two pages. Rewrite it at each milestone (a stage boundary, a finished feature, a deploy) and before a handoff. After a compaction or in a new session, read this file and ask `tools/ask.py` for cards; do not re-read files it already summarises. `scripts/context_check.py` checks its size, its commit and the cards it cites.

Commit: 8edd174

## Where the build is

Stage 5, M0-M5 and M7 built and playable; browser check passes in Chromium (Playwright installed in .venv, R1.4 done). Core modules: fx/geom/mesh/render (POTS), model.rs (rig, data/models.json from tools/models.py), clip.rs (keyed clips, data/clips.json from tools/clips.py, inertialization, allowed-transition graph), world.rs (combat), pilot.rs (Vagrancy trees, data/pilots.json, Pack tokens), tutorial.rs, scene.rs (bodies + LTC-style skyline rings), game.rs (screens, arenas from data/arenas.json, checkpoints, cutscenes, input map data/pads.json, camera, ui view model, uniforms). Shim exposes Game. Page: web/app.js, gl.js (POTS + scenery upload + software check), audio.js (data/audio.json), style.css; data/haptics.json, glyphs.json, text.json (agent-written strings: list them in SECOND-ORDER). Harness: crates/core/examples/harness.rs + data/scenarios/*.txt (--json for journal). Screenshots: tools/shots.py -> ../hack-and-slash-journal/img/build/. 33 breaks caught (tests/breaks.json).

## Decisions in force

Rules CS-1..CS-20 (CLAUDE.md agreed rules). All questions on defaults; CS-Q22 grants Chumworld Arena code. Deploy grant: chum-and-slash after ./scripts/check.sh passes. Notebook rows R1.1-R1.15.

## What is next

1. Journal: screenshots entry (img/build/*.png); list agent strings at foot of SECOND-ORDER.md; LICENSES.md credits (POTS, Vagrancy, LTC, Chumworld Arena, realtime kit).
2. M6 feel rounds: run harness scenarios to files, spawn 3 playtester subagents (veteran, newcomer, animator) on the text, change one thing a round, journal entry with before/after.
3. M6.1 journal animations from harness --json (SnapClass step player pattern).
4. Full ./scripts/check.sh (three engines), then scripts/publish.sh to chum-and-slash; live entry; republish journal artifact https://claude.ai/artifact/HFNKrsPgkiULwcmrmEADUx.

## Cards the next session needs

`card:docs.context.how-to-read-builder`
`card:kits.realtime.drawing-less-on-a-software-renderer`
`card:docs.checks.things-the-browser-check-cannot-see`
