# Handoff

Stage 9, written by the agent. `C5 Consolidate in your own words`. A fresh session starts from this file, not from memory.

Commit: 98f6240 (published); later commits are notes and the journal

## What this is

Chum and Slash: a third-person sci-fi hack and slash for the browser, after Devil May Cry 5 and Dragon's Dogma 2, with characters built from primitives. The MVP: the prologue's three arenas (tutorials, checkpoints, skippable cutscenes) and a sandbox. Live at https://sgilson7.github.io/chum-and-slash/ from the public repository sgilson7/chum-and-slash. Keyboard and mouse, or a standard-mapped PS5 or Xbox pad with rumble.

## Rules that hold it up

CS-1 to CS-20 (CLAUDE.md, "Rules agreed"). Sam's combat rules each have a core test in `crates/core/tests/combat.rs`; CS-2 `every_landed_hit_is_told_four_ways`; CS-11 `no_joint_jumps_between_ticks` and `no_joint_jumps_through_a_change_of_clip`; CS-12 `prompt_resumes_only_on_its_input_and_performs_it`; CS-13 `checkpoint_before_each_fight`, `every_cutscene_skips`; M7.3 `bot_clears_each_arena`. 40 breaks in `tests/breaks.json` each turn a test red.

## Shape of the code

| Where | What |
| --- | --- |
| `crates/core/src/world.rs` | the fight: combo machine, stamina, stun, launch, air, hits, hit-stop, shake, flinch, sounds and rumbles as events |
| `clip.rs`, `model.rs` | keyed clips and the two-level rig (data from `tools/clips.py`, `tools/models.py`) |
| `pilot.rs` | Vagrancy's trees and the pack's tokens (`data/pilots.json`) |
| `tutorial.rs`, `game.rs` | prompts; screens, arenas, checkpoints, input mapping, camera, view model |
| `scene.rs`, `render.rs`, `mesh.rs` | the draw list and the vista rings (after POTS and Long Time Comin') |
| `web/` | app.js (input, loop, UI), gl.js (POTS's WebGL), audio.js |
| `crates/core/examples/harness.rs` | the text harness; `tutorial_trace.rs` for the journal |

## Commands

`cargo test` (126 tests); `./scripts/build.sh`; `PATH=$PWD/.venv/bin:$PATH ./scripts/check.sh` (passes, three engines); `python3 scripts/break_tests.py` (40 of 40 caught); `cargo run -q --example harness -- data/scenarios/first_combo.txt [--json out.json]`; `.venv/bin/python tools/shots.py`; `python3 tools/journal_anim.py`; `./scripts/publish.sh` then `BUILDER_ORIGIN=https://sgilson7.github.io/chum-and-slash .venv/bin/python tests/browser/check.py`.

## What will bite within the hour

`cargo fmt` rewraps long lines, so scripted edits must match the formatted text. Q16 products must go through `fx::mul` (three overflows came from `int(n) * scale`). The harness prints the top three joints only: a feel change it cannot print is a change playtesters cannot see (R3.1). publish.sh needs the repository's local git identity, now set.

## The journal

`../hack-and-slash-journal/index.html`, private at https://claude.ai/artifact/HFNKrsPgkiULwcmrmEADUx. Newest entries: 2026-11-18-live and 2026-11-18-feel-round-5. Pages: How I read this, The brief, Research (7 reviews), Plan, The build, Feel rounds (5), Moving parts (6 animations from the harness), Live now.

## Open for the person

Every `SECOND-ORDER.md` row with status *person*, by number, with what the person has to decide or supply. `scripts/notebook_check.py` fails while one is missing.

- R1.16: read and rewrite the story text, arena names and prompts in `data/text.json` (listed at the foot of `SECOND-ORDER.md`).

## Mistakes so far

Feel: round 2 measured nothing because the harness printed no poses (R3.1). Core: a buffer that aged before it was tried (R3.2); tokens acted on after the pack moved them (R2.7); Q16 overflows in tests and tutorial (fixed with `fx::mul`). Tests: breaks that a second guard masked (R1.14, R2.6).

## What passed

2026-11-18, commit 98f6240: `./scripts/check.sh` BUILDER CHECK PASSED (browser check in chromium, firefox, webkit); `./scripts/count-tests.sh` 126 passed, 0 failed; the browser check against https://sgilson7.github.io/chum-and-slash passed in all three engines.

Tokens (`tools/tokens.py`): | 2026-11-18 14:35 | 07850e6c | 254 calls | 155k at first plan | peak 659k | mean 398k | 1 compaction | 100.3M re-read | 53% above 200k |

## After the first deploy

- T1: the camera sits up and to the hero's side (6 m back, 1.2 m aside, 1.5 m up, `data/look.json`), and the mouse or right stick steers it; lock-on turns it toward the target only after 90 quiet ticks.
- T3: the sandbox's spawn tools were out of reach of the locked mouse. Escape now opens a pause card that carries them; `sandbox_pause_card_spawns`.
- T2: mashing light could not reach the cap's recovery (a capped move was cancellable). A capped move now plays out; `mashing_light_still_reaches_the_cap`.

## The next action

Sam plays the republished build with a PS5 pad; then feel round 6: "a kill frees the hero".
