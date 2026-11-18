# Plan

Stage 2, written by the agent from `BRIEF.md` and the seven literature reviews in the journal (Research page, 2026-11-18). `B2 Require step-by-step justification`.

Each decision in build order, with the reason for it and the test that will show it holds. The person reads this at stage 3 (`C3 Interrogate the claim`) and accepts, revises, or answers the open questions. Rules CS-1 to CS-20 are the opening entry's; questions CS-Q1 to CS-Q25 are the journal's answer chips. The plan builds on every question's default until Sam answers.

## Build order

Eight milestones. Each ends with `./scripts/quick-check.sh` passing, a journal entry, and `STATE.md` rewritten. M6 is the feel loop Sam asked for; it starts as soon as M3 gives the harness something to report.

| # | Decision | Reason | Test that shows it holds |
| --- | --- | --- | --- |
| M0.1 | Copy `kits/realtime/` (fixed 60 Hz tick, pointer lock with fallback, Q16.16 `fx.rs`, seeded `rng.rs`, probes). | The game is realtime; D2 bans floats and clocks in the core (CS-1, CS-Q12). | The kit's own tests; `boundary.rs` |
| M0.2 | One `World::tick(input) -> Frame` in the core. `Frame` holds the scene (poses of primitives), and lists of sound events, rumble events and harness lines. The page and the harness call the same function. | The playtesters, the journal animations and the page must read one simulation, not three (CS-15). | `harness_and_page_read_the_same_frame` (a recorded input replayed through both entry points hashes the same) |
| M0.3 | The text harness: `cargo run --example harness -- <script>` plays a scripted input file and prints one line per tick: tick, each actor's clip, frame and phase (anticipation, strike, follow-through, recovery), hit-stop left, the enemy's state, reaction and tree node, stamina, and the sounds and rumbles fired. `--json` writes the same steps for the journal. | Sam asked for a harness that tells an agent what played and how it looks (cues 157, 166). | `harness_line_names_clip_frame_phase_hitstop_reaction_sound_rumble` |
| M0.4 | Input as actions: the page passes raw keys, mouse buttons and standard-mapped pad buttons and axes; the core maps them through `data/pads.json` (CS-Q17, CS-Q18). A pad counts only when `mapping === "standard"`; the page polls `getGamepads()` each frame. | No remap library is needed: Chrome, Edge, Safari and Firefox 152+ report both pads as standard (controllers review). Polling keeps one path for real and scripted pads. | `pad_buttons_map_to_actions`; browser check with a scripted `getGamepads` (CS-14) |
| M1.1 | Port Protection of the Spheres' meshes from primitives, instance packing, WebGL 2 renderer, shaders and fixed-step loop (MIT, credited), per `refs/pots.md`'s "What to copy" (CS-Q11). Draw between the last two ticks (Fiedler), not ahead of the latest. | POTS already splits core and page Builder's way; the animation review found it samples ahead and overflows an i32 after about 18 minutes, so those two parts are written anew. | `render_alpha_interpolates_between_last_two_ticks`; a still view draws the same pixels (browser check) |
| M1.2 | The hero and the dog as models in `data/models.json`: two-level joints (shoulder and elbow, hip and knee), three angles each. Hero: black shirt, green coat, cuffed corduroy trousers with alternating white and brown stripes about 1 cm wide, a bladed utility tool that glows. Dog: bright, long-limbed, plant-like. | CS-20; POTS's one-hinge joints cannot show an elbow or a knee. | `hero_model_has_the_described_parts_and_colours` (CS-20) |
| M1.3 | The sandbox screen: a flat park, a spawn menu (dog, large dog), lock-on, and an arena picker. | The MVP's sandbox, and the place every later milestone is tried. | `sandbox_spawns_each_enemy_kind`; browser check opens the sandbox and spawns a dog |
| M1.4 | A frame-time probe and the software-renderer draw scale (`data/display.json`). | Browser first (CS-18); SwiftShader cost is per instance, not per triangle (vistas review, Long Time Comin' G7). | Probe numbers in `MEASUREMENTS.md` |
| M2.1 | Keyed clips in `data/clips.json`: keys per joint, easing per segment, event spans (hit window, cancel window, sound and rumble cues) and a root curve, sampled in the core in Q16.16. | Sam asked for keyframes and smooth in-betweening (cues 163-165); POTS has no keyframes (animation review). | `clip_samples_match_hand_worked_keys` |
| M2.2 | Transitions by inertialization (Bollo's quintic, reaching zero on a set tick): 6 ticks for attack cancels, 12 for locomotion changes. | Fewer pops than crossfades at lower cost (Gears of War 4); a fixed end tick suits Q16.16 and a test. | `inertialization_reaches_zero_on_its_last_tick` |
| M2.3 | Root motion is a request: the clip's curve says how far forward and up, the core's physics applies it, aimed at the lock-on target. | CS-9: attacks move the attacker forward; the core owns collision. | `every_attack_moves_its_attacker_forward` (CS-9) |
| M2.4 | Hit-stop stops the clip clock at contact; the target blends into its hurt pose over 4 ticks inside the stop, or shakes in place when it does not stagger. | Sakurai's refinements (combat review); Sam's "shake in place a tiny bit" (cue 123). | Part of `every_landed_hit_is_told_four_ways` (CS-2) |
| M2.5 | No joint moves more than 45° between two ticks, except at a declared cut, through every allowed transition. | Sam: "the worst thing that can happen in a game is that the game feels very, like, jerky" (cue 164). | `no_joint_jumps_between_ticks` (CS-11) |
| M3.1 | The combo machine from `data/combos.json`: light, heavy, launcher and jump; an 8-tick input buffer; caps of light 4, heavy 3 and interleaved 6 (CS-Q20); a 24-tick recovery after a cap in which no attack starts. | CS-7; the caps sit inside DMC5's own string lengths (combat review). | `light_string_caps_at_four`, `heavy_caps_at_three`, `interleaved_caps_at_six`, `no_attack_starts_in_recovery` |
| M3.2 | The first combos: light, heavy, light, light, heavy, and the same ending in a launcher (CS-Q7). | CS-8. | `first_combos_perform_from_their_inputs` |
| M3.3 | Stamina spent only by sprinting, moves in the air and abilities; jumping is free (CS-Q9). | CS-3; Dragon's Dogma 2's model minus blocking and climbing (combat review). | `only_sprint_air_moves_and_abilities_spend_stamina` |
| M3.4 | Stun: a long first window (90 ticks) refreshed by each hit, ending when the combo ends (CS-Q8). A launcher lifts only a stunned enemy, 3.0 m. Air hits float the target (gravity × 0.25 for 20 ticks) and deal 150% damage. | CS-4, CS-5; Dragon's Dogma 2 already gates strong follow-ups on an off-balance target (combat review). | `launcher_lifts_only_a_stunned_enemy`, `air_hits_deal_half_again`, `stun_lasts_while_the_combo_lasts` |
| M3.5 | Hit feel from Chumworld Arena's `feel.rs` (hit-stop, camera knock, stagger, decaying to exactly zero), if CS-Q22 grants it; hit-stop 4, 7 and 10 ticks for light, heavy and launcher, capped at 12. Push-back 0.3 m light, 0.8 m heavy. | CS-2, CS-9. | `every_landed_hit_is_told_four_ways`, `every_landed_hit_pushes_its_target_back` |
| M3.6 | Sound and rumble as core events: a swing always fires a whoosh; contact adds transient, body, flesh and tail layers; heavy adds the low layer; a rising combo pitch of one semitone a hit, capped at five; variation from a hash of tick and enemy id. Rumble per hit kind from `data/haptics.json` (light 0.0/0.35 for 4 ticks, heavy 0.55/0.45 for 7, launcher 1.0/0.6 for 14); a weaker rumble never cuts a stronger one. | CS-2; Sam's hit and miss sounds (cues 189-194) and rumble steps (cue 104); sound and controllers reviews. | `a_miss_sounds_unlike_a_hit`, `rumble_rises_light_heavy_launcher` |
| M3.7 | The Whirlwind Strike on the ability modifier plus light: a two-turn spin that stuns every enemy within reach and costs stamina. | Sam's first ability (cues 55-69). | `whirlwind_stuns_every_dog_in_reach`, `abilities_cost_stamina` |
| M3.8 | Lock-on on by default when a fight starts; the lock-on button releases it for freestyle and takes it back (CS-Q21). | Cue 41. | `lock_on_starts_with_the_fight_and_toggles` |
| M4.1 | Port the behaviour trees per CS-Q23 and `refs/vagrancy.md`'s Behaviour trees bullet: the rule list and decision loop, beats and recipes, the trace, and the tests the trees review marks for porting; change Q20.12 to Q16.16 and x-only distance to the ground plane. | Sam: "completely reuse that implementation" (cue 64); CS-10. | Vagrancy's ported tests; `parity_with_vagrancy_on_a_recorded_fight` |
| M4.2 | Every enemy tree starts with stunned → reel and airborne → tumble (both interrupting), then idle, then combat; the core also ignores an enemy's input while it is stunned or airborne. | CS-6; 74 of Vagrancy's 77 trees already start with a knocked-down rule (trees review). | `every_enemy_kind_has_idle_stunned_airborne` |
| M4.3 | The dog: strafe at 4 to 6 m, lunge inside 3.5 m (crouch 18 ticks as the telegraph, leap 12, bite 6, recover 30); reaction 12 ticks. The large injured dog: idle and stunned only, never attacks. | Arena 1 and Arena 3 (cues 39, 79). | `dog_lunges_after_strafing`, `injured_dog_never_attacks` |
| M4.4 | A `Pack` hands out attack tokens before the enemies tick: one attacker at a time, and for Arena 2's tutorial a volley that gives all three dogs a token on the same tick and starts the slow-down. | Vagrancy has no coordinator (trees review); Arena 2 needs three dogs attacking together (cue 68). | `pack_gives_one_token_normally`, `tutorial_volley_attacks_together` |
| M5.1 | Web Audio sounds synthesized from `data/audio.json` (CS-Q16); a click-to-start screen, since every browser holds audio until a click. | No licences; tunable in data; sound review. | Browser check: the start screen appears, then the sandbox |
| M5.2 | Rumble through `vibrationActuator.playEffect("dual-rumble")` where it exists; silence otherwise. No WebHID in the MVP (CS-Q25). | Rumble works in Chrome, Edge and Safari, not Firefox (controllers review). | Browser check with a scripted pad records `playEffect` calls |
| M6 | Feel rounds: scripted fights run through the harness; playtester subagents (CS-Q15) read the output and say how it looks and feels; one change a round; each round a journal entry with before and after; repeat until a playtester says it feels great. Then Sam plays it. | Order of work step 5; Agents as Playtesters; CS-16. | Journal entries; the numbers changed live in `data/` |
| M6.1 | Journal animations built on the SnapClass step player (steps of `{go, work, t, c, hl}` over an SVG, with Play, Pause, Step, Restart, a key and every step as text), fed by the harness's `--json` output: combo flow, animation states and transitions, each enemy's tree with the active node lit, stun, launch and air states over time, hit-stop and hit reaction timelines, and the tutorial sequence. | CS-15: every animation draws real data. | A journal check that each animation's data names the harness command that made it |
| M7.1 | The backdrop per CS-Q24: four rings and a sky dome (25 to 150 m park and trees; 150 m to 2.1 km lake with a mirrored skyline; 2.1 to 5 km the city; 8 to 15 km hills), haze from 150 m, Chumworld Arena's two depth ranges, about 12 draws. The arena edge drops away so the city lies below the eye. | Sam's vistas (cues 91-115); vistas review; Gas Works Kite Hill's real sightline (1.9 km of water, towers 4.6 km away). | `backdrop_stays_within_budget`; flicker scan; frame probe |
| M7.2 | Tutorials in the core: slow to 0.2 speed over 10 ticks, hold 30, freeze, and wait; only the named input resumes time and performs the move; one line of text after 3 wrong presses or 600 ticks; each prompt shown once and passed for good; button names for the device last used. | CS-12; tutorials review; Andersen et al. (CHI 2012). | `prompt_resumes_only_on_its_input_and_performs_it`, `passed_prompt_never_returns` |
| M7.3 | Arena 1 (one young dog, CS-Q13), Arena 2 (three dogs, the Whirlwind prompt), Arena 3 (the injured large dog; the launcher combo kills it); a checkpoint just before each fight; cutscenes skipped by a 45-tick hold; checkpoints hold within a visit (CS-Q19). | The MVP (cues 51-81, 98-101). | `bot_clears_each_arena`; `checkpoint_before_each_fight`; `every_cutscene_skips` |
| M8 | Install Playwright (R1.4), run `./scripts/check.sh`, publish with `scripts/publish.sh` to `chum-and-slash` (CS-Q5, CS-Q10), run the browser check against the live address, and write a *live* entry. | Sam's permission: deploy when the checks pass (CS-19). | `check.sh`; the browser check on the live address |

## Starting numbers

All are the reviewers' estimates, not frame data from Devil May Cry 5 or Dragon's Dogma 2, which none of the reviews could find. They live in `data/` and the feel rounds change them.

| Value | Start | From |
| --- | --- | --- |
| Hit-stop light / heavy / launcher / cap | 4 / 7 / 10 / 12 ticks | combat review |
| Light startup, strike, follow-through, recovery | 5, 3, 6, 10 ticks | animation review |
| Heavy startup, strike, follow-through, recovery | 14, 4, 10, 16 ticks | animation review |
| Input buffer; string reset | 8 ticks; 35 ticks | combat review |
| Recovery after a cap | 24 ticks | combat review |
| Forward motion light / heavy | 0.5 m / 1.0 m | combat review |
| Push-back light / heavy | 0.3 m / 0.8 m | combat review |
| Stun window | 90 ticks, refreshed by each hit | combat review; CS-Q8 |
| Launch height; air float | 3.0 m; gravity × 0.25 for 20 ticks | combat review |
| Dog lunge | crouch 18, leap 12, bite 6, recover 30 ticks | trees review |
| Tutorial slow-down | 0.2 speed over 10 ticks, hold 30, then freeze | tutorials review |
| Rumble light / heavy / launcher | 0.0/0.35 4 ticks; 0.55/0.45 7; 1.0/0.6 14 | controllers review |

## Changes to Builder's defaults

- D21: subagents named in `BRIEF.md` (seven reviewers; playtesters per CS-Q15).
- Nothing else. D3 holds: every number above is data.

## Open questions for the person

The journal holds them as answer chips: CS-Q1 to CS-Q22 on the "How I read this" page, and CS-Q23 to CS-Q25 on the Plan page. The plan builds on each default until answered.

## Status

- [x] Read by the person on: 2026-11-18 ("build through to publishing without stopping"; DECISIONS.md)
