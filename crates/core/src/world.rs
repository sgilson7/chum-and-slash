//! The fight: one tick of the hero, the enemies, their hits and what the
//! player sees, hears and feels of them (CS-1 to CS-9, M3).
//!
//! `World::tick` is the one function the page and the text harness both
//! call (M0.2). It returns the tick's events: sounds, rumbles, combos, and
//! the rest; `World::describe` says in words what each body did, for the
//! harness and its playtesters.
//!
//! Order within a tick: input and lock-on; the hero's moves and motion; the
//! enemies' intents (pilot) and motion; hits; stun, air and landing;
//! hit-stop, shake and the camera's knock wind down.

use crate::clip::{self, Clips, Player};
use crate::fx::{self, ONE};
use crate::geom::{facing, v3, V3};
use crate::model::{self, Model, Pose};
use crate::pilot::{self, Intent};
use serde::Deserialize;
use std::collections::BTreeMap;

pub const COMBAT_JSON: &str = include_str!("../../../data/combat.json");

// ---------------------------------------------------------------- rules

#[derive(Debug, Clone, Deserialize)]
pub struct MoveSpec {
    pub press: String,
    #[serde(default)]
    pub air: bool,
    #[serde(default)]
    pub ability: bool,
    pub damage: i32,
    pub hitstop: u32,
    pub push_mm: i32,
    pub reach_mm: i32,
    pub arc_deg: i32,
    pub stagger: i32,
    pub hit: String,
    #[serde(default)]
    pub launches: bool,
    #[serde(default)]
    pub stamina: i32,
    #[serde(default)]
    pub stuns: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComboSpec {
    pub buffer: u32,
    pub reset: u32,
    pub recovery: u32,
    pub cap_light: u32,
    pub cap_heavy: u32,
    pub cap_total: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NamedCombo {
    pub name: String,
    pub presses: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StaminaSpec {
    pub max: i32,
    pub sprint_per_tick: i32,
    pub regen_per_tick: i32,
    pub regen_delay: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StunSpec {
    pub window: u32,
    pub tail: u32,
    pub air_float: u32,
    pub air_gravity_pct: i32,
    pub launch_mm: i32,
    pub landed: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeroSpec {
    pub run_mm_s: i32,
    pub sprint_mm_s: i32,
    pub jump_mm: i32,
    pub gravity_mm_s2: i32,
    pub radius_mm: i32,
    pub lock_range_mm: i32,
    pub snap_mm: i32,
    pub health: i32,
    pub turn_deg: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnemySpec {
    pub model: String,
    pub scale_pct: i32,
    pub health: i32,
    pub poise: i32,
    pub radius_mm: i32,
    pub push_pct: i32,
    pub bite: i32,
    pub attacks: bool,
    #[serde(default)]
    pub dies_on_combo: Option<String>,
    #[serde(default)]
    pub injured: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rules {
    pub moves: BTreeMap<String, MoveSpec>,
    pub combo: ComboSpec,
    pub combos: Vec<NamedCombo>,
    pub hitstop_cap: u32,
    pub stamina: StaminaSpec,
    pub stun: StunSpec,
    pub hero: HeroSpec,
    pub push_ticks: u32,
    pub shake_ticks: u32,
    pub enemies: BTreeMap<String, EnemySpec>,
}

impl Rules {
    pub fn shipped() -> Rules {
        serde_json::from_str(COMBAT_JSON).expect("combat.json is checked by its test")
    }
}

/// Gravity per tick², from mm/s² (60 ticks a second).
pub fn per_tick2(mm_s2: i32) -> i32 {
    model::mm(mm_s2) / 3600
}

/// Speed per tick, from mm/s.
pub fn per_tick(mm_s: i32) -> i32 {
    model::mm(mm_s) / 60
}

/// The upward speed that rises `h` under gravity `g` (both Q16, per tick).
pub fn rise_speed(g: i32, h: i32) -> i32 {
    fx::sqrt(2 * fx::mul(g, h))
}

// ---------------------------------------------------------------- input

pub const B_LIGHT: u32 = 1;
pub const B_HEAVY: u32 = 2;
pub const B_LAUNCH: u32 = 4;
pub const B_JUMP: u32 = 8;
/// held: the ability modifier (L1 / LB / Q)
pub const B_MOD: u32 = 16;
pub const B_LOCK: u32 = 32;
pub const B_SPRINT: u32 = 64;
pub const B_SKIP: u32 = 128;
pub const B_FWD: u32 = 256;
pub const B_BACK: u32 = 512;
pub const B_LEFT: u32 = 1024;
pub const B_RIGHT: u32 = 2048;

/// One tick's input: buttons held, a stick (Q16, overrides the keys when
/// it is pushed), and how far the camera was turned.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Input {
    pub bits: u32,
    pub stick_x: i32,
    pub stick_z: i32,
    pub look_yaw: i32,
    pub look_pitch: i32,
}

impl Input {
    pub fn bits(bits: u32) -> Input {
        Input {
            bits,
            ..Input::default()
        }
    }
}

// ---------------------------------------------------------------- events

/// Something the page plays or the harness reports. `a` and `b` carry a
/// sound's playback rate and gain (Q16), so the page does no maths (D3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub kind: &'static str,
    pub name: String,
    pub a: i32,
    pub b: i32,
}

fn ev(kind: &'static str, name: &str) -> Event {
    Event {
        kind,
        name: name.to_string(),
        a: ONE,
        b: ONE,
    }
}

/// Playback rates one semitone apart, from -3 to +8, as Q16:
/// round(65536 × 2^(s/12)), worked out once by hand so the core needs no
/// power function.
pub const SEMITONES: [i32; 12] = [
    55109, 58386, 61858, 65536, 69433, 73562, 77936, 82570, 87480, 92682, 98193, 104032,
];

pub fn semitone(s: i32) -> i32 {
    SEMITONES[(s + 3).clamp(0, 11) as usize]
}

/// A small hash for variation, so the core needs no random device.
pub fn mix(a: u32, b: u32) -> u32 {
    let mut x = a.wrapping_mul(0x9E37_79B9) ^ b.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^ (x >> 12)
}

/// How strong each rumble is, so a weaker one never cuts a stronger one
/// short (controllers review).
pub fn rumble_rank(name: &str) -> (u32, u32) {
    match name {
        "light_hit" => (1, 4),
        "whirl_hit" => (2, 9),
        "heavy_hit" => (2, 7),
        "launcher_hit" => (3, 14),
        "hurt" => (2, 10),
        _ => (0, 0),
    }
}

// ---------------------------------------------------------------- bodies

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Stunned,
    Airborne,
    Down,
    Dead,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Idle => "idle",
            State::Stunned => "stunned",
            State::Airborne => "airborne",
            State::Down => "down",
            State::Dead => "dead",
        }
    }
}

/// A move in progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Act {
    pub name: String,
    pub serial: u32,
    pub hit: Vec<u32>,
    pub capped: bool,
    pub ends_string: bool,
    /// extra forward per tick while closing on a locked target (Q16)
    pub snap: i32,
    pub snap_ticks: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Combo {
    pub presses: Vec<String>,
    pub light_run: u32,
    pub heavy_run: u32,
    pub recovery: u32,
    pub buffer: Option<(String, u32)>,
    pub since_end: u32,
}

#[derive(Debug, Clone)]
pub struct Hero {
    pub pos: V3,
    pub vel: V3,
    pub yaw: i32,
    pub grounded: bool,
    pub anim: Player,
    pub prev_pose: Pose,
    pub prev_pos: V3,
    pub prev_yaw: i32,
    pub hitstop: u32,
    pub stamina: i32,
    pub rested: u32,
    pub health: i32,
    pub combo: Combo,
    pub act: Option<Act>,
    pub lock: Option<u32>,
    pub lock_on: bool,
    pub sprinting: bool,
    pub prev_bits: u32,
}

#[derive(Debug, Clone)]
pub struct Enemy {
    pub id: u32,
    pub kind: String,
    pub pos: V3,
    pub vel: V3,
    pub yaw: i32,
    pub state: State,
    pub stun: u32,
    pub health: i32,
    pub anim: Player,
    pub prev_pose: Pose,
    pub prev_pos: V3,
    pub prev_yaw: i32,
    pub hitstop: u32,
    pub shake: u32,
    pub float: u32,
    pub knock: V3,
    pub knock_ticks: u32,
    /// the tick-count left in the current lunge's bite, and whether it bit
    pub lunge: u32,
    pub bit: bool,
    pub brain: pilot::Brain,
    /// stunned by the hero's current string (its stun ends with the string)
    pub in_string: bool,
    pub dead_for: u32,
    /// the last move that hit it, for the harness
    pub reaction: &'static str,
}

impl Enemy {
    pub fn spec<'a>(&self, r: &'a Rules) -> &'a EnemySpec {
        &r.enemies[&self.kind]
    }

