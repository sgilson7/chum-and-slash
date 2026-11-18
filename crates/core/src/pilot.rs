//! Enemy behaviour trees, ported from Vagrancy's `crates/pilot` (MIT, Sam
//! Gilson; CS-10, CS-Q23, M4).
//!
//! As in Vagrancy, a tree pilot is data: an ordered list of rules, each a
//! set of conditions and a move (`data/pilots.json`). The first rule whose
//! conditions hold starts its move, and the pilot commits to it until it
//! ends; a rule marked `interrupt` may break in. It decides on the world as
//! it was `reaction` ticks ago. Ported: `Cond`/`Rule` (lib.rs:831-889),
//! `Tree::new`/`holds`/`pick` (1062-1133), `trace`/`input` (1190-1267), node
//! numbering (view.rs:242-293) and xorshift64* seeded `0x7EE ^ salt`
//! (sim/src/rng.rs). Changed for this game (trees review): conditions read
//! a dog's world (distance on the ground plane, stunned, airborne, the
//! pack's token) instead of a sword fight's; the world clone kept per tick
//! is a small `Seen`; a move is a dog's intent, not a 16-bit key record.
//!
//! Every tree starts with stunned, then airborne, then idle (CS-6, M4.2):
//! the core also refuses a stunned or airborne dog any move (world.rs).

use crate::clip::Clips;
use crate::fx::{self, ONE};
use crate::geom::{facing, v3, V3};
use crate::model::{self, Model};
use crate::world::{yaw_of, Enemy, Rules, State, World};
use serde::Deserialize;
use std::collections::{BTreeMap, VecDeque};

pub const PILOTS_JSON: &str = include_str!("../../../data/pilots.json");

/// xorshift64*, as Vagrancy's `sim::rng::Rng` (ported from Floodline).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XorShift {
    state: u64,
}

impl XorShift {
    pub fn new(seed: u64) -> XorShift {
        XorShift {
            state: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed },
        }
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % n as u64) as u32
    }
}

/// One condition on what the pilot sees. Distances in cm, as Vagrancy's.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Cond {
    /// The hero is farther than this, cm, on the ground plane.
    GapAbove(i32),
    GapBelow(i32),
    MeStunned,
    MeAirborne,
    MeDown,
    /// The pack has given me the attack token (M4.4).
    HasToken,
    /// The tutorial's volley: every dog attacks on the same tick (Arena 2).
    Volley,
    /// The hero is in the air.
    OppAirborne,
    /// A seeded chance, in percent, drawn when the rule is considered.
    Chance(u32),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Rule {
    #[serde(default, rename = "if")]
    pub when: Vec<Cond>,
    #[serde(rename = "do")]
    pub act: String,
    #[serde(default)]
    pub interrupt: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TreeSpec {
    pub reaction: u32,
    pub rules: Vec<Rule>,
}

#[derive(Clone, Debug, Deserialize)]
struct PilotsFile {
    trees: BTreeMap<String, TreeSpec>,
}

pub fn trees() -> BTreeMap<String, TreeSpec> {
    serde_json::from_str::<PilotsFile>(PILOTS_JSON)
        .map(|f| f.trees)
        .unwrap_or_default()
}

/// What the pilot saw on one tick (Vagrancy kept a whole world clone).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Seen {
    pub gap_cm: i32,
    pub stunned: bool,
    pub airborne: bool,
    pub down: bool,
    pub token: bool,
    pub volley: bool,
    pub opp_airborne: bool,
}

/// The ids a rule's nodes took, depth first from 0 at the root
/// (Vagrancy's `view::RuleIds`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuleIds {
    pub seq: Option<u16>,
    pub conds: Vec<u16>,
    pub act: u16,
}

/// Numbers the nodes as Vagrancy's `tree_nodes` does: the root selector is
/// 0; each rule with conditions is a sequence, then its conditions, then
/// its move.
pub fn node_ids(rules: &[Rule]) -> Vec<RuleIds> {
    let mut next = 1u16;
    let mut take = || {
        let n = next;
        next += 1;
        n
    };
    rules
        .iter()
        .map(|r| {
            let seq = if r.when.is_empty() { None } else { Some(take()) };
            let conds = r.when.iter().map(|_| take()).collect();
            RuleIds {
                seq,
                conds,
                act: take(),
            }
        })
        .collect()
}

