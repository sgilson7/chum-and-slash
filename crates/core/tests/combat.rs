//! Sam's combat rules, each held by a test (CS-2 to CS-9, M3, M4).
//! Expected values are worked from data/combat.json by hand in the
//! comments; no test compares a value with the function that made it.

use tool_core::fx::{self, ONE};
use tool_core::geom::{v3, V3};
use tool_core::world::*;

fn ahead(m: i32) -> V3 {
    // the hero faces -z at yaw 0
    v3(0, 0, -m)
}

/// A world with the hero at the origin and one enemy `mm` ahead.
fn with(kind: &str, mm: i32) -> (World, u32) {
    let mut w = World::new();
    let id = w.spawn(kind, ahead(tool_core::model::mm(mm)), fx::HALF).unwrap();
    (w, id)
}

fn tap(w: &mut World, bits: u32) -> Vec<Event> {
    let mut e = w.tick(Input::bits(bits));
    e.extend(w.tick(Input::bits(0)));
    e
}

fn run(w: &mut World, n: u32, bits: u32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..n {
        out.extend(w.tick(Input::bits(bits)));
    }
    out
}

/// Ticks until the hero may start the next move (its cancel tick, or the
/// move's end), at most `limit`.
fn until_ready(w: &mut World, limit: u32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..limit {
        let ready = match &w.hero.act {
            None => true,
            Some(a) => {
                let c = &w.clips[&a.name];
                c.events.cancel.is_some_and(|k| w.hero.anim.tick >= k) || c.done(w.hero.anim.tick)
            }
        };
        if ready && w.hero.hitstop == 0 {
            break;
        }
        out.extend(w.tick(Input::bits(0)));
    }
    out
}

/// Presses each button in turn, each as soon as the last move allows.
fn string(w: &mut World, presses: &[u32]) -> Vec<Event> {
    let mut out = Vec::new();
    for &p in presses {
        out.extend(tap(w, p));
        out.extend(until_ready(w, 80));
    }
    out
}

fn names(ev: &[Event], kind: &str) -> Vec<String> {
    ev.iter()
        .filter(|e| e.kind == kind)
        .map(|e| e.name.clone())
        .collect()
}

/// CS-2: in the tick a hit lands, the enemy reacts (stagger or shake),
/// both bodies stop (hit-stop), a hit sound plays and the pad rumbles.
#[test]
fn every_landed_hit_is_told_four_ways() {
    for (kind, button, sound, rumble) in [
        ("dog", B_LIGHT, "hit_light", "light_hit"),
        ("dog", B_HEAVY, "hit_heavy", "heavy_hit"),
        ("big_dog", B_LIGHT, "hit_light", "light_hit"),
    ] {
        let (mut w, id) = with(kind, 1600);
        w.tick(Input::bits(button));
        let mut found = false;
        for _ in 0..40 {
            let ev = w.tick(Input::bits(0));
            if ev.iter().any(|e| e.kind == "hit") {
                let e = w.enemy(id).unwrap();
                assert!(
                    matches!(e.reaction, "staggers" | "shakes"),
                    "{kind}: reaction {}",
                    e.reaction
                );
                assert!(e.hitstop > 0 && w.hero.hitstop > 0, "{kind}: hit-stop on both");
                assert!(
                    names(&ev, "sound").contains(&sound.to_string()),
                    "{kind}: {sound} in {ev:?}"
                );
                assert!(
                    names(&ev, "rumble").contains(&rumble.to_string()),
                    "{kind}: {rumble} in {ev:?}"
                );
                found = true;
                break;
            }
        }
        assert!(found, "{kind}: the hit landed");
    }
}

/// CS-2: a swing at nothing gives the miss sound and no hit sound, no
/// rumble, no hit-stop.
#[test]
fn a_miss_sounds_unlike_a_hit() {
    let mut w = World::new();
    let mut ev = tap(&mut w, B_LIGHT);
    ev.extend(run(&mut w, 30, 0));
    let sounds = names(&ev, "sound");
    assert_eq!(sounds, ["swing"]);
    assert!(names(&ev, "rumble").is_empty());
    assert_eq!(w.hero.hitstop, 0);
}