    pub fn scale(&self, r: &Rules) -> i32 {
        self.spec(r).scale_pct * ONE / 100
    }

    pub fn radius(&self, r: &Rules) -> i32 {
        model::mm(self.spec(r).radius_mm)
    }

    pub fn stunned_like(&self) -> bool {
        matches!(self.state, State::Stunned | State::Down)
    }
}

/// The camera's knock: Chumworld Arena's `feel::ring` (CS-Q22, MIT): each
/// tick it swings back through the middle and keeps 55% of itself, and
/// under 1/2000 of a metre it is nothing, so a still view draws the same
/// pixels.
pub const RING_PCT: i32 = 55;
pub const QUIET: i32 = ONE / 2000;
pub const KNOCK_MAX: i32 = ONE * 5 / 100;

pub fn ring(k: V3) -> V3 {
    let n = v3(
        -k.x * RING_PCT / 100,
        -k.y * RING_PCT / 100,
        -k.z * RING_PCT / 100,
    );
    if n.len() < QUIET {
        V3::ZERO
    } else {
        n
    }
}

#[derive(Debug, Clone)]
pub struct World {
    pub rules: Rules,
    pub clips: Clips,
    pub models: BTreeMap<String, Model>,
    pub tick: u32,
    pub hero: Hero,
    pub enemies: Vec<Enemy>,
    pub next_id: u32,
    pub serial: u32,
    pub cam_yaw: i32,
    pub cam_pitch: i32,
    pub knock: V3,
    pub prev_knock: V3,
    /// the arena's floor: a disc of this radius around the origin
    pub floor_radius: i32,
    /// game speed (Q16): the tutorial slows it; 0 is paused
    pub speed: i32,
    pub speed_acc: i32,
    pub rumble_until: u32,
    pub rumble_rank: u32,
    /// the pack's attack tokens (M4.4)
    pub pack: pilot::Pack,
    pub events: Vec<Event>,
}

