//! Keyed clips, sampled in the core each tick (CS-11, M2).
//!
//! A clip is a list of keys per joint (a tick, three angles in degrees, and
//! the easing into that key), a root curve (how far forward and up the body
//! is carried, in mm), and events: the hit window, the tick from which the
//! next move may cancel this one, and the ticks where the strike's phases
//! change. Protection of the Spheres wrote its poses as curves in Rust
//! (`refs/pots.md`, "Not to copy"); here they are data, `data/clips.json`.
//!
//! A change of clip does not snap: the difference between the old pose and
//! the new clip's first pose is carried as an offset that dies away along a
//! quintic and is exactly nothing on its last tick (after Bollo's
//! inertialization, GDC 2018; animation review).

use crate::fx::{self, ONE};
use crate::model::{Model, Pose};
use serde::Deserialize;
use std::collections::BTreeMap;

pub const CLIPS_JSON: &str = include_str!("../../../data/clips.json");

/// The most a joint may turn between two ticks, outside a declared cut
/// (animation review: 45 degrees).
pub const MAX_STEP_DEG: i32 = 45;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ease {
    Linear,
    In,
    Out,
    Inout,
    Hold,
}

#[derive(Debug, Clone, Deserialize)]
struct KeySpec(u32, [i32; 3], Ease);

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Events {
    /// first and last tick on which the move can land a hit
    #[serde(default)]
    pub hit: Option<[u32; 2]>,
    /// from this tick the next move may start
    #[serde(default)]
    pub cancel: Option<u32>,
    /// ends of anticipation, strike and follow-through; the rest is recovery
    #[serde(default)]
    pub phases: Option<[u32; 3]>,
    /// ticks where a jump in pose is allowed (a cut), for the jump test
    #[serde(default)]
    pub cuts: Vec<u32>,
}

#[derive(Debug, Clone, Deserialize)]
struct ClipSpec {
    len: u32,
    #[serde(default, rename = "loop")]
    looping: bool,
    #[serde(default)]
    keys: BTreeMap<String, Vec<KeySpec>>,
    /// (tick, forward mm, up mm)
    #[serde(default)]
    root: Vec<[i32; 3]>,
    #[serde(default)]
    events: Events,
}

#[derive(Debug, Clone, Deserialize)]
struct ClipsFile {
    clips: BTreeMap<String, ClipSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    pub tick: u32,
    pub angles: [i32; 3],
    pub ease: Ease,
}

#[derive(Debug, Clone)]
pub struct Clip {
    pub name: String,
    pub len: u32,
    pub looping: bool,
    pub keys: BTreeMap<String, Vec<Key>>,
    pub root: Vec<[i32; 3]>,
    pub events: Events,
}

pub type Clips = BTreeMap<String, Clip>;

pub fn load(json: &str) -> Result<Clips, String> {
    let file: ClipsFile = serde_json::from_str(json).map_err(|e| format!("clips.json: {e}"))?;
    let mut out = BTreeMap::new();
    for (name, c) in file.clips {
        let mut keys = BTreeMap::new();
        for (joint, ks) in c.keys {
            let mut v: Vec<Key> = ks
                .into_iter()
                .map(|KeySpec(tick, a, ease)| Key {
                    tick,
                    angles: crate::model::deg3(a),
                    ease,
                })
                .collect();
            v.sort_by_key(|k| k.tick);
            if v.iter().any(|k| k.tick > c.len) {
                return Err(format!("{name}: a key on {joint} lies past the clip's end"));
            }
            keys.insert(joint, v);
        }
        let root = c
            .root
            .iter()
            .map(|r| [r[0], crate::model::mm(r[1]), crate::model::mm(r[2])])
            .collect();
        out.insert(
            name.clone(),
            Clip {
                name,
                len: c.len,
                looping: c.looping,
                keys,
                root,
                events: c.events,
            },
        );
    }
    Ok(out)
}

pub fn shipped() -> Clips {
    load(CLIPS_JSON).unwrap_or_default()
}

/// `s` (Q16, 0 to ONE) shaped by an ease.
pub fn ease(e: Ease, s: i32) -> i32 {
    let s = s.clamp(0, ONE);
    match e {
        Ease::Linear => s,
        Ease::In => fx::mul(s, s),
        Ease::Out => {
            let r = ONE - s;
            ONE - fx::mul(r, r)
        }
        Ease::Inout => {
            // 3s² - 2s³
            let s2 = fx::mul(s, s);
            3 * s2 - 2 * fx::mul(s2, s)
        }
        Ease::Hold => {
            if s >= ONE {
                ONE
            } else {
                0
            }
        }
    }
}