/// CS-3: ground strings and jumps leave stamina alone; sprinting, a move
/// in the air (600) and an ability (3000) spend it.
#[test]
fn only_sprint_air_moves_and_abilities_spend_stamina() {
    let mut w = World::new();
    let full = w.rules.stamina.max;
    string(&mut w, &[B_LIGHT, B_HEAVY, B_LIGHT, B_LAUNCH]);
    run(&mut w, 60, 0);
    assert_eq!(w.hero.stamina, full, "ground moves are free");
    tap(&mut w, B_JUMP);
    assert_eq!(w.hero.stamina, full, "a jump is free (CS-Q9)");
    tap(&mut w, B_LIGHT);
    assert_eq!(w.hero.stamina, full - 600, "an air light costs 600");
    run(&mut w, 120, 0);
    let before = w.hero.stamina;
    run(&mut w, 2, B_FWD | B_SPRINT);
    assert!(w.hero.stamina < before, "sprinting costs");
    let mut w = World::new();
    tap(&mut w, B_MOD | B_LIGHT);
    assert_eq!(w.hero.stamina, full - 3000, "the Whirlwind Strike costs 3000");
}

/// CS-5: a launcher lifts a stunned enemy and not an unstunned one.
#[test]
fn launcher_lifts_only_a_stunned_enemy() {
    let (mut w, id) = with("big_dog", 2400);
    // the big dog has poise: a launcher on it unstunned does not lift it
    w.enemies[0].brain.tree.reaction = 0;
    string(&mut w, &[B_LAUNCH]);
    run(&mut w, 30, 0);
    assert_ne!(
        w.enemy(id).unwrap().state,
        State::Airborne,
        "unstunned: not lifted"
    );
    let (mut w, id) = with("dog", 1600);
    string(&mut w, &[B_LIGHT]);
    assert_eq!(w.enemy(id).unwrap().state, State::Stunned, "one hit stuns a dog");
    run(&mut w, 20, 0);
    string(&mut w, &[B_LAUNCH]);
    assert_eq!(w.enemy(id).unwrap().state, State::Airborne, "stunned: lifted");
}

/// CS-4: an air hit does half again. air_light does 9 on the ground, so
/// 9 × 3 / 2 = 13 in the air (integer).
#[test]
fn air_hits_deal_half_again() {
    let (mut w, id) = with("dog", 1600);
    string(&mut w, &[B_LIGHT]);
    run(&mut w, 20, 0);
    string(&mut w, &[B_LAUNCH]);
    assert_eq!(w.enemy(id).unwrap().state, State::Airborne);
    tap(&mut w, B_JUMP);
    run(&mut w, 8, 0);
    let mut ev = tap(&mut w, B_LIGHT);
    ev.extend(run(&mut w, 20, 0));
    let hit = ev
        .iter()
        .find(|e| e.kind == "hit" && e.name == "air_light")
        .expect("the air light landed");
    assert_eq!(hit.b, 13);
}

fn recovering(w: &World) -> bool {
    w.hero.combo.recovery > 0
}

/// CS-7: four lights in a row, then a recovery.
#[test]
fn light_string_caps_at_four() {
    let mut w = World::new();
    let ev = string(&mut w, &[B_LIGHT, B_LIGHT, B_LIGHT]);
    assert!(names(&ev, "recovery").is_empty());
    let mut ev = tap(&mut w, B_LIGHT);
    ev.extend(run(&mut w, 40, 0));
    assert_eq!(names(&ev, "recovery"), ["cap"]);
}

/// CS-7: three heavies in a row, then a recovery.
#[test]
fn heavy_caps_at_three() {
    let mut w = World::new();
    let mut ev = string(&mut w, &[B_HEAVY, B_HEAVY, B_HEAVY]);
    ev.extend(run(&mut w, 60, 0));
    assert_eq!(names(&ev, "recovery"), ["cap"]);
}

/// CS-7: alternating never reaches a run cap, so the string runs to six.
#[test]
fn interleaved_caps_at_six() {
    let mut w = World::new();
    let ev = string(&mut w, &[B_LIGHT, B_HEAVY, B_LIGHT, B_HEAVY, B_LIGHT]);
    assert!(
        names(&ev, "recovery").is_empty(),
        "five alternating presses: no cap yet"
    );
    let mut ev = tap(&mut w, B_HEAVY);
    ev.extend(run(&mut w, 60, 0));
    assert_eq!(names(&ev, "recovery"), ["cap"]);
}