impl World {
    pub fn new() -> World {
        let rules = Rules::shipped();
        let clips = clip::shipped();
        let models = model::shipped();
        let hero_model = &models["hero"];
        let stamina = rules.stamina.max;
        let health = rules.hero.health;
        let rest = hero_model.rest();
        World {
            hero: Hero {
                pos: V3::ZERO,
                vel: V3::ZERO,
                yaw: 0,
                grounded: true,
                anim: Player::new("idle"),
                prev_pose: rest,
                prev_pos: V3::ZERO,
                prev_yaw: 0,
                hitstop: 0,
                stamina,
                rested: 0,
                health,
                combo: Combo::default(),
                act: None,
                lock: None,
                lock_on: true,
                sprinting: false,
                prev_bits: 0,
            },
            rules,
            clips,
            models,
            tick: 0,
            enemies: Vec::new(),
            next_id: 1,
            serial: 0,
            cam_yaw: 0,
            cam_pitch: -fx::deg(12),
            knock: V3::ZERO,
            prev_knock: V3::ZERO,
            floor_radius: fx::int(24),
            speed: ONE,
            speed_acc: 0,
            rumble_until: 0,
            rumble_rank: 0,
            pack: pilot::Pack::default(),
            events: Vec::new(),
        }
    }

    pub fn spawn(&mut self, kind: &str, at: V3, yaw: i32) -> Result<u32, String> {
        let spec = self
            .rules
            .enemies
            .get(kind)
            .ok_or(format!("no enemy called {kind}"))?;
        let model = &self.models[&spec.model];
        let id = self.next_id;
        self.next_id += 1;
        let injured = spec.injured;
        self.enemies.push(Enemy {
            id,
            kind: kind.to_string(),
            pos: at,
            vel: V3::ZERO,
            yaw,
            state: State::Idle,
            stun: 0,
            health: spec.health,
            anim: Player::new(if injured { "dog_injured" } else { "dog_idle" }),
            prev_pose: model.rest(),
            prev_pos: at,
            prev_yaw: yaw,
            hitstop: 0,
            shake: 0,
            float: 0,
            knock: V3::ZERO,
            knock_ticks: 0,
            lunge: 0,
            bit: false,
            brain: pilot::Brain::for_kind(kind),
            in_string: false,
            dead_for: 0,
            reaction: "",
        });
        Ok(id)
    }

    pub fn enemy(&self, id: u32) -> Option<&Enemy> {
        self.enemies.iter().find(|e| e.id == id)
    }

    fn gravity(&self) -> i32 {
        per_tick2(self.rules.hero.gravity_mm_s2)
    }

    fn rumble(&mut self, name: &str) {
        let (rank, ticks) = rumble_rank(name);
        if self.tick < self.rumble_until && rank < self.rumble_rank {
            return;
        }
        self.rumble_rank = rank;
        self.rumble_until = self.tick + ticks;
        self.events.push(ev("rumble", name));
    }

    fn sound(&mut self, name: &str, semis: i32, salt: u32) {
        // ±1 semitone and a little gain, from a hash of the tick (sound review)
        let h = mix(self.tick, salt);
        let var = (h % 3) as i32 - 1;
        let gain = ONE - (((h >> 4) % 13) as i32) * ONE / 100;
        self.events.push(Event {
            kind: "sound",
            name: name.to_string(),
            a: semitone(semis + var),
            b: gain,
        });
    }

    // ------------------------------------------------------------ the tick

    /// One tick. Game speed below one (the tutorial's slow-down) runs the
    /// world on some ticks only; at zero it is paused, and only the input
    /// the tutorial waits for gets through (tutorial.rs).
    pub fn tick(&mut self, input: Input) -> Vec<Event> {
        self.events.clear();
        self.speed_acc += self.speed;
        if self.speed_acc < ONE {
            self.hero.prev_bits &= input.bits;
            return std::mem::take(&mut self.events);
        }
        self.speed_acc -= ONE;
        self.step(input);
        std::mem::take(&mut self.events)
    }

    fn step(&mut self, input: Input) {
        self.tick += 1;
        self.keep_previous();
        let pressed = input.bits & !self.hero.prev_bits;
        self.hero.prev_bits = input.bits;
        self.cam_yaw = fx::wrap(self.cam_yaw + input.look_yaw);
        self.cam_pitch = (self.cam_pitch + input.look_pitch).clamp(-fx::deg(40), fx::deg(20));
        self.lock_on(pressed);
        self.hero_moves(input, pressed);
        self.hero_motion(input, pressed);
        self.pack.step(&self.enemies, &self.rules);
        for i in 0..self.enemies.len() {
            self.enemy_step(i);
        }
        self.hits();
        self.bites();
        self.prev_knock = self.knock;
        self.knock = ring(self.knock);
    }

    fn keep_previous(&mut self) {
        let hm = &self.models["hero"];
        self.hero.prev_pose = self.hero.anim.pose(&self.clips, hm);
        self.hero.prev_pos = self.hero.pos;
        self.hero.prev_yaw = self.hero.yaw;
        let dm = &self.models["dog"];
        for e in self.enemies.iter_mut() {
            e.prev_pose = e.anim.pose(&self.clips, dm);
            e.prev_pos = e.pos;
            e.prev_yaw = e.yaw;
        }
    }

    fn alive(&self) -> impl Iterator<Item = &Enemy> {
        self.enemies.iter().filter(|e| e.state != State::Dead)
    }