/// Which nodes are lit: the running branch, and the conditions checked
/// this tick that held or failed (Vagrancy's `view::Trace`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Trace {
    pub active: Vec<u16>,
    pub held: Vec<u16>,
    pub failed: Vec<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tree {
    pub rules: Vec<Rule>,
    pub reaction: u32,
    seen: VecDeque<Seen>,
    rng: XorShift,
    pub current: Option<(String, u32)>,
    pub running: Option<usize>,
    checked: Vec<(u16, bool)>,
    ids: Vec<RuleIds>,
    /// how long the current move lasts, drawn when it starts
    pub length: u32,
}

impl Tree {
    pub fn new(rules: Vec<Rule>, reaction: u32, salt: u64) -> Tree {
        let ids = node_ids(&rules);
        Tree {
            rules,
            reaction,
            seen: VecDeque::new(),
            rng: XorShift::new(0x7EE ^ salt),
            current: None,
            running: None,
            checked: Vec::new(),
            ids,
            length: 0,
        }
    }

    fn holds(&mut self, c: &Cond, s: &Seen) -> bool {
        match c {
            Cond::GapAbove(cm) => s.gap_cm > *cm,
            Cond::GapBelow(cm) => s.gap_cm < *cm,
            Cond::MeStunned => s.stunned,
            Cond::MeAirborne => s.airborne,
            Cond::MeDown => s.down,
            Cond::HasToken => s.token,
            Cond::Volley => s.volley,
            Cond::OppAirborne => s.opp_airborne,
            Cond::Chance(pct) => self.rng.below(100) < *pct,
        }
    }

    /// The first rule whose conditions all hold, checked in order and
    /// stopping at a rule's first failing condition (Vagrancy's `pick`).
    fn pick(&mut self, s: &Seen, interrupts_only: bool) -> Option<(usize, String)> {
        self.checked.clear();
        for i in 0..self.rules.len() {
            let r = self.rules[i].clone();
            if interrupts_only && !r.interrupt {
                continue;
            }
            let mut all = true;
            for (j, c) in r.when.iter().enumerate() {
                let ok = self.holds(c, s);
                self.checked.push((self.ids[i].conds[j], ok));
                if !ok {
                    all = false;
                    break;
                }
            }
            if all {
                return Some((i, r.act));
            }
        }
        None
    }

    pub fn trace(&self) -> Trace {
        let mut t = Trace::default();
        for &(id, ok) in &self.checked {
            if ok {
                t.held.push(id)
            } else {
                t.failed.push(id)
            }
        }
        if let Some(r) = self.running {
            let ids = &self.ids[r];
            t.active.push(0);
            if let Some(sq) = ids.seq {
                t.active.push(sq);
                t.active.extend(ids.conds.iter().copied());
            }
            t.active.push(ids.act);
        }
        t
    }

    /// One tick: remember what was seen, decide on what was seen
    /// `reaction` ticks ago, and return the move and its tick (Vagrancy's
    /// `input`, giving a move instead of keys).
    pub fn input(&mut self, now: Seen) -> Option<(String, u32)> {
        self.seen.push_back(now);
        while self.seen.len() as u32 > self.reaction + 1 {
            self.seen.pop_front();
        }
        let seen = *self.seen.front().unwrap_or(&now);
        let interrupt = if self.current.is_some() {
            self.pick(&seen, true)
        } else {
            None
        };
        if let Some((rule, name)) = interrupt {
            if self.current.as_ref().map(|(n, _)| n != &name).unwrap_or(true) {
                self.length = self.draw_length(&name);
                self.current = Some((name, 0));
                self.running = Some(rule);
            }
        }
        if self.current.is_none() {
            match self.pick(&seen, false) {
                Some((rule, n)) => {
                    self.length = self.draw_length(&n);
                    self.current = Some((n, 0));
                    self.running = Some(rule);
                }
                None => {
                    self.running = None;
                }
            }
        }
        let (name, t) = self.current.clone()?;
        if t >= self.length {
            self.current = None;
            self.running = None;
            return None;
        }
        self.current = Some((name.clone(), t + 1));
        Some((name, t))
    }