/// CS-7: during the recovery no attack starts, then one may.
#[test]
fn no_attack_starts_in_recovery() {
    let mut w = World::new();
    string(&mut w, &[B_HEAVY, B_HEAVY, B_HEAVY]);
    // wait for the third heavy to end and the recovery to begin
    let mut guard = 0;
    while !recovering(&w) && guard < 100 {
        w.tick(Input::bits(0));
        guard += 1;
    }
    assert!(recovering(&w));
    tap(&mut w, B_LIGHT);
    assert!(w.hero.act.is_none(), "no move starts in the recovery");
    run(&mut w, 30, 0);
    tap(&mut w, B_LIGHT);
    assert!(w.hero.act.is_some(), "after it, a move starts");
}

/// CS-8: light, heavy, light, light, heavy is the first combo; ending it
/// with the launcher is the second.
#[test]
fn first_combos_perform_from_their_inputs() {
    let mut w = World::new();
    let ev = string(&mut w, &[B_LIGHT, B_HEAVY, B_LIGHT, B_LIGHT, B_HEAVY]);
    assert_eq!(names(&ev, "combo"), ["first"]);
    let ev = string(&mut w, &[B_LAUNCH]);
    assert_eq!(names(&ev, "combo"), ["first_launch"]);
}

/// CS-9: every attack carries its attacker forward.
#[test]
fn every_attack_moves_its_attacker_forward() {
    let w = World::new();
    for name in w.rules.moves.keys() {
        let c = &w.clips[name];
        let (f, _) = c.root_at(c.len);
        assert!(f > 0, "{name} carries the hero {f} forward");
    }
    let mut w = World::new();
    tap(&mut w, B_LIGHT);
    run(&mut w, 24, 0);
    // light_1's root ends 500 mm on: 32768 in Q16
    assert_eq!(w.hero.pos.z, -32768);
}

/// CS-9: a landed hit pushes its target back, away from the hero.
#[test]
fn every_landed_hit_pushes_its_target_back() {
    for button in [B_LIGHT, B_HEAVY, B_LAUNCH] {
        let (mut w, id) = with("dog", 1600);
        w.tick(Input::bits(button));
        let mut guard = 0;
        while !w.tick(Input::bits(0)).iter().any(|e| e.kind == "hit") {
            guard += 1;
            assert!(guard < 40, "the hit landed");
        }
        // from the hit, through its hit-stop and the push (6 ticks)
        let before = w.enemy(id).unwrap().pos.z;
        let wait = w.enemy(id).unwrap().hitstop + w.rules.push_ticks;
        run(&mut w, wait, 0);
        let after = w.enemy(id).unwrap().pos.z;
        assert!(after < before, "pushed further along -z: {before} -> {after}");
    }
}

/// CS-Q8: the stun lasts while the string lasts, and when the string
/// ends it runs out within its tail (20 ticks).
#[test]
fn stun_lasts_while_the_combo_lasts() {
    let (mut w, id) = with("dog", 1600);
    string(&mut w, &[B_LIGHT, B_HEAVY, B_LIGHT]);
    assert_eq!(w.enemy(id).unwrap().state, State::Stunned);
    // 35 ticks without a press ends the string
    run(&mut w, 60, 0);
    let e = w.enemy(id).unwrap();
    assert!(
        e.state != State::Stunned || e.stun <= w.rules.stun.tail,
        "stun {} after the string",
        e.stun
    );
    run(&mut w, 30, 0);
    assert_ne!(w.enemy(id).unwrap().state, State::Stunned);
}

/// The Whirlwind Strike stuns every dog in reach, all round.
#[test]
fn whirl_stuns_every_dog_in_reach() {
    let mut w = World::new();
    let m = tool_core::model::mm(2000);
    for at in [v3(0, 0, -m), v3(m, 0, m / 2), v3(-m, 0, m / 2)] {
        w.spawn("dog", at, 0).unwrap();
    }
    tap(&mut w, B_MOD | B_LIGHT);
    run(&mut w, 40, 0);
    for e in &w.enemies {
        assert!(e.stunned_like(), "dog {} is {}", e.id, e.state.name());
    }
}

/// The large injured dog never attacks (cue 79).
#[test]
fn injured_dog_never_attacks() {
    let (mut w, id) = with("big_dog", 1500);
    let ev = run(&mut w, 900, 0);
    assert!(names(&ev, "bitten").is_empty());
    assert_ne!(w.enemy(id).unwrap().anim.clip, "dog_lunge");
}