    fn lock_on(&mut self, pressed: u32) {
        if pressed & B_LOCK != 0 {
            self.hero.lock_on = !self.hero.lock_on;
        }
        let range = model::mm(self.rules.hero.lock_range_mm);
        let still = self
            .hero
            .lock
            .and_then(|id| self.enemy(id))
            .is_some_and(|e| e.state != State::Dead);
        if !self.hero.lock_on {
            self.hero.lock = None;
        } else if !still {
            let p = self.hero.pos;
            self.hero.lock = self
                .alive()
                .map(|e| (e.pos.sub(p).len_xz(), e.id))
                .filter(|(d, _)| *d <= range)
                .min()
                .map(|(_, id)| id);
        }
        if let Some(t) = self.hero.lock.and_then(|id| self.enemy(id)) {
            // the camera swings round behind the hero to face the target
            let to = t.pos.sub(self.hero.pos);
            let want = yaw_of(to);
            let d = fx::wrap(want - self.cam_yaw);
            self.cam_yaw = fx::wrap(self.cam_yaw + d / 8);
        }
    }

    fn hero_moves(&mut self, input: Input, pressed: u32) {
        let c = &self.rules.combo;
        let (buffer, reset, recovery) = (c.buffer, c.reset, c.recovery);
        if self.hero.hitstop > 0 {
            self.hero.hitstop -= 1;
            self.hero.anim.step(true);
            if let Some(p) = press_of(pressed) {
                self.hero.combo.buffer = Some((p.to_string(), 0));
            }
            return;
        }
        // a press is buffered; it waits for the move's cancel tick
        if let Some(p) = press_of(pressed) {
            self.hero.combo.buffer = Some((p.to_string(), 0));
        }
        if let Some((_, age)) = self.hero.combo.buffer.as_mut() {
            *age += 1;
            if *age > buffer {
                self.hero.combo.buffer = None;
            }
        }
        if self.hero.combo.recovery > 0 {
            self.hero.combo.recovery -= 1;
            self.hero.combo.buffer = None;
            self.hero.anim.step(false);
            if self.hero.combo.recovery == 0 {
                self.hero.anim.play(&self.clips, &self.models["hero"], "idle", 12);
            }
            return;
        }
        let ready = match &self.hero.act {
            None => true,
            Some(a) => {
                let c = &self.clips[&a.name];
                c.events.cancel.is_some_and(|k| self.hero.anim.tick >= k) || c.done(self.hero.anim.tick)
            }
        };
        if ready {
            if let Some((p, _)) = self.hero.combo.buffer.clone() {
                if self.start(&p, input.bits & B_MOD != 0) {
                    self.hero.combo.buffer = None;
                    self.hero.anim.step(false);
                    return;
                }
                self.hero.combo.buffer = None;
            }
        }
        // a move that has run out
        if let Some(a) = self.hero.act.clone() {
            if self.clips[&a.name].done(self.hero.anim.tick) {
                self.hero.act = None;
                self.hero.combo.since_end = 0;
                if a.capped {
                    self.end_string();
                    self.hero.combo.recovery = recovery;
                    self.hero
                        .anim
                        .play(&self.clips, &self.models["hero"], "recover", 12);
                    self.events.push(ev("recovery", "cap"));
                } else if a.ends_string {
                    self.end_string();
                }
            }
        } else if !self.hero.combo.presses.is_empty() {
            self.hero.combo.since_end += 1;
            if self.hero.combo.since_end > reset {
                self.end_string();
            }
        }
        self.hero.anim.step(false);
    }

    fn end_string(&mut self) {
        self.hero.combo.presses.clear();
        self.hero.combo.light_run = 0;
        self.hero.combo.heavy_run = 0;
        let tail = self.rules.stun.tail;
        for e in self.enemies.iter_mut().filter(|e| e.in_string) {
            e.in_string = false;
            if e.state == State::Stunned {
                e.stun = e.stun.min(tail);
            }
        }
    }