    /// A move's length in ticks. A strafe runs 60 to 120 ticks, drawn from
    /// the tree's own generator (trees review: "a second or two").
    fn draw_length(&mut self, name: &str) -> u32 {
        match name {
            "strafe_left" | "strafe_right" => 60 + self.rng.below(61),
            "lunge" => LUNGE_TICKS,
            "approach" | "back_off" => 20,
            "idle" => 30,
            _ => 1,
        }
    }

    pub fn end(&mut self) {
        self.current = None;
        self.running = None;
    }
}

/// The lunge's length: crouch 18, leap 12, bite 6, recover 30.
pub const LUNGE_TICKS: u32 = 66;

/// What a dog does this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    Hold,
    Toward,
    Away,
    CircleLeft,
    CircleRight,
    Lunge,
}

/// An enemy's brain: its tree, the node it is on (for the harness and the
/// journal's lit trees), and the pack's flags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brain {
    pub tree: Tree,
    pub node: String,
    pub token: bool,
    pub volley: bool,
    /// ticks before this dog may be given the token again: it strafes, and
    /// only every once in a while closes to bite (cue 39; round 1, R2.1)
    pub cooldown: u32,
}

/// The rest between a dog's lunges, in ticks (1 s).
pub const LUNGE_COOLDOWN: u32 = 60;

impl Brain {
    pub fn for_kind(kind: &str) -> Brain {
        let all = trees();
        let spec = all
            .get(kind)
            .or_else(|| all.get("dog"))
            .cloned()
            .unwrap_or(TreeSpec {
                reaction: 12,
                rules: Vec::new(),
            });
        let salt = kind
            .bytes()
            .fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64));
        Brain {
            tree: Tree::new(spec.rules, spec.reaction, salt),
            node: "idle".into(),
            token: false,
            volley: false,
            cooldown: 0,
        }
    }

    pub fn with_tree(&mut self, name: &str, salt: u64) {
        if let Some(spec) = trees().get(name) {
            self.tree = Tree::new(spec.rules.clone(), spec.reaction, salt);
        }
    }
}

/// The pack hands out attack tokens before the enemies move (M4.4): one
/// dog lunges at a time, unless the tutorial's volley gives every dog a
/// token on the same tick.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pack {
    pub holder: Option<u32>,
    /// set by the tutorial for one tick: every idle dog attacks now
    pub volley: bool,
    /// the dogs given the volley
    pub volleyed: Vec<u32>,
}

impl Pack {
    pub fn step(&mut self, enemies: &[Enemy], rules: &Rules) {
        let free = |e: &Enemy| e.state == State::Idle && rules.enemies[&e.kind].attacks;
        let rested = |e: &Enemy| free(e) && e.brain.cooldown == 0;
        if let Some(h) = self.holder {
            let keep = enemies
                .iter()
                .any(|e| e.id == h && free(e) && (e.brain.token || e.anim.clip == "dog_lunge"));
            if !keep {
                self.holder = None;
            }
        }
        if self.volley {
            self.volleyed = enemies.iter().filter(|e| free(e)).map(|e| e.id).collect();
            self.volley = false;
        }
        if self.holder.is_none() {
            // the dog that has waited longest is the one nearest in id order
            self.holder = enemies.iter().filter(|e| rested(e)).map(|e| e.id).next();
        }
    }
}

/// What a dog sees now (it decides on what it saw `reaction` ticks ago).
pub fn seen(w: &World, i: usize) -> Seen {
    let e = &w.enemies[i];
    let gap = e.pos.sub(w.hero.pos).len_xz();
    Seen {
        gap_cm: fx::mul(gap, fx::int(100)) / ONE,
        stunned: e.state == State::Stunned,
        airborne: e.state == State::Airborne,
        down: e.state == State::Down,
        token: w.pack.holder == Some(e.id),
        volley: w.pack.volleyed.contains(&e.id),
        opp_airborne: !w.hero.grounded,
    }
}

