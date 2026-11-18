# Second-order notebook

Stage 6, kept by the agent as the work happens, not afterwards. `B3 Build in self-questioning`.

The code and tests are the work; this file is the record about the work. It is where a decision the agent made alone, an assumption that turned out wrong, or something only the person can do becomes visible to the person. With the plan approved in advance, it is the main place those decisions surface before the person uses the tool. [`docs/procedure.md`](https://github.com/sgilson7/builder-setting/blob/main/docs/procedure.md#the-second-order-notebook) says why.

One row per assumption, surprise or open item. **Kind** is *divergence* (what was assumed, what turned out to be true, what changed), *finding*, or *worklist* (the person's to do). Number rows by round (R1.1, R1.2, then R2.1 after the first triage) so a later round does not renumber an earlier one. **Rule** is the rule the row produced for later sessions, and where it now lives (`CLAUDE.md`, a test, `tests/breaks.json`), or blank. **Status** is *done*, *open*, or *person*; every *person* row is named in `HANDOFF.md`. `scripts/notebook_check.py` checks the form of each row and that the notebook is not empty once `PLAN.md` holds a decision, not what the rows say.

| # | Kind | Assumed or found | What is true | What changed | Rule | Status |
| --- | --- | --- | --- | --- | --- | --- |
| R1.1 | finding | Vagrancy's digest (`refs/vagrancy.md`) would point to its behaviour trees | It names only the road and the chart; the trees live in `crates/pilot` (1,734 lines) | The trees review adds a "What to copy" bullet to the digest before any port (rule 20) | | open |
| R1.2 | divergence | Every reference in the brief has a digest | Protection of the Spheres and Pursuer had none | Shallow clones in `../refs-src/`; digests drafted with `tools/digest.py`; the animation and vistas reviews fill their "What to copy" | | open |
| R1.3 | worklist | Chumworld Arena's code comes under a licence | Its repository has no licence file and GitHub reports none | CS-Q22 A grants it under MIT (DECISIONS.md); credited in LICENSES.md | | done |
| R1.4 | worklist | The browser check runs on this Mac | Playwright is not installed, so step 9 of `check.sh` fails on the untouched template | Install with `scripts/setup.sh` before the first deploy; needs your approval to install packages | | person |
| R1.5 | finding | Sam's note quotes cue 191 as "plays it low noise. Bullshit." | The transcript reads "have it play low noise. Bullshit." | The brief matches the transcript's words and keeps Sam's reading beside it (CS-Q6) | | done |
| R1.6 | finding | The repository can be named "Chum and Slash" | GitHub repository names hold no spaces | Asked as CS-Q10; default `chum-and-slash` | | open |
| R1.7 | divergence | Sam's distant rings come from Chumworld Arena | Its rings are stone stands within 32 m; the building rings far off are Long Time Comin's (`arena.rs:1061-1158`) | Asked as CS-Q24; the plan takes ring placement from Long Time Comin' and depth ranges and haze from Chumworld Arena | | open |
| R1.8 | divergence | Protection of the Spheres' ability animations are keyframed | A pose is seven hinge angles written in Rust: no keys, blends, events or root motion (animation review) | The plan ports its meshes and renderer and builds keyed clips new (M1.1, M2) | | done |
| R1.9 | finding | Vagrancy's trees port into a Q16.16 core as they are | They use Q20.12 and measure distance along x only; about eleven changes, most already made in Chumworld Arena's `pilot.rs` | Asked as CS-Q23 | | open |
| R1.10 | divergence | The realtime kit's `fx.rs` would be the core's fixed point | POTS's `fx.rs`, `geom.rs`, `mesh.rs` and `render.rs` are written against POTS's own `fx` (same Q16.16, same 65,536 units a turn) | POTS's `fx.rs` is the base, with the kit's `sqrt`, `length` and `Fx` added (crates/core/src/fx.rs) | | done |
| R1.11 | finding | The transition test could walk every pair of clips from every tick | 77 jumps over 45°, all from changes the game never makes (cancelling before the cancel tick, a tumbling dog lunging) | The test walks the allowed graph (`clip::allowed`); one real case left (launcher into launcher, 46°) fixed by an 8-tick blend in | `clip::allowed`, `no_joint_jumps_through_a_change_of_clip` | done |
| R1.12 | finding | Chumworld Arena's `feel.rs` ports whole | It is driven by blade speed and contact (sparks, blood, a physics arm); this game's hit-stop comes from the move | Only the camera knock and its ring to exactly zero were ported, with Chumworld Arena's own test sequence | `the_camera_knock_rings_to_exactly_nothing` | done |
| R1.13 | divergence | A parity test replays a recorded Vagrancy fight through the port | Vagrancy's conditions read a 2D sword world (tip near head, x-only gap); no dog fight exists to record | Parity is held where the engine is the same: xorshift64* draws (printed by Vagrancy's own `Rng`), node numbering, reaction delay, interrupts | `parity_with_vagrancy_rng` and three ported tree tests | done |
| R1.14 | finding | Each behaviour has one guard | Breaking the injured dog's `attacks` flag, or a stunned dog's tree rule, left their tests passing: two guards each | The world always asks the tree (CS-10), so the tree alone holds the injured dog; the stunned-dog test uses a lunge-only tree so the core's gate is what it tests | `tests/breaks.json` | done |
| R1.15 | finding | 0.1 m a tick is ONE/10 | Interpolated keys give 32768 − 26214 = 6554 (0.1 m is 6553.6 in Q16) | The hand-worked test uses the interpolated values | | done |

## Strings written by the agent

Each reader-visible string the agent wrote, with its key, for the person to read.

| Key | String |
| --- | --- |