    /// Starts the move a press asks for, if it may start; true if it did.
    fn start(&mut self, press: &str, modifier: bool) -> bool {
        let airborne = !self.hero.grounded;
        let name = if modifier && press == "light" {
            "whirl".to_string()
        } else if airborne {
            match press {
                "light" => "air_light".to_string(),
                "heavy" => "air_heavy".to_string(),
                _ => return false,
            }
        } else {
            match press {
                "light" => format!(
                    "light_{}",
                    self.hero.combo.presses.iter().filter(|p| *p == "light").count() % 4 + 1
                ),
                "heavy" => format!(
                    "heavy_{}",
                    self.hero.combo.presses.iter().filter(|p| *p == "heavy").count() % 3 + 1
                ),
                "launcher" => "launcher".to_string(),
                _ => return false,
            }
        };
        let spec = self.rules.moves[&name].clone();
        if spec.stamina > self.hero.stamina {
            self.events.push(ev("tired", &name));
            return false;
        }
        if spec.stamina > 0 {
            self.hero.stamina -= spec.stamina;
            self.hero.rested = 0;
        }
        let c = &mut self.hero.combo;
        let first = c.presses.is_empty();
        if spec.ability {
            // an ability stands outside the string
            c.presses.clear();
            c.light_run = 0;
            c.heavy_run = 0;
        } else {
            c.presses.push(press.to_string());
            match press {
                "light" => {
                    c.light_run += 1;
                    c.heavy_run = 0;
                }
                "heavy" => {
                    c.heavy_run += 1;
                    c.light_run = 0;
                }
                _ => {
                    c.light_run = 0;
                    c.heavy_run = 0;
                }
            }
        }
        let cs = &self.rules.combo;
        let capped = !spec.ability
            && (c.light_run >= cs.cap_light
                || c.heavy_run >= cs.cap_heavy
                || c.presses.len() as u32 >= cs.cap_total);
        let presses = c.presses.clone();
        if let Some(done) = self.rules.combos.iter().find(|k| k.presses == presses) {
            let n = done.name.clone();
            self.events.push(ev("combo", &n));
        }
        // close on a locked target at the start of a string
        let mut snap = 0;
        let mut snap_ticks = 0;
        let clip = &self.clips[&name];
        if let Some(t) = self.hero.lock.and_then(|id| self.enemy(id)) {
            let to = t.pos.sub(self.hero.pos);
            self.hero.yaw = yaw_of(to);
            if first && !spec.ability && self.hero.grounded {
                let reach = model::mm(spec.reach_mm) * 3 / 4;
                let gap = (to.len_xz() - reach - clip.root_at(clip.len).0)
                    .clamp(0, model::mm(self.rules.hero.snap_mm));
                snap_ticks = clip.events.phases.map(|p| p[1]).unwrap_or(1).max(1);
                snap = gap / snap_ticks as i32;
            }
        }
        self.serial += 1;
        self.hero.act = Some(Act {
            name: name.clone(),
            serial: self.serial,
            hit: Vec::new(),
            capped,
            ends_string: press == "launcher",
            snap,
            snap_ticks,
        });
        let blend = clip::blend_for(&name);
        self.hero
            .anim
            .play(&self.clips, &self.models["hero"], &name, blend);
        true
    }

    fn hero_motion(&mut self, input: Input, pressed: u32) {
        let g = self.gravity();
        let h = &self.rules.hero;
        let (run, sprint, jump_mm, turn) = (
            per_tick(h.run_mm_s),
            per_tick(h.sprint_mm_s),
            h.jump_mm,
            fx::deg(h.turn_deg),
        );
        let st = &self.rules.stamina;
        let (max, sprint_cost, regen, delay) =
            (st.max, st.sprint_per_tick, st.regen_per_tick, st.regen_delay);
        let hero_frozen = self.hero.hitstop > 0;
        // stamina comes back after a rest (CS-3: only sprint, air moves and abilities spend it)
        if self.hero.rested >= delay {
            self.hero.stamina = (self.hero.stamina + regen).min(max);
        } else {
            self.hero.rested += 1;
        }
        if hero_frozen {
            return;
        }
        // the way the stick or keys point, turned by the camera
        let (mut mx, mut mz) = (input.stick_x, input.stick_z);
        if mx == 0 && mz == 0 {
            mx = (if input.bits & B_RIGHT != 0 { ONE } else { 0 })
                - (if input.bits & B_LEFT != 0 { ONE } else { 0 });
            mz = (if input.bits & B_FWD != 0 { ONE } else { 0 })
                - (if input.bits & B_BACK != 0 { ONE } else { 0 });
        }
        let fwd = facing(self.cam_yaw, 0);
        let right = v3(-fwd.z, 0, fwd.x);
        let mut want = fwd.scale(mz).add(right.scale(mx));
        if want.len() > ONE {
            want = want.norm();
        }
        let acting = self.hero.act.is_some() || self.hero.combo.recovery > 0;
        let moving = want.len() > ONE / 5 && !acting;
        self.hero.sprinting =
            moving && input.bits & B_SPRINT != 0 && self.hero.grounded && self.hero.stamina >= sprint_cost;
        if self.hero.sprinting {
            self.hero.stamina -= sprint_cost;
            self.hero.rested = 0;
        }
        let speed = if self.hero.sprinting { sprint } else { run };
        if self.hero.grounded {
            if moving {
                self.hero.vel.x = fx::mul(want.x, speed);
                self.hero.vel.z = fx::mul(want.z, speed);
                let target = match self.hero.lock.and_then(|id| self.enemy(id)) {
                    Some(t) if !self.hero.sprinting => yaw_of(t.pos.sub(self.hero.pos)),
                    _ => yaw_of(want),
                };
                let d = fx::wrap(target - self.hero.yaw).clamp(-turn, turn);
                self.hero.yaw = fx::wrap(self.hero.yaw + d);
            } else {
                self.hero.vel.x = 0;
                self.hero.vel.z = 0;
            }
        }
        // jumping is free (CS-Q9); a jump from the ground reaches jump_mm
        if pressed & B_JUMP != 0 && self.hero.grounded && self.hero.combo.recovery == 0 {
            let cancel_ok = match &self.hero.act {
                None => true,
                Some(a) => self.clips[&a.name]
                    .events
                    .cancel
                    .is_some_and(|k| self.hero.anim.tick >= k),
            };
            if cancel_ok {
                self.hero.vel.y = rise_speed(g, model::mm(jump_mm));
                self.hero.grounded = false;
                if self.hero.act.take().is_some_and(|a| a.ends_string || a.capped) {
                    self.end_string();
                }
                self.hero.anim.play(&self.clips, &self.models["hero"], "jump", 8);
                self.events.push(ev("jump", "jump"));
            }
        }
        // the move's own carry forward (root motion as a request, M2.3)
        if let Some(a) = self.hero.act.clone() {
            let c = &self.clips[&a.name];
            let (f, _) = c.root_step(self.hero.anim.tick);
            let mut f = f;
            if self.hero.anim.tick <= a.snap_ticks {
                f += a.snap;
            }
            // never run through the locked target
            if let Some(t) = self.hero.lock.and_then(|id| self.enemy(id)) {
                let gap = t.pos.sub(self.hero.pos).len_xz()
                    - t.radius(&self.rules)
                    - model::mm(self.rules.hero.radius_mm);
                f = f.min(gap.max(0));
            }
            let dir = facing(self.hero.yaw, 0);
            self.hero.pos = self.hero.pos.add(dir.scale(f));
            if self.hero.grounded {
                self.hero.vel.x = 0;
                self.hero.vel.z = 0;
            }
        }
        // the air: gravity, eased while an air string is landing hits
        if !self.hero.grounded {
            let float = self
                .hero
                .act
                .as_ref()
                .is_some_and(|a| self.rules.moves[&a.name].air && !a.hit.is_empty());
            let gg = if float {
                g * self.rules.stun.air_gravity_pct / 100
            } else {
                g
            };
            self.hero.vel.y -= gg;
        }
        self.hero.pos = self.hero.pos.add(self.hero.vel);
        if self.hero.pos.y <= 0 && !self.hero.grounded {
            self.hero.pos.y = 0;
            self.hero.vel = V3::ZERO;
            self.hero.grounded = true;
            if self
                .hero
                .act
                .as_ref()
                .is_some_and(|a| self.rules.moves[&a.name].air)
            {
                self.hero.act = None;
                self.end_string();
            }
            if self.hero.act.is_none() {
                self.hero.anim.play(&self.clips, &self.models["hero"], "idle", 12);
            }
            self.events.push(ev("land", "hero"));
        }
        self.keep_on_floor();
        // the locomotion clip follows the motion
        if self.hero.act.is_none() && self.hero.combo.recovery == 0 {
            let want_clip = if !self.hero.grounded {
                if self.hero.anim.clip == "jump" && !self.clips["jump"].done(self.hero.anim.tick) {
                    "jump"
                } else {
                    "fall"
                }
            } else if moving && self.hero.sprinting {
                "sprint"
            } else if moving {
                "run"
            } else {
                "idle"
            };
            if self.hero.anim.clip != want_clip {
                self.hero
                    .anim
                    .play(&self.clips, &self.models["hero"], want_clip, 12);
            }
        }
    }

