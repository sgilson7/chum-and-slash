# Second-order notebook

Stage 6, kept by the agent as the work happens, not afterwards. `B3 Build in self-questioning`.

The code and tests are the work; this file is the record about the work. It is where a decision the agent made alone, an assumption that turned out wrong, or something only the person can do becomes visible to the person. With the plan approved in advance, it is the main place those decisions surface before the person uses the tool. [`docs/procedure.md`](https://github.com/sgilson7/builder-setting/blob/main/docs/procedure.md#the-second-order-notebook) says why.

One row per assumption, surprise or open item. **Kind** is *divergence* (what was assumed, what turned out to be true, what changed), *finding*, or *worklist* (the person's to do). Number rows by round (R1.1, R1.2, then R2.1 after the first triage) so a later round does not renumber an earlier one. **Rule** is the rule the row produced for later sessions, and where it now lives (`CLAUDE.md`, a test, `tests/breaks.json`), or blank. **Status** is *done*, *open*, or *person*; every *person* row is named in `HANDOFF.md`. `scripts/notebook_check.py` checks the form of each row and that the notebook is not empty once `PLAN.md` holds a decision, not what the rows say.

| # | Kind | Assumed or found | What is true | What changed | Rule | Status |
| --- | --- | --- | --- | --- | --- | --- |
| R1.1 | finding | Vagrancy's digest (`refs/vagrancy.md`) would point to its behaviour trees | It names only the road and the chart; the trees live in `crates/pilot` (1,734 lines) | The trees review adds a "What to copy" bullet to the digest before any port (rule 20) | | open |
| R1.2 | divergence | Every reference in the brief has a digest | Protection of the Spheres and Pursuer had none | Shallow clones in `../refs-src/`; digests drafted with `tools/digest.py`; the animation and vistas reviews fill their "What to copy" | | open |
| R1.3 | worklist | Chumworld Arena's code comes under a licence | Its repository has no licence file and GitHub reports none | CS-Q22 A grants it under MIT (DECISIONS.md); credited in LICENSES.md | | done |
| R1.4 | worklist | The browser check runs on this Mac | Playwright is not installed, so step 9 of `check.sh` fails on the untouched template | Installed with `scripts/setup.sh` into `.venv` (Sam: build through to publishing) | | done |
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
| R1.16 | worklist | Story text, arena names and prompts are the person's (BRIEF: decisions that stay with the person) | The agent wrote all of `data/text.json` to make the prologue playable | Listed below under Strings written by the agent, for Sam to read and rewrite | | person |
| R2.1 | divergence | Dogs strafe, then lunge "every once in a while" | Round 1 (newcomer): one dog chain-lunged, given the token again the moment its lunge ended | A 60-tick rest after each lunge before the pack offers the token again | `a_dog_rests_between_lunges` | done |
| R2.2 | finding | A bitten hero plays the hurt clip | Round 1 (newcomer): locomotion replaced it with idle after the 3-tick hit-stop | Locomotion waits for the hurt clip to end | `a_bite_plays_the_hurt_clip_through` | done |
| R2.3 | finding | A jump is taken like an attack | Round 1 (veteran): a jump pressed two ticks before the launcher's cancel tick was dropped; jumps had no buffer | Jumps wait in the 8-tick buffer | `an_early_jump_waits_for_the_cancel` | done |
| R2.4 | finding | Hit-stop stops both bodies on each hit | Round 1 (veteran): a whirl reaching dogs on different ticks stalled the spin twice | The attacker stops once per move | `the_whirl_stops_the_hero_once` | done |
| R2.5 | divergence | The injured large dog is an ordinary enemy that does not attack | Round 2's before run: it stood up into the idle clip, and with its poise it was never stunned, so the launcher could never lift it | It lies stunned for good; hits never shorten that stun | `the_injured_dog_stays_down_and_stunned` | done |
| R2.6 | finding | Each break isolates one guard | After R2.1 and R2.5, the injured dog and the one-lunge rule each have two guards (a lasting stun and a tree without a lunge; the pack's holder and the token gate) | Two breaks retired; the remaining breaks target the core gates | `tests/breaks.json` | done |
| R2.7 | finding | Dogs decide on a token they hold | With reaction delay, a dog acted on a token seen 12 ticks ago, after the pack had moved it: two dogs lunged at once | A dog lunges only on a token it holds now | `pack_gives_one_token_normally` | done |
| R3.1 | finding | The text harness shows what a feel change changes | Round 2's change moved only joint angles; the harness printed clip names and frames, so before and after were byte-identical and all three playtesters said so. I had also told one that transitions were printed, which was false | Each body's line names its three most-turned joints, and the flinch is printed beside the shake | the harness (`World::describe`) | done |
| R3.2 | finding | A buffered press lasts its 8 ticks | It aged before it was tried, so a press made exactly 8 ticks before a cancel tick was dropped on the tick it could start (the lesson's launcher) | Tried first, then aged | `a_press_the_buffer_length_early_still_comes_out` | done |
| R4.1 | finding | The cap tests prove the cap | They pressed each button only after the move allowed it, so none mashed; mashing cancelled the capped fourth light and skipped the recovery for good (T2) | A capped move cannot be cancelled; a mashing test | `mashing_light_still_reaches_the_cap` | done |
| R5.1 | divergence | Arena 4's "large dog" is the kind Arena 3 already has | `big_dog` is Arena 3's injured dog: it never attacks, lies stunned and dies on the first launch. A large dog in a pack fight has to fight back | A new kind, `large_dog` (170% size, 200 health, poise 2, bite 14) with its own tree (slower reaction 24, lunges from further off). The sandbox's "Spawn a large dog" still spawns the injured `big_dog` | `data/combat.json`, `data/pilots.json`; `arena4_is_four_dogs_and_a_large_dog_at_once` Sam to say whether the sandbox button should spawn the fighting one | person |
| R5.2 | finding | Five dogs at once is beatable | The scripted player clears Arena 4 in 1,201 ticks (20 s) with 48 of 100 health left; it loses none in Arenas 1 to 3. Only one dog holds the attack token at a time, so the five take turns | Nothing; the numbers are for Sam to judge in play | `bot_clears_each_arena` | done |
| R6.1 | divergence | Air moves count toward the ground string's caps (4 light, 3 heavy, 6 total) | Under those caps an air combo would stop at four lights and then recover in mid-air, which cuts across "maintain the airtime until you drop the combo" | Air moves stand outside the ground caps; their stamina cost (600 a light, 1,200 a heavy, 1,500 a slam, of 10,000) bounds an air combo. The slam hangs through its wind-up (7 ticks), then dives at 300 mm a tick | `world.rs` `start`, `hero_motion`; `data/combat.json` `air` | done |
| R6.2 | divergence | How a style grade should weigh things | Sam named multi-enemy hits, air time and not repeating a combo | Each hit scores its damage at 100%, +50% for an air move, +25% on an airborne target, +50% for a move's second and later targets, +100% for a slam, halved when the string repeats one of the last 3; +30 a named combo; +1 per 4 ticks of air combo; a bite costs twice its damage. Grade = points per damage dealt: S 180%, A 140%, B 110%, C 80%. The ground-only scripted player grades C, D, A, D in Arenas 1 to 4 | `data/style.json`, `crates/core/src/style.rs` | done |

## Strings written by the agent

Each reader-visible string the agent wrote, with its key, for the person to read.

| Key | String |
| --- | --- |
| `text.title` | Chum and Slash |
| `text.tagline` | The wall has fallen. Get out of the city. |
| `text.start` | Click to start |
| `text.start_note` | Sound and the controller start after a click. |
| `text.menu.prologue` | Play the prologue |
| `text.menu.sandbox` | Sandbox |
| `text.menu.controls` | Keyboard and mouse, or plug in a PlayStation or Xbox controller. |
| `text.arena1.title` | The park above the lake |
| `text.arena1.c1` | The wall fell at dawn. The Defence Force is gone, and the colony is leaving the city. |
| `text.arena1.c2` | Your utility tool is in your hand: axe, pick and blade. Fast things slow near the fauna, so everyone fights up close. |
| `text.arena1.c3` | Something bright moves between the trees. |
| `text.arena2.title` | The overlook |
| `text.arena2.c1` | A colonist lies by the path, badly hurt. |
| `text.arena2.c2` | "They hunt in threes. Hold the ability button and strike, and spin: it stops all of them at once." |
| `text.arena2.c3` | You have learned the Whirlwind Strike. |
| `text.arena3.title` | The lakeside steps |
| `text.arena3.c1` | A guard waves you down toward the water. |
| `text.arena3.c2` | "That big one is badly hurt and will not fight back. Light, heavy, light, light, heavy, then launch it." |
| `text.arena3.c3` | Finish it with that string. |
| `text.arena4.title` | The north gate |
| `text.arena4.c1` | The last carts of the colony roll toward the north gate. |
| `text.arena4.c2` | A pack has found the road: four dogs and a large one, all at once. |
| `text.arena4.c3` | Hold the gate until the carts are through. |
| `text.hud.style` | Style |
| `text.style.grade` | Style grade |
| `text.style.air` | Air time |
| `text.style.most` | Most hit at once |
| `text.style.slams` | Slams |
| `text.style.repeats` | Strings repeated |
| `text.style.bites` | Bites taken |
| `text.sandbox.title` | Sandbox |
| `text.sandbox.spawn_dog` | Spawn a dog |
| `text.sandbox.spawn_pup` | Spawn a young dog |
| `text.sandbox.spawn_big` | Spawn a large dog |
| `text.sandbox.clear` | Clear the arena |
| `text.sandbox.arena` | Go to an arena |
| `text.skip` | Hold to skip |
| `text.next` | Go on |
| `text.won` | The way is clear. |
| `text.won.last` | The prologue's four fights are done. More of the city waits. |
| `text.lost` | You fell. The fight starts again from the checkpoint. |
| `text.retry` | Try again |
| `text.continue` | Go on |
| `text.menu` | Back to the menu |
| `text.resume` | Resume |
| `text.paused` | Paused |
| `text.hint` | Press the button shown to do the move. |
| `text.strip.again` | The string broke. Start again with a light attack. |
| `text.hud.health` | Health |
| `text.hud.stamina` | Stamina |
| `text.hud.lock` | Lock-on |
| `text.action.light` | Light attack |
| `text.action.heavy` | Heavy attack |
| `text.action.launcher` | Launcher |
| `text.action.jump` | Jump |
| `text.action.whirl` | Whirlwind Strike |
| `text.no_webgl` | This page needs WebGL 2. Try another browser. |