/// How much of a transition's offset is left at `age` of `len` ticks:
/// 1 - (10s³ - 15s⁴ + 6s⁵), exactly ONE at the start and exactly 0 at the
/// end.
pub fn decay(age: u32, len: u32) -> i32 {
    if len == 0 || age >= len {
        return 0;
    }
    let s = (age as i64 * ONE as i64 / len as i64) as i32;
    let s3 = fx::mul(fx::mul(s, s), s);
    let s4 = fx::mul(s3, s);
    let s5 = fx::mul(s4, s);
    ONE - (10 * s3 - 15 * s4 + 6 * s5)
}

fn lerp(a: i32, b: i32, t: i32) -> i32 {
    a + fx::mul(b - a, t)
}

impl Clip {
    /// A tick within the clip: a looping clip wraps, others stop on the end.
    pub fn local(&self, tick: u32) -> u32 {
        if self.looping && self.len > 0 {
            tick % self.len
        } else {
            tick.min(self.len)
        }
    }

    /// The three angles of one joint at a tick.
    pub fn angles(&self, joint: &str, tick: u32) -> [i32; 3] {
        let Some(keys) = self.keys.get(joint) else {
            return [0; 3];
        };
        let t = self.local(tick);
        let Some(first) = keys.first() else {
            return [0; 3];
        };
        if t <= first.tick {
            return first.angles;
        }
        for w in keys.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            if t <= b.tick {
                let span = (b.tick - a.tick).max(1);
                let s = ease(b.ease, ((t - a.tick) as i64 * ONE as i64 / span as i64) as i32);
                return [
                    lerp(a.angles[0], b.angles[0], s),
                    lerp(a.angles[1], b.angles[1], s),
                    lerp(a.angles[2], b.angles[2], s),
                ];
            }
        }
        keys.last().map(|k| k.angles).unwrap_or([0; 3])
    }

    pub fn pose(&self, model: &Model, tick: u32) -> Pose {
        model.joints.iter().map(|j| self.angles(&j.name, tick)).collect()
    }

    /// How far the root has been carried by `tick`: (forward, up), Q16 m.
    pub fn root_at(&self, tick: u32) -> (i32, i32) {
        let t = self.local(tick) as i32;
        let Some(first) = self.root.first() else {
            return (0, 0);
        };
        if t <= first[0] {
            return (first[1], first[2]);
        }
        for w in self.root.windows(2) {
            let (a, b) = (w[0], w[1]);
            if t <= b[0] {
                let s = ((t - a[0]) as i64 * ONE as i64 / (b[0] - a[0]).max(1) as i64) as i32;
                return (lerp(a[1], b[1], s), lerp(a[2], b[2], s));
            }
        }
        let l = self.root[self.root.len() - 1];
        (l[1], l[2])
    }

    /// The root's movement over one tick (from `tick - 1` to `tick`).
    pub fn root_step(&self, tick: u32) -> (i32, i32) {
        if tick == 0 || (self.looping && self.local(tick) == 0) {
            return (0, 0);
        }
        let (f1, u1) = self.root_at(tick);
        let (f0, u0) = self.root_at(tick - 1);
        (f1 - f0, u1 - u0)
    }

    /// The phase of an attack at a tick, for the harness.
    pub fn phase(&self, tick: u32) -> &'static str {
        match self.events.phases {
            None => "",
            Some([a, s, f]) => {
                if tick < a {
                    "anticipation"
                } else if tick < s {
                    "strike"
                } else if tick < f {
                    "follow-through"
                } else {
                    "recovery"
                }
            }
        }
    }

    pub fn hits_at(&self, tick: u32) -> bool {
        matches!(self.events.hit, Some([a, b]) if tick >= a && tick <= b)
    }

    pub fn done(&self, tick: u32) -> bool {
        !self.looping && tick >= self.len
    }
}

/// One body's animation: the clip playing, its tick, and the offset left
/// over from the last change of clip.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Player {
    pub clip: String,
    pub tick: u32,
    pub offset: Pose,
    pub blend_age: u32,
    pub blend_len: u32,
}

impl Player {
    pub fn new(clip: &str) -> Player {
        Player {
            clip: clip.to_string(),
            ..Player::default()
        }
    }