    fn keep_on_floor(&mut self) {
        let r = self.floor_radius;
        let p = v3(self.hero.pos.x, 0, self.hero.pos.z);
        if p.len() > r {
            let q = p.norm().scale(r);
            self.hero.pos.x = q.x;
            self.hero.pos.z = q.z;
        }
    }

    fn enemy_step(&mut self, i: usize) {
        let g = self.gravity();
        let stun = self.rules.stun.clone();
        let dm = self.models["dog"].clone();
        // the tree runs every tick, as Vagrancy's do, so its lit node is
        // always current; the core acts on it only for an idle dog (M4.2)
        let thought = if self.enemies[i].state != State::Dead && self.enemies[i].hitstop == 0 {
            pilot::think(self, i)
        } else {
            Intent::Hold
        };
        let intent = if self.enemies[i].state == State::Idle {
            thought
        } else {
            Intent::Hold
        };
        let hero_pos = self.hero.pos;
        let clips = &self.clips;
        let e = &mut self.enemies[i];
        if e.shake > 0 {
            e.shake -= 1;
        }
        if e.hitstop > 0 {
            e.hitstop -= 1;
            e.anim.step(true);
            return;
        }
        if e.knock_ticks > 0 {
            e.knock_ticks -= 1;
            e.pos = e.pos.add(e.knock);
        }
        match e.state {
            State::Dead => {
                e.dead_for += 1;
            }
            State::Stunned => {
                if e.stun > 0 {
                    e.stun -= 1;
                }
                if e.stun == 0 {
                    e.state = State::Idle;
                    e.in_string = false;
                    e.anim.play(clips, &dm, "dog_idle", 12);
                } else if e.anim.clip != "dog_stunned" && clips[&e.anim.clip].done(e.anim.tick) {
                    e.anim.play(clips, &dm, "dog_stunned", 12);
                }
            }
            State::Down => {
                if e.stun > 0 {
                    e.stun -= 1;
                }
                if e.stun == 0 {
                    e.state = State::Stunned;
                    e.stun = stun.tail;
                    e.anim.play(clips, &dm, "dog_stunned", 12);
                }
            }
            State::Airborne => {
                let gg = if e.float > 0 {
                    g * stun.air_gravity_pct / 100
                } else {
                    g
                };
                if e.float > 0 {
                    e.float -= 1;
                }
                e.vel.y -= gg;
                e.pos = e.pos.add(e.vel);
                if e.pos.y <= 0 {
                    e.pos.y = 0;
                    e.vel = V3::ZERO;
                    e.state = State::Down;
                    e.stun = stun.landed;
                    e.anim.play(clips, &dm, "dog_down", 12);
                    self.events.push(ev("land", "enemy"));
                }
            }
            State::Idle => {
                // the tree decides what a dog does (CS-10); `attacks` only
                // keeps the pack's tokens from a dog that never attacks
                pilot::act(e, intent, hero_pos, clips, &dm);
            }
        }
        if e.state != State::Airborne {
            e.pos.y = 0;
        }
        e.anim.step(false);
        let r = self.floor_radius;
        let p = v3(e.pos.x, 0, e.pos.z);
        if p.len() > r {
            let q = p.norm().scale(r);
            e.pos.x = q.x;
            e.pos.z = q.z;
        }
    }