/// The tree's move for this tick, as an intent; also records the node.
pub fn think(w: &mut World, i: usize) -> Intent {
    let s = seen(w, i);
    let e = &mut w.enemies[i];
    // a dog already lunging finishes its lunge
    if e.anim.clip == "dog_lunge" && e.anim.tick < LUNGE_TICKS {
        e.brain.node = "lunge".into();
        return Intent::Lunge;
    }
    let mut mv = e.brain.tree.input(s);
    // a dog decides on what it saw `reaction` ticks ago, but it lunges only
    // on a token it holds now (or the tutorial's volley): two dogs never
    // lunge at once outside the volley (M4.4)
    let holds_now = w.pack.holder == Some(e.id) || w.pack.volleyed.contains(&e.id);
    if mv.as_ref().is_some_and(|(n, _)| n == "lunge") && !holds_now {
        e.brain.tree.end();
        mv = None;
    }
    if mv.as_ref().is_some_and(|(n, _)| n == "lunge") {
        w.pack.volleyed.retain(|id| *id != e.id);
    }
    e.brain.node = mv
        .as_ref()
        .map(|(n, _)| n.clone())
        .unwrap_or_else(|| "none".into());
    match mv.as_ref().map(|(n, _)| n.as_str()) {
        Some("approach") => Intent::Toward,
        Some("back_off") => Intent::Away,
        Some("strafe_left") => Intent::CircleLeft,
        Some("strafe_right") => Intent::CircleRight,
        Some("lunge") => Intent::Lunge,
        _ => Intent::Hold,
    }
}

/// A dog's walk speed (3 m/s) and the ring it strafes on (5 m).
pub const TROT_MM_S: i32 = 3000;
pub const RING_MM: i32 = 5000;

/// Carries out an intent: walks, circles, or lunges with the clip's root
/// motion.
pub fn act(e: &mut Enemy, intent: Intent, hero: V3, clips: &Clips, m: &Model) {
    let to = hero.sub(e.pos);
    let face = yaw_of(to);
    let trot = crate::world::per_tick(TROT_MM_S);
    let dir = v3(to.x, 0, to.z).norm();
    let side = v3(-dir.z, 0, dir.x);
    let gap = to.len_xz();
    let ring = model::mm(RING_MM);
    let mut want_clip = "dog_strafe";
    match intent {
        Intent::Hold => {
            want_clip = "dog_idle";
            e.yaw = turn_to(e.yaw, face);
        }
        Intent::Toward => {
            e.pos = e.pos.add(dir.scale(trot));
            e.yaw = turn_to(e.yaw, face);
        }
        Intent::Away => {
            e.pos = e.pos.sub(dir.scale(trot));
            e.yaw = turn_to(e.yaw, face);
        }
        Intent::CircleLeft | Intent::CircleRight => {
            let s = if intent == Intent::CircleLeft {
                side
            } else {
                side.neg()
            };
            // round the hero, drifting back toward the ring
            let radial = fx::div(gap - ring, ring).clamp(-ONE / 2, ONE / 2);
            let step = s.scale(trot).add(dir.scale(fx::mul(radial, trot)));
            e.pos = e.pos.add(step);
            e.yaw = turn_to(e.yaw, face);
        }
        Intent::Lunge => {
            if e.anim.clip != "dog_lunge" {
                e.yaw = face;
                e.bit = false;
                e.anim.play(clips, m, "dog_lunge", 6);
                e.brain.token = false;
                return;
            }
            // aim while crouched, then commit to the leap
            if e.anim.tick < 18 {
                e.yaw = turn_to(e.yaw, face);
            }
            let (f, _) = clips["dog_lunge"].root_step(e.anim.tick);
            e.pos = e.pos.add(facing(e.yaw, 0).scale(f));
            if clips["dog_lunge"].done(e.anim.tick) {
                e.brain.tree.end();
                e.brain.cooldown = LUNGE_COOLDOWN;
                e.anim.play(clips, m, "dog_idle", 12);
            }
            return;
        }
    }
    if e.anim.clip != want_clip {
        e.anim.play(clips, m, want_clip, 12);
    }
}