    /// The pose to draw: the clip's pose plus what is left of the offset.
    pub fn pose(&self, clips: &Clips, model: &Model) -> Pose {
        let base = clips
            .get(&self.clip)
            .map(|c| c.pose(model, self.tick))
            .unwrap_or_else(|| model.rest());
        let k = decay(self.blend_age, self.blend_len);
        if k == 0 || self.offset.is_empty() {
            return base;
        }
        base.iter()
            .zip(self.offset.iter())
            .map(|(b, o)| {
                [
                    b[0] + fx::mul(o[0], k),
                    b[1] + fx::mul(o[1], k),
                    b[2] + fx::mul(o[2], k),
                ]
            })
            .collect()
    }

    /// Start another clip; the pose does not jump: the gap between what was
    /// drawn and the new clip's first pose fades over `blend` ticks.
    pub fn play(&mut self, clips: &Clips, model: &Model, clip: &str, blend: u32) {
        let now = self.pose(clips, model);
        let next = clips
            .get(clip)
            .map(|c| c.pose(model, 0))
            .unwrap_or_else(|| model.rest());
        // wrapped, so a clip that ends two turns round (the whirl) does not
        // unwind them
        self.offset = now
            .iter()
            .zip(next.iter())
            .map(|(a, b)| {
                [
                    fx::wrap(a[0] - b[0]),
                    fx::wrap(a[1] - b[1]),
                    fx::wrap(a[2] - b[2]),
                ]
            })
            .collect();
        self.clip = clip.to_string();
        self.tick = 0;
        self.blend_age = 0;
        self.blend_len = blend;
    }

    /// One tick on. `frozen` holds the clip's clock (hit-stop); the offset
    /// still fades, so a hit-stop never stores up a jump.
    pub fn step(&mut self, frozen: bool) {
        if !frozen {
            self.tick += 1;
        }
        if self.blend_age < self.blend_len {
            self.blend_age += 1;
        }
    }
}

/// Which changes of clip the game makes (CS-11 tests these): a reaction
/// (hurt, stunned, tumble, dead) can cut in at any tick; an attack gives
/// way to the next move only from its cancel tick, or once it ends; a
/// tumbling dog only lands; a lunging dog finishes its lunge.
pub fn allowed(from: &Clip, at: u32, to: &str) -> bool {
    let reaction = ["hurt", "dog_hurt", "dog_stunned", "dog_tumble", "dog_dead"];
    if reaction.contains(&to) {
        return true;
    }
    let name = from.name.as_str();
    if name == "dog_tumble" {
        return matches!(to, "dog_down" | "dog_stunned");
    }
    if name == "dog_lunge" {
        return at >= from.len && matches!(to, "dog_idle" | "dog_strafe");
    }
    if name.starts_with("dog") {
        return to != "dog_injured" || name == "dog_injured";
    }
    if to.starts_with("air_") && !(name.starts_with("air_") || matches!(name, "jump" | "fall" | "launcher")) {
        return false;
    }
    match from.events.cancel {
        Some(c) => at >= c,
        None => from.looping || at >= from.len || from.events.phases.is_none(),
    }
}