    /// The hero's move lands on every enemy in reach and in its arc during
    /// its hit window, once per enemy per move.
    fn hits(&mut self) {
        let Some(act) = self.hero.act.clone() else {
            return;
        };
        if self.hero.hitstop > 0 {
            return;
        }
        let t = self.hero.anim.tick;
        let (swing_tick, hits_now) = {
            let clip = &self.clips[&act.name];
            (clip.events.phases.map(|p| p[0]), clip.hits_at(t))
        };
        let spec = self.rules.moves[&act.name].clone();
        // the swing is heard whether it lands or not: the miss sound
        if swing_tick == Some(t) {
            let s = if act.name == "whirl" { "whirl" } else { "swing" };
            self.sound(s, 0, act.serial);
        }
        if !hits_now {
            return;
        }
        let reach = model::mm(spec.reach_mm);
        let half_arc = fx::deg(spec.arc_deg) / 2;
        let fwd = facing(self.hero.yaw, 0);
        let hero = self.hero.pos;
        let mut landed = Vec::new();
        for (i, e) in self.enemies.iter().enumerate() {
            if e.state == State::Dead || act.hit.contains(&e.id) {
                continue;
            }
            let to = e.pos.sub(hero);
            let d = to.len_xz() - e.radius(&self.rules);
            if d > reach || to.y.abs() > fx::mul(fx::int(2), e.scale(&self.rules)) {
                continue;
            }
            let ang = fx::wrap(yaw_of(to) - yaw_of(fwd)).abs();
            if spec.arc_deg < 360 && ang > half_arc && to.len_xz() > e.radius(&self.rules) {
                continue;
            }
            landed.push(i);
        }
        for i in landed {
            self.land(i, &act, &spec);
        }
    }

    fn land(&mut self, i: usize, act: &Act, spec: &MoveSpec) {
        let stun = self.rules.stun.clone();
        let cap = self.rules.hitstop_cap;
        let (push_ticks, shake_ticks) = (self.rules.push_ticks, self.rules.shake_ticks);
        let dm = self.models["dog"].clone();
        let combo_index = self.hero.combo.presses.len() as i32;
        let finishing = self
            .rules
            .combos
            .iter()
            .find(|k| k.presses == self.hero.combo.presses)
            .map(|k| k.name.clone());
        let e = &mut self.enemies[i];
        let espec = self.rules.enemies[&e.kind].clone();
        if let Some(a) = self.hero.act.as_mut() {
            a.hit.push(e.id);
        }
        let was_air = e.state == State::Airborne;
        // CS-4: an airborne enemy takes half again
        let dmg = if was_air { spec.damage * 3 / 2 } else { spec.damage };
        e.health -= dmg;
        let hs = spec.hitstop.min(cap);
        e.hitstop = hs;
        self.hero.hitstop = hs;
        // CS-9: the hit pushes the target back along the line from the hero
        let away = e.pos.sub(self.hero.pos);
        let away = if away.len_xz() == 0 {
            facing(self.hero.yaw, 0)
        } else {
            v3(away.x, 0, away.z).norm()
        };
        let push = model::mm(spec.push_mm) * espec.push_pct / 100;
        e.knock = away.scale(push / push_ticks as i32);
        e.knock_ticks = push_ticks;
        let mut died = false;
        let killable = match &espec.dies_on_combo {
            Some(c) => finishing.as_deref() == Some(c.as_str()) && spec.launches,
            None => true,
        };
        if killable && (e.health <= 0 || espec.dies_on_combo.is_some()) {
            died = true;
        } else if e.health <= 0 {
            e.health = 1;
        }
        let stunned_before = e.stunned_like();
        if died {
            e.state = State::Dead;
            e.anim.play(&self.clips, &dm, "dog_dead", 12);
            e.reaction = "dies";
        } else if spec.launches && stunned_before {
            // CS-5: only a stunned enemy is lifted
            e.state = State::Airborne;
            e.vel = v3(
                0,
                rise_speed(
                    per_tick2(self.rules.hero.gravity_mm_s2),
                    model::mm(stun.launch_mm),
                ),
                0,
            );
            e.float = 0;
            e.anim.play(&self.clips, &dm, "dog_tumble", 6);
            e.reaction = "launched";
        } else if was_air {
            e.float = stun.air_float;
            e.vel.y = e.vel.y.max(ONE / 100);
            e.reaction = "juggled";
        } else {
            // one hit stuns a dog (cue 39); a whirl stuns all it reaches
            let one_hit = espec.poise == 0 || spec.stuns;
            if one_hit || stunned_before {
                e.state = State::Stunned;
                e.stun = stun.window;
                e.in_string = !spec.ability;
            }
            if spec.stagger > espec.poise {
                e.anim.play(&self.clips, &dm, "dog_hurt", 6);
                e.reaction = "staggers";
            } else {
                e.shake = shake_ticks;
                e.reaction = "shakes";
            }
        }
        let e_id = e.id;
        let hit_name = format!("hit_{}", spec.hit);
        let rumble = format!(
            "{}_hit",
            if spec.hit == "launch" {
                "launcher"
            } else {
                spec.hit.as_str()
            }
        );
        // the combo rises a semitone a hit, at most five (sound review)
        self.sound(&hit_name, combo_index.clamp(0, 5), e_id);
        self.rumble(&rumble);
        let k = away.scale(KNOCK_MAX * spec.stagger / 2);
        self.knock = self.knock.add(k);
        self.events.push(Event {
            kind: "hit",
            name: act.name.clone(),
            a: e_id as i32,
            b: dmg,
        });
        if died {
            self.events.push(ev("die", &format!("{e_id}")));
        }
    }