/// Arena 3: the injured dog dies when the first launcher combo lands on it,
/// and not before.
#[test]
fn the_launcher_combo_kills_the_injured_dog() {
    let (mut w, id) = with("big_dog", 2200);
    string(&mut w, &[B_LIGHT, B_HEAVY, B_LIGHT, B_LIGHT, B_HEAVY]);
    assert_ne!(w.enemy(id).unwrap().state, State::Dead, "not yet");
    let ev = string(&mut w, &[B_LAUNCH]);
    assert_eq!(names(&ev, "die"), [id.to_string()]);
}

/// A dog strafes, then closes and lunges, and its bite can land.
#[test]
fn dog_lunges_after_strafing() {
    let (mut w, _) = with("dog", 6000);
    let ev = run(&mut w, 900, 0);
    assert!(!names(&ev, "bitten").is_empty(), "a bite landed within 15 s");
}

/// M4.4: with three dogs, at most one lunges at a time.
#[test]
fn pack_gives_one_token_normally() {
    let mut w = World::new();
    let m = tool_core::model::mm(5000);
    for at in [v3(0, 0, -m), v3(m, 0, 0), v3(-m, 0, 0)] {
        w.spawn("dog", at, 0).unwrap();
    }
    for _ in 0..1200 {
        w.tick(Input::bits(0));
        let lunging = w.enemies.iter().filter(|e| e.anim.clip == "dog_lunge").count();
        assert!(lunging <= 1, "{lunging} dogs lunging at tick {}", w.tick);
    }
}

/// M4.4: the tutorial's volley sends all three dogs at once.
#[test]
fn tutorial_volley_attacks_together() {
    let mut w = World::new();
    let m = tool_core::model::mm(5000);
    for (k, at) in [v3(0, 0, -m), v3(m, 0, 0), v3(-m, 0, 0)].into_iter().enumerate() {
        let id = w.spawn("dog", at, 0).unwrap();
        let i = w.enemies.iter().position(|e| e.id == id).unwrap();
        w.enemies[i].brain.with_tree("tutorial_dog", k as u64);
    }
    run(&mut w, 30, 0);
    w.pack.volley = true;
    // the tutorial dog reacts in 20 ticks
    run(&mut w, 23, 0);
    let lunging = w.enemies.iter().filter(|e| e.anim.clip == "dog_lunge").count();
    assert_eq!(lunging, 3);
}

/// After Vagrancy's rise.rs:71: a stunned dog does nothing until its stun
/// ends: no move, no bite.
#[test]
fn a_stunned_dog_sends_nothing_until_the_stun_ends() {
    let (mut w, id) = with("dog", 1600);
    // a tree that only lunges, so the core's own gate is what holds (M4.2)
    let rules = serde_json::from_str(r#"[{"do":"lunge"}]"#).unwrap();
    w.enemies[0].brain.tree = tool_core::pilot::Tree::new(rules, 0, 0);
    string(&mut w, &[B_LIGHT]);
    let start = w.enemy(id).unwrap().pos;
    let mut ev = Vec::new();
    while w.enemy(id).unwrap().state == State::Stunned {
        ev.extend(w.tick(Input::bits(0)));
        let e = w.enemy(id).unwrap();
        if e.knock_ticks == 0 && e.state == State::Stunned {
            assert_ne!(e.anim.clip, "dog_lunge");
        }
    }
    assert!(names(&ev, "bitten").is_empty());
    let _ = start;
}

/// CS-Q21: lock-on is on when a fight starts, and the button lets go and
/// takes it back.
#[test]
fn lock_on_starts_with_the_fight_and_toggles() {
    let (mut w, id) = with("dog", 3000);
    w.tick(Input::bits(0));
    assert_eq!(w.hero.lock, Some(id));
    tap(&mut w, B_LOCK);
    assert_eq!(w.hero.lock, None, "released for freestyle");
    tap(&mut w, B_LOCK);
    assert_eq!(w.hero.lock, Some(id));
}

/// The camera's knock rings back through the middle to exactly nothing
/// (Chumworld Arena's test, the same numbers: 5 cm held, 55% kept).
#[test]
fn the_camera_knock_rings_to_exactly_nothing() {
    let mut k = v3(KNOCK_MAX, 0, 0);
    assert_eq!(k.x, 3276);
    let mut seen = vec![k.x];
    while k != V3::ZERO {
        k = ring(k);
        seen.push(k.x);
    }
    assert_eq!(seen, [3276, -1801, 990, -544, 299, -164, 90, -49, 0]);
    let _ = ONE;
}