/// The transition length into a clip: short for attacks so a cancel stays
/// quick (6 ticks), longer for everything else (12): animation review.
pub fn blend_for(clip: &str) -> u32 {
    // the launcher opens with a crouch, so 8 ticks in costs nothing and
    // keeps a launcher-to-launcher change under the bound (46 degrees at 6)
    if clip == "launcher" {
        return 8;
    }
    // a hit reaction is a cut: it shows on the tick of contact (round 2)
    if clip == "dog_hurt" {
        return 0;
    }
    let attack = [
        "light",
        "heavy",
        "launch",
        "air_",
        "whirl",
        "dog_lunge",
        "dog_bite",
    ];
    if attack.iter().any(|a| clip.starts_with(a)) {
        6
    } else {
        12
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model;

    fn all() -> Clips {
        load(CLIPS_JSON).expect("clips.json loads")
    }

    #[test]
    fn eases_start_at_zero_and_end_at_one() {
        for e in [Ease::Linear, Ease::In, Ease::Out, Ease::Inout] {
            assert_eq!(ease(e, 0), 0, "{e:?}");
            assert_eq!(ease(e, ONE), ONE, "{e:?}");
        }
        // hand-worked: s = 1/2 → in 1/4, out 3/4, inout 1/2
        assert_eq!(ease(Ease::In, ONE / 2), ONE / 4);
        assert_eq!(ease(Ease::Out, ONE / 2), ONE * 3 / 4);
        assert_eq!(ease(Ease::Inout, ONE / 2), ONE / 2);
    }

    /// M2.2: the offset is whole at the start and exactly nothing on its
    /// last tick; at half way the quintic leaves exactly half.
    #[test]
    fn inertialization_reaches_zero_on_its_last_tick() {
        assert_eq!(decay(0, 6), ONE);
        assert_eq!(decay(6, 6), 0);
        assert_eq!(decay(3, 6), ONE / 2);
        assert!(decay(1, 6) > decay(2, 6) && decay(4, 6) > decay(5, 6) && decay(5, 6) > 0);
    }

    /// M2.1, hand-worked: a key at tick 0 of 0° and at tick 10 of 90°, linear,
    /// gives 45° at tick 5 and 90° after the end.
    #[test]
    fn clip_samples_match_hand_worked_keys() {
        let c = load(
            r#"{"clips":{"t":{"len":10,"keys":{"a":[[0,[0,0,0],"linear"],[10,[90,0,0],"linear"]]},
            "root":[[0,0,0],[10,1000,0]]}}}"#,
        )
        .unwrap()
        .remove("t")
        .unwrap();
        assert_eq!(c.angles("a", 5)[0], fx::deg(45));
        assert_eq!(c.angles("a", 12)[0], fx::deg(90));
        assert_eq!(c.root_at(5).0, ONE / 2, "half a metre at half way");
        // a tenth of a metre a tick: 0.1 m is 6553.6 in Q16, and the keys
        // interpolate to 26214 at tick 4 and 32768 at tick 5
        assert_eq!(c.root_step(5).0, 32768 - 26214);
    }

    fn worst_step(clip: &Clip, m: &model::Model) -> (i32, u32, String) {
        let mut worst = (0, 0, String::new());
        let end = if clip.looping { clip.len + 1 } else { clip.len };
        for t in 1..=end {
            if clip.events.cuts.contains(&t) {
                continue;
            }
            let (a, b) = (clip.pose(m, t - 1), clip.pose(m, t));
            for (j, (p, q)) in a.iter().zip(b.iter()).enumerate() {
                for k in 0..3 {
                    let d = (fx::wrap(q[k] - p[k])).abs();
                    if d > worst.0 {
                        worst = (d, t, m.joints[j].name.clone());
                    }
                }
            }
        }
        worst
    }

    /// CS-11 within clips: no joint turns more than 45° between two ticks
    /// in any shipped clip, except at a declared cut.
    #[test]
    fn no_joint_jumps_between_ticks() {
        let models = model::shipped();
        for (name, c) in all() {
            let m = if name.starts_with("dog") {
                &models["dog"]
            } else {
                &models["hero"]
            };
            let (d, t, j) = worst_step(&c, m);
            assert!(
                d <= fx::deg(MAX_STEP_DEG),
                "{name}: {j} turns {} degrees at tick {t}",
                d * 360 / fx::TURN
            );
        }
    }

    /// CS-11 across clips: start any clip from any tick of another clip of
    /// the same body, and no joint jumps more than the bound, using the
    /// transition lengths the game uses (6 for attacks, 12 otherwise).
    #[test]
    fn no_joint_jumps_through_a_change_of_clip() {
        let models = model::shipped();
        let clips = all();
        let mut bad = Vec::new();
        for (from_name, from) in &clips {
            let body = if from_name.starts_with("dog") {
                "dog"
            } else {
                "hero"
            };
            let m = &models[body];
            for (to_name, to) in clips
                .iter()
                .filter(|(n, _)| n.starts_with("dog") == (body == "dog"))
            {
                let blend = crate::clip::blend_for(to_name);
                for at in (0..=from.len).step_by(3).filter(|&at| allowed(from, at, to_name)) {
                    let mut p = Player::new(from_name);
                    p.tick = at;
                    let before = p.pose(&clips, m);
                    p.play(&clips, m, to_name, blend);
                    let mut prev = before;
                    for _ in 0..to.len.min(blend + 4) {
                        p.step(false);
                        let now = p.pose(&clips, m);
                        for (j, (a, b)) in prev.iter().zip(now.iter()).enumerate() {
                            for k in 0..3 {
                                let d = fx::wrap(b[k] - a[k]).abs();
                                if d > fx::deg(MAX_STEP_DEG) && !to.events.cuts.contains(&p.tick) {
                                    bad.push(format!(
                                        "{from_name}@{at} -> {to_name}: {} turns {} degrees at tick {}",
                                        m.joints[j].name,
                                        d * 360 / fx::TURN,
                                        p.tick
                                    ));
                                }
                            }
                        }
                        prev = now;
                    }
                }
            }
        }
        assert!(
            bad.is_empty(),
            "{} jumps, first: {:?}",
            bad.len(),
            &bad[..bad.len().min(12)]
        );
    }
}