    /// A lunging dog's bite lands on the hero within reach of its jaw.
    fn bites(&mut self) {
        let hp = self.hero.pos;
        let mut bitten = Vec::new();
        for e in self.enemies.iter_mut() {
            if e.state != State::Idle || e.anim.clip != "dog_lunge" || e.bit || e.hitstop > 0 {
                continue;
            }
            let c = &self.clips["dog_lunge"];
            if c.hits_at(e.anim.tick) {
                let reach = fx::mul(model::mm(1300), e.spec(&self.rules).scale_pct * ONE / 100);
                if e.pos.sub(hp).len_xz() <= reach {
                    e.bit = true;
                    bitten.push(e.spec(&self.rules).bite);
                }
            }
        }
        for b in bitten {
            self.hero.health -= b;
            self.hero.hitstop = 3;
            self.sound("bite", 0, 7);
            self.rumble("hurt");
            if self.hero.act.is_none() {
                self.hero.anim.play(&self.clips, &self.models["hero"], "hurt", 12);
            }
            self.events.push(Event {
                kind: "bitten",
                name: "hero".into(),
                a: b,
                b: self.hero.health,
            });
        }
    }

    // ------------------------------------------------------------ words

    /// What the tick did, in words, for the harness (M0.3): each body's
    /// clip, frame and phase, hit-stop, state, reaction and tree node.
    pub fn describe(&self) -> String {
        let h = &self.hero;
        let clip = &self.clips[&h.anim.clip];
        let mut s = format!(
            "t{:05} hero {}@{} {} hs{} stam{} hp{}{}{}",
            self.tick,
            h.anim.clip,
            h.anim.tick,
            if clip.events.phases.is_some() {
                clip.phase(h.anim.tick)
            } else {
                "-"
            },
            h.hitstop,
            h.stamina / 100,
            h.health,
            if h.combo.presses.is_empty() {
                String::new()
            } else {
                format!(" string[{}]", h.combo.presses.join(","))
            },
            if h.combo.recovery > 0 {
                format!(" recovery{}", h.combo.recovery)
            } else {
                String::new()
            },
        );
        for e in &self.enemies {
            s.push_str(&format!(
                " | {}{} {} {}@{} hs{}{} hp{} node:{}{}",
                e.kind,
                e.id,
                e.state.name(),
                e.anim.clip,
                e.anim.tick,
                e.hitstop,
                if e.shake > 0 {
                    format!(" shake{}", e.shake)
                } else {
                    String::new()
                },
                e.health,
                e.brain.node,
                if e.stun > 0 {
                    format!(" stun{}", e.stun)
                } else {
                    String::new()
                },
            ));
        }
        s
    }
}

impl World {
    /// One tick as data, for the journal's animations (CS-15): the same
    /// fields `describe` puts in words, plus each tree's lit nodes.
    pub fn snapshot(&self, events: &[Event]) -> serde_json::Value {
        let h = &self.hero;
        let clip = &self.clips[&h.anim.clip];
        serde_json::json!({
            "t": self.tick,
            "hero": {
                "clip": h.anim.clip, "frame": h.anim.tick, "len": clip.len,
                "phase": clip.phase(h.anim.tick), "hitstop": h.hitstop,
                "stamina": h.stamina, "health": h.health,
                "string": h.combo.presses, "recovery": h.combo.recovery,
                "y_mm": h.pos.y * 1000 / ONE, "grounded": h.grounded,
            },
            "enemies": self.enemies.iter().map(|e| {
                let tr = e.brain.tree.trace();
                serde_json::json!({
                    "id": e.id, "kind": e.kind, "state": e.state.name(),
                    "clip": e.anim.clip, "frame": e.anim.tick, "hitstop": e.hitstop,
                    "shake": e.shake, "stun": e.stun, "health": e.health,
                    "node": e.brain.node, "reaction": e.reaction,
                    "y_mm": e.pos.y * 1000 / ONE,
                    "active": tr.active, "held": tr.held, "failed": tr.failed,
                })
            }).collect::<Vec<_>>(),
            "events": events.iter().map(|e| serde_json::json!({"kind": e.kind, "name": e.name})).collect::<Vec<_>>(),
        })
    }
}

impl Default for World {
    fn default() -> Self {
        World::new()
    }
}

/// The press a tick's new buttons ask for, light first.
pub fn press_of(pressed: u32) -> Option<&'static str> {
    if pressed & B_LIGHT != 0 {
        Some("light")
    } else if pressed & B_HEAVY != 0 {
        Some("heavy")
    } else if pressed & B_LAUNCH != 0 {
        Some("launcher")
    } else {
        None
    }
}

/// The yaw that faces along a direction on the ground (0 faces -z).
pub fn yaw_of(d: V3) -> i32 {
    crate::geom::yaw_pitch_of(v3(d.x, 0, d.z)).0
}