fn turn_to(from: i32, to: i32) -> i32 {
    let d = fx::wrap(to - from).clamp(-fx::deg(12), fx::deg(12));
    fx::wrap(from + d)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parity with Vagrancy's generator: the same seed (`0x7EE ^ salt`)
    /// draws the same numbers. Expected values were printed by Vagrancy's
    /// own `sim::rng::Rng` (scratch crate, 2026-11-18).
    #[test]
    fn parity_with_vagrancy_rng() {
        for (salt, want) in [
            (0u64, [93, 23, 51, 68, 69, 7]),
            (1, [24, 6, 68, 97, 1, 86]),
            (2, [31, 25, 76, 43, 34, 6]),
        ] {
            let mut r = XorShift::new(0x7EE ^ salt);
            let got: Vec<u32> = (0..6).map(|_| r.below(100)).collect();
            assert_eq!(got, want, "salt {salt}");
        }
    }

    fn rules(json: &str) -> Vec<Rule> {
        serde_json::from_str(json).unwrap()
    }

    /// Vagrancy's numbering: root 0; a rule with conditions takes a
    /// sequence id, then its conditions, then its move.
    #[test]
    fn nodes_are_numbered_depth_first() {
        let r = rules(
            r#"[{"if":["me_stunned"],"do":"reel"},{"do":"idle"},{"if":[{"gap_below":300},"has_token"],"do":"lunge"}]"#,
        );
        let ids = node_ids(&r);
        assert_eq!(
            ids[0],
            RuleIds {
                seq: Some(1),
                conds: vec![2],
                act: 3
            }
        );
        assert_eq!(
            ids[1],
            RuleIds {
                seq: None,
                conds: vec![],
                act: 4
            }
        );
        assert_eq!(
            ids[2],
            RuleIds {
                seq: Some(5),
                conds: vec![6, 7],
                act: 8
            }
        );
    }

    /// The pilot decides on what it saw `reaction` ticks ago (Vagrancy).
    #[test]
    fn a_pilot_acts_on_what_it_saw_reaction_ticks_ago() {
        let r = rules(r#"[{"if":[{"gap_below":300}],"do":"lunge"},{"do":"idle"}]"#);
        let mut t = Tree::new(r, 3, 0);
        let far = Seen {
            gap_cm: 900,
            ..Seen::default()
        };
        let near = Seen {
            gap_cm: 100,
            ..Seen::default()
        };
        assert_eq!(t.input(far).unwrap().0, "idle");
        t.end();
        for k in 0..3 {
            let got = t.input(near).unwrap().0;
            assert_eq!(got, "idle", "tick {k}: still seeing the far hero");
            t.end();
        }
        assert_eq!(
            t.input(near).unwrap().0,
            "lunge",
            "the fourth tick sees the near hero"
        );
    }

    /// A running move is not left for a later rule, but an interrupt breaks in.
    #[test]
    fn a_running_move_holds_until_an_interrupt() {
        let r = rules(
            r#"[{"if":["me_stunned"],"do":"reel","interrupt":true},{"if":[{"gap_below":300}],"do":"lunge"},{"do":"idle"}]"#,
        );
        let mut t = Tree::new(r, 0, 0);
        let s = Seen {
            gap_cm: 900,
            ..Seen::default()
        };
        assert_eq!(t.input(s).unwrap().0, "idle");
        let near = Seen {
            gap_cm: 100,
            ..Seen::default()
        };
        assert_eq!(t.input(near).unwrap().0, "idle", "committed to idle");
        let hit = Seen {
            stunned: true,
            ..near
        };
        assert_eq!(t.input(hit).unwrap().0, "reel", "the stun breaks in");
        assert_eq!(t.trace().active, vec![0, 1, 2, 3]);
    }

    /// CS-6: every shipped tree chooses among stunned, airborne and idle
    /// before anything else, and those first rules interrupt.
    #[test]
    fn every_enemy_kind_has_idle_stunned_airborne() {
        let all = trees();
        for kind in crate::world::Rules::shipped().enemies.keys() {
            let t = all.get(kind).unwrap_or_else(|| panic!("{kind} has no tree"));
            let firsts: Vec<(&Vec<Cond>, &str, bool)> = t
                .rules
                .iter()
                .take(3)
                .map(|r| (&r.when, r.act.as_str(), r.interrupt))
                .collect();
            assert_eq!(firsts[0].0, &vec![Cond::MeStunned], "{kind}: stunned first");
            assert!(firsts[0].2, "{kind}: the stun interrupts");
            assert_eq!(firsts[1].0, &vec![Cond::MeAirborne], "{kind}: airborne second");
            assert!(firsts[1].2, "{kind}: the air interrupts");
            assert_eq!(firsts[2].1, "idle", "{kind}: idle third");
        }
    }
}
