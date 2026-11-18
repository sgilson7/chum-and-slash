//! The game around the fight: screens, the prologue's four arenas with
//! their cutscenes, checkpoints and tutorials, the sandbox, input from keys,
//! mouse and pads, the camera, and what the page draws (M1, M5, M7).
//!
//! The page reports raw input and draws what this module returns; every
//! choice (which button is which action, when a cutscene ends, where the
//! camera sits) is made here.

use crate::fx::{self, ONE};
use crate::geom::{facing, v3};
use crate::model::{mm, rgb255};
use crate::render::{self, Batch, Camera, Finished};
use crate::scene::{self, Backdrop};
use crate::ticks::Clock;
use crate::tutorial::{Beat, Tutorial};
use crate::world::*;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const ARENAS_JSON: &str = include_str!("../../../data/arenas.json");
pub const PADS_JSON: &str = include_str!("../../../data/pads.json");
pub const LOOK_JSON: &str = include_str!("../../../data/look.json");
pub const TEXT_JSON: &str = include_str!("../../../data/text.json");

#[derive(Debug, Clone, Deserialize)]
pub struct SpawnSpec {
    pub kind: String,
    pub x_mm: i32,
    pub z_mm: i32,
    #[serde(default)]
    pub tree: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArenaSpec {
    pub id: String,
    pub title: String,
    pub lines: Vec<String>,
    pub spawn: Vec<SpawnSpec>,
    pub beats: Vec<Beat>,
    pub strip: Vec<String>,
    pub sky: [i32; 3],
    pub fog: [i32; 3],
    pub backdrop: Backdrop,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SandboxSpec {
    pub title: String,
    pub sky: [i32; 3],
    pub fog: [i32; 3],
    pub backdrop: Backdrop,
}

#[derive(Debug, Clone, Deserialize)]
struct ArenasFile {
    arenas: Vec<ArenaSpec>,
    sandbox: SandboxSpec,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pads {
    pub keys: BTreeMap<String, String>,
    pub pad: BTreeMap<String, String>,
    pub dead_zone_pct: i32,
    pub look_px_per_turn: i32,
    pub stick_look_deg: i32,
    pub skip_hold: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Look {
    pub light_dir: [i32; 3],
    pub light: [i32; 3],
    pub ink: [i32; 3],
    pub ink_size: [i32; 3],
    pub bands: [i32; 4],
    pub ambient: [i32; 3],
    pub ground: [i32; 3],
    pub fog_m: [i32; 2],
    pub stripe_count: i32,
    pub stripe_brown: [i32; 3],
    pub camera: CameraSpec,
}

/// The fight camera (T1): up and to the hero's side, far enough back for a
/// landscape view of the hero, the swing and the target.
#[derive(Debug, Clone, Deserialize)]
pub struct CameraSpec {
    pub distance_mm: i32,
    pub side_mm: i32,
    pub height_mm: i32,
    pub fov_deg: i32,
    pub lock_lean_pct: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Title,
    Menu,
    Cutscene,
    Fight,
    Won,
    Lost,
    Sandbox,
    Paused,
}

impl Screen {
    pub fn name(self) -> &'static str {
        match self {
            Screen::Title => "title",
            Screen::Menu => "menu",
            Screen::Cutscene => "cutscene",
            Screen::Fight => "fight",
            Screen::Won => "won",
            Screen::Lost => "lost",
            Screen::Sandbox => "sandbox",
            Screen::Paused => "paused",
        }
    }
}

/// The action a mapped name sets.
pub fn action_bit(name: &str) -> u32 {
    match name {
        "light" => B_LIGHT,
        "heavy" => B_HEAVY,
        "launcher" => B_LAUNCH,
        "jump" => B_JUMP,
        "mod" => B_MOD,
        "lock" => B_LOCK,
        "cycle" => B_CYCLE,
        "sprint" => B_SPRINT,
        "skip" => B_SKIP,
        "fwd" => B_FWD,
        "back" => B_BACK,
        "left" => B_LEFT,
        "right" => B_RIGHT,
        _ => 0,
    }
}

/// Which kind of pad an id names: Sony's vendor id or name means the
/// PlayStation buttons; anything else gets Xbox's.
pub fn pad_kind(id: &str) -> &'static str {
    let l = id.to_ascii_lowercase();
    if l.contains("054c") || l.contains("dualsense") || l.contains("playstation") {
        "playstation"
    } else {
        "xbox"
    }
}

/// Cutscene lines go on by themselves after this many ticks (6 s).
pub const LINE_TICKS: u32 = 360;

#[derive(Debug, Clone)]
pub struct Game {
    pub world: World,
    pub screen: Screen,
    paused_from: Screen,
    clock: Clock,
    pub arenas: Vec<ArenaSpec>,
    pub sandbox: SandboxSpec,
    pub pads: Pads,
    pub look: Look,
    pub text: BTreeMap<String, String>,
    pub arena: usize,
    pub in_sandbox: bool,
    pub line: usize,
    pub line_t: u32,
    pub skip_t: u32,
    pub checkpoint: Option<World>,
    pub tutorial: Tutorial,
    pub passed: BTreeSet<String>,
    keys: BTreeSet<String>,
    tapped: BTreeSet<String>,
    look_dx: i32,
    look_dy: i32,
    pad_buttons: Vec<bool>,
    pad_tapped: Vec<bool>,
    pad_axes: Vec<i32>,
    pub device: &'static str,
    prev_bits: u32,
    pending: Vec<Event>,
    pub alpha: i32,
    pub aspect: i32,
    pub scenery: Finished,
    pub scenery_version: u32,
    pub dynamic: Finished,
    sky: [i32; 3],
    fog: [i32; 3],
    pub fights_done: u32,
}

impl Default for Game {
    fn default() -> Self {
        Game::new()
    }
}

impl Game {
    pub fn new() -> Game {
        let file: ArenasFile = serde_json::from_str(ARENAS_JSON).expect("arenas.json is checked by its test");
        let pads: Pads = serde_json::from_str(PADS_JSON).expect("pads.json is checked by its test");
        let look: Look = serde_json::from_str(LOOK_JSON).expect("look.json is checked by its test");
        let text: BTreeMap<String, String> =
            serde_json::from_str::<BTreeMap<String, serde_json::Value>>(TEXT_JSON)
                .expect("text.json is checked by its test")
                .into_iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
                .collect();
        let sky = file.sandbox.sky;
        let fog = file.sandbox.fog;
        let mut g = Game {
            world: World::new(),
            screen: Screen::Title,
            paused_from: Screen::Fight,
            clock: Clock::default(),
            arenas: file.arenas,
            sandbox: file.sandbox,
            pads,
            look,
            text,
            arena: 0,
            in_sandbox: false,
            line: 0,
            line_t: 0,
            skip_t: 0,
            checkpoint: None,
            tutorial: Tutorial::none(),
            passed: BTreeSet::new(),
            keys: BTreeSet::new(),
            tapped: BTreeSet::new(),
            look_dx: 0,
            look_dy: 0,
            pad_buttons: Vec::new(),
            pad_tapped: Vec::new(),
            pad_axes: Vec::new(),
            device: "keyboard",
            prev_bits: 0,
            pending: Vec::new(),
            alpha: 0,
            aspect: ONE * 16 / 9,
            scenery: (Vec::new(), Vec::new()),
            scenery_version: 0,
            dynamic: (Vec::new(), Vec::new()),
            sky,
            fog,
            fights_done: 0,
        };
        let b = g.sandbox.backdrop.clone();
        g.build_scenery(&b);
        g
    }

    pub fn t(&self, key: &str) -> String {
        self.text.get(key).cloned().unwrap_or_else(|| key.to_string())
    }

    fn build_scenery(&mut self, b: &Backdrop) {
        let mut batch = Batch::new();
        scene::backdrop(b, &mut batch);
        self.scenery = batch.finish();
        self.scenery_version += 1;
        self.world.floor_radius = fx::int(b.floor_m);
    }

    // ------------------------------------------------------------ input

    pub fn key(&mut self, code: &str, down: bool) {
        if down {
            self.keys.insert(code.to_string());
            self.tapped.insert(code.to_string());
            self.device = "keyboard";
        } else {
            self.keys.remove(code);
        }
    }

    pub fn mouse(&mut self, dx: i32, dy: i32) {
        self.look_dx += dx;
        self.look_dy += dy;
    }

    /// A standard-mapped pad's buttons (pressed or not) and axes (Q16).
    pub fn pad(&mut self, buttons: Vec<bool>, axes: Vec<i32>, id: &str) {
        if buttons.iter().any(|b| *b) || axes.iter().any(|a| a.abs() > ONE / 2) {
            self.device = pad_kind(id);
        }
        let n = buttons.len().max(self.pad_tapped.len());
        let mut tapped = vec![false; n];
        for (i, b) in buttons.iter().enumerate() {
            let was = self.pad_buttons.get(i).copied().unwrap_or(false);
            tapped[i] = (*b && !was) || self.pad_tapped.get(i).copied().unwrap_or(false);
        }
        self.pad_tapped = tapped;
        self.pad_buttons = buttons;
        self.pad_axes = axes;
    }

    fn dead(&self, v: i32) -> i32 {
        let dz = ONE * self.pads.dead_zone_pct / 100;
        if v.abs() < dz {
            0
        } else {
            v
        }
    }

    /// The action bits held or tapped since the last frame.
    pub fn bits(&self) -> u32 {
        let mut b = 0;
        for k in self.keys.iter().chain(self.tapped.iter()) {
            if let Some(a) = self.pads.keys.get(k) {
                b |= action_bit(a);
            }
        }
        for (i, on) in self.pad_buttons.iter().enumerate() {
            if *on || self.pad_tapped.get(i).copied().unwrap_or(false) {
                if let Some(a) = self.pads.pad.get(&i.to_string()) {
                    b |= action_bit(a);
                }
            }
        }
        b
    }

    fn input(&mut self, bits: u32, first: bool) -> Input {
        let ax = |g: &Game, i: usize| g.dead(g.pad_axes.get(i).copied().unwrap_or(0));
        let turn_px = fx::TURN / self.pads.look_px_per_turn.max(1);
        let stick_look = fx::deg(self.pads.stick_look_deg);
        let (mut ly, mut lp) = (0, 0);
        if first {
            ly = -self.look_dx * turn_px;
            lp = -self.look_dy * turn_px;
            self.look_dx = 0;
            self.look_dy = 0;
        }
        ly -= fx::mul(ax(self, 2), stick_look);
        lp -= fx::mul(ax(self, 3), stick_look);
        Input {
            bits,
            stick_x: ax(self, 0),
            stick_z: -ax(self, 1),
            look_yaw: ly,
            look_pitch: lp,
        }
    }

    // ------------------------------------------------------------ time

    /// One display frame: runs the fixed ticks the time allows.
    pub fn frame(&mut self, now_us: i64) {
        let bits = self.bits();
        let ticks = self.clock.frame(now_us, bits);
        for (k, b) in ticks.iter().enumerate() {
            let input = self.input(*b, k == 0);
            self.step(input);
        }
        if !ticks.is_empty() {
            self.tapped.clear();
            self.pad_tapped.clear();
        }
        self.alpha = self.clock.alpha();
    }

    pub fn step(&mut self, input: Input) {
        let pressed = input.bits & !self.prev_bits;
        self.prev_bits = input.bits;
        match self.screen {
            Screen::Cutscene => self.cutscene_tick(input, pressed),
            Screen::Fight => {
                if pressed & B_SKIP != 0 {
                    self.paused_from = Screen::Fight;
                    self.screen = Screen::Paused;
                    return;
                }
                let ev = self.tutorial.tick(&mut self.world, input, pressed);
                self.keep(ev);
                if self.world.hero.health <= 0 {
                    self.screen = Screen::Lost;
                    self.pending.push(Event {
                        kind: "screen",
                        name: "lost".into(),
                        a: ONE,
                        b: ONE,
                    });
                } else if self.world.enemies.iter().all(|e| e.state == State::Dead)
                    && self.tutorial.fight_t > 60
                {
                    let dead_long = self.world.enemies.iter().all(|e| e.dead_for > 90);
                    if dead_long {
                        self.passed = self.tutorial.passed.clone();
                        self.fights_done += 1;
                        self.screen = Screen::Won;
                    }
                }
            }
            Screen::Sandbox => {
                if pressed & B_SKIP != 0 {
                    self.paused_from = Screen::Sandbox;
                    self.screen = Screen::Paused;
                    return;
                }
                let ev = self.world.tick(input);
                self.keep(ev);
                if self.world.hero.health <= 0 {
                    self.world.hero.health = self.world.rules.hero.health;
                }
                self.world.enemies.retain(|e| e.dead_for < 240);
            }
            _ => {
                // menus: the scene behind still turns slowly
                self.world.cam_yaw = fx::wrap(self.world.cam_yaw + fx::deg(1) / 8);
            }
        }
    }

    fn keep(&mut self, ev: Vec<Event>) {
        self.pending.extend(ev);
    }

    fn cutscene_tick(&mut self, _input: Input, pressed: u32) {
        self.world.cam_yaw = fx::wrap(self.world.cam_yaw + fx::deg(1) / 6);
        self.line_t += 1;
        if self.prev_bits & B_SKIP != 0 {
            self.skip_t += 1;
        } else {
            self.skip_t = 0;
        }
        let lines = self.arenas[self.arena].lines.len();
        if self.skip_t >= self.pads.skip_hold {
            self.begin_fight();
            return;
        }
        if pressed & B_LIGHT != 0 || self.line_t >= LINE_TICKS {
            self.line += 1;
            self.line_t = 0;
            if self.line >= lines {
                self.begin_fight();
            }
        }
    }

    // ------------------------------------------------------------ arenas

    fn fresh_world(&mut self, sky: [i32; 3], fog: [i32; 3], b: &Backdrop) {
        self.world = World::new();
        self.sky = sky;
        self.fog = fog;
        self.build_scenery(b);
        self.world.cam_yaw = fx::deg(b.city_deg);
    }

    pub fn begin_arena(&mut self, i: usize) {
        let i = i.min(self.arenas.len() - 1);
        self.arena = i;
        self.in_sandbox = false;
        let a = self.arenas[i].clone();
        self.fresh_world(a.sky, a.fog, &a.backdrop);
        for (k, s) in a.spawn.iter().enumerate() {
            if let Ok(id) = self.world.spawn(&s.kind, v3(mm(s.x_mm), 0, mm(s.z_mm)), fx::HALF) {
                if !s.tree.is_empty() {
                    if let Some(e) = self.world.enemies.iter_mut().find(|e| e.id == id) {
                        e.brain.with_tree(&s.tree, k as u64);
                    }
                }
            }
        }
        self.line = 0;
        self.line_t = 0;
        self.skip_t = 0;
        self.screen = Screen::Cutscene;
    }

    /// The checkpoint is taken here, just before the fight starts (cue 99).
    pub fn begin_fight(&mut self) {
        let a = &self.arenas[self.arena];
        self.world.cam_yaw = 0;
        self.checkpoint = Some(self.world.clone());
        self.tutorial = Tutorial::new(a.beats.clone(), a.strip.clone(), self.passed.clone());
        self.screen = Screen::Fight;
    }

    pub fn retry(&mut self) {
        if let Some(c) = self.checkpoint.clone() {
            self.world = c;
            let a = &self.arenas[self.arena];
            self.passed = self.passed.union(&self.tutorial.passed).cloned().collect();
            self.tutorial = Tutorial::new(a.beats.clone(), a.strip.clone(), self.passed.clone());
            self.screen = Screen::Fight;
        }
    }

    pub fn begin_sandbox(&mut self) {
        let s = self.sandbox.clone();
        self.fresh_world(s.sky, s.fog, &s.backdrop);
        self.in_sandbox = true;
        self.tutorial = Tutorial::none();
        self.screen = Screen::Sandbox;
    }

    /// Spawns an enemy 6 m ahead of where the camera looks, a little left,
    /// ahead or right in turn, so it comes into view.
    pub fn spawn_near(&mut self, kind: &str) -> Result<u32, String> {
        let n = self.world.next_id as i32 % 3 - 1;
        let a = fx::wrap(self.world.cam_yaw + fx::deg(30) * n);
        let p = self.world.hero.pos.add(facing(a, 0).scale(fx::int(6)));
        self.world.spawn(kind, v3(p.x, 0, p.z), 0)
    }

    pub fn act(&mut self, name: &str, value: &str) -> Result<(), String> {
        match name {
            "start" => self.screen = Screen::Menu,
            "prologue" => self.begin_arena(0),
            "sandbox" => self.begin_sandbox(),
            "arena" => self.begin_arena(value.parse::<usize>().map_err(|_| format!("no arena {value}"))?),
            "spawn" => {
                self.spawn_near(value)?;
            }
            "clear" => self.world.enemies.clear(),
            "retry" => self.retry(),
            "continue" => {
                if self.arena + 1 < self.arenas.len() {
                    self.begin_arena(self.arena + 1);
                } else {
                    self.screen = Screen::Menu;
                }
            }
            "menu" => {
                let s = self.sandbox.clone();
                self.fresh_world(s.sky, s.fog, &s.backdrop);
                self.screen = Screen::Menu;
            }
            "pause" => {
                if matches!(self.screen, Screen::Fight | Screen::Sandbox) {
                    self.paused_from = self.screen;
                    self.screen = Screen::Paused;
                }
            }
            "resume" => {
                if self.screen == Screen::Paused {
                    self.screen = self.paused_from;
                }
            }
            "skip" => {
                if self.screen == Screen::Cutscene {
                    self.begin_fight();
                }
            }
            "resize" => {
                let (w, h) = value.split_once('x').ok_or("resize takes WxH")?;
                let (w, h) = (
                    w.parse::<i32>().map_err(|e| e.to_string())?,
                    h.parse::<i32>().map_err(|e| e.to_string())?,
                );
                self.aspect = fx::div(fx::int(w.max(1)), fx::int(h.max(1)));
            }
            other => return Err(format!("no action called {other}")),
        }
        Ok(())
    }

    /// Sounds, rumbles and the rest since the page last asked.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.pending)
    }

    // ------------------------------------------------------------ the view

    pub fn camera(&self) -> Camera {
        let w = &self.world;
        let hp = w.hero.prev_pos.lerp(w.hero.pos, self.alpha);
        let chest = hp.add(v3(0, mm(1500), 0));
        if matches!(self.screen, Screen::Title | Screen::Menu | Screen::Cutscene) {
            // look out over the vista from the arena's edge
            let fwd = facing(w.cam_yaw, -fx::deg(4));
            let eye = v3(0, mm(2600), 0).sub(fwd.scale(fx::int(7)));
            return Camera {
                eye,
                fwd,
                fov: fx::deg(55),
                far: fx::int(16000),
                up: v3(0, ONE, 0),
            };
        }
        let c = &self.look.camera;
        // the pivot sits to the hero's right, so the hero stands left of
        // centre with the swing and the target in view (T1)
        let flat = facing(w.cam_yaw, 0);
        let right = v3(-flat.z, 0, flat.x);
        let pivot = chest.add(right.scale(mm(c.side_mm)));
        let mut look_at = pivot;
        if let Some(t) = w.hero.lock.and_then(|id| w.enemy(id)) {
            let tp = t.prev_pos.lerp(t.pos, self.alpha).add(v3(0, mm(800), 0));
            look_at = pivot.lerp(tp, ONE * c.lock_lean_pct / 100);
        }
        let back = facing(w.cam_yaw, w.cam_pitch);
        let knock = w.prev_knock.lerp(w.knock, self.alpha);
        let eye = pivot
            .sub(back.scale(mm(c.distance_mm)))
            .add(v3(0, mm(c.height_mm), 0))
            .add(knock);
        let fwd = look_at.sub(eye).norm();
        Camera {
            eye,
            fwd,
            fov: fx::deg(c.fov_deg),
            far: fx::int(16000),
            up: v3(0, ONE, 0),
        }
    }

    pub fn render(&mut self) {
        let mut b = Batch::new();
        scene::bodies(&self.world, self.alpha, &mut b);
        self.dynamic = b.finish();
    }

    pub fn draws(&self) -> Vec<u32> {
        render::draws(&self.scenery.1, &self.dynamic.1)
    }

    pub fn mats(&self) -> Vec<i32> {
        self.camera().mat_values(self.aspect).to_vec()
    }

    pub fn clear_color(&self) -> Vec<i32> {
        rgb255(self.sky).to_vec()
    }

    pub fn uniforms(&self) -> Vec<i32> {
        let l = &self.look;
        let pc = |v: i32| v * ONE / 100;
        let dir = v3(pc(l.light_dir[0]), pc(l.light_dir[1]), pc(l.light_dir[2])).norm();
        let mut out = vec![dir.x, dir.y, dir.z];
        out.extend_from_slice(&rgb255(self.sky));
        out.extend_from_slice(&rgb255(self.fog));
        out.extend_from_slice(&[fx::int(l.fog_m[0]), fx::int(l.fog_m[1])]);
        out.extend_from_slice(&rgb255(l.light));
        out.extend_from_slice(&rgb255(l.ink));
        out.extend_from_slice(&[pc(l.ink_size[0]), pc(l.ink_size[1]), pc(l.ink_size[2])]);
        out.extend_from_slice(&[pc(l.bands[0]), pc(l.bands[1]), pc(l.bands[2]), pc(l.bands[3])]);
        out.extend_from_slice(&[pc(l.ambient[0]), pc(l.ambient[1]), pc(l.ambient[2])]);
        out.extend_from_slice(&[pc(l.ground[0]), pc(l.ground[1]), pc(l.ground[2])]);
        // uQ: one Q16 step; uEps: a length too small to divide by
        out.push(1);
        out.push(ONE / 10_000);
        out.extend_from_slice(&[
            fx::int(render::PASS_SOLID as i32),
            fx::int(render::PASS_INK as i32),
            fx::int(render::PASS_TINT as i32),
            ONE / 2,
        ]);
        out.extend_from_slice(&[fx::int(crate::model::STYLE_GLOW), ONE / 2]);
        // uStripe: the corduroy's brown and how many wales round a leg (CS-20)
        let br = rgb255(l.stripe_brown);
        out.extend_from_slice(&[br[0], br[1], br[2], fx::int(l.stripe_count)]);
        out.extend_from_slice(&[fx::int(crate::model::STYLE_STRIPES), ONE / 2]);
        // uTau: a full turn in radians, for the stripes' angle
        out.push(411_775);
        out
    }

    // ------------------------------------------------------------ words

    fn button(&self, action: &str) -> String {
        let g: serde_json::Value =
            serde_json::from_str(include_str!("../../../data/glyphs.json")).unwrap_or_default();
        g[self.device][action].as_str().unwrap_or(action).to_string()
    }

    /// What the page shows, as a view model: the screen, its text, its
    /// buttons, the fight's gauges and any prompt.
    pub fn ui(&self) -> serde_json::Value {
        use serde_json::json;
        let w = &self.world;
        let btn =
            |label: &str, act: &str, value: &str| json!({"label": self.t(label), "act": act, "value": value});
        let mut out = json!({"screen": self.screen.name(), "device": self.device, "title": self.t("title")});
        match self.screen {
            Screen::Title => {
                out["lines"] = json!([self.t("tagline"), self.t("start_note")]);
                out["buttons"] = json!([btn("start", "start", "")]);
            }
            Screen::Menu => {
                out["lines"] = json!([self.t("menu.controls")]);
                let mut b = vec![
                    btn("menu.prologue", "prologue", ""),
                    btn("menu.sandbox", "sandbox", ""),
                ];
                for (i, a) in self.arenas.iter().enumerate() {
                    b.push(json!({"label": self.t(&a.title), "act": "arena", "value": i.to_string()}));
                }
                out["buttons"] = json!(b);
            }
            Screen::Cutscene => {
                let a = &self.arenas[self.arena];
                out["heading"] = json!(self.t(&a.title));
                out["lines"] = json!([self.t(&a.lines[self.line.min(a.lines.len() - 1)])]);
                out["skip"] = json!({
                    "label": format!("{}: {}", self.button("skip"), self.t("skip")),
                    "next": format!("{}: {}", self.button("light"), self.t("next")),
                    "pct": self.skip_t * 100 / self.pads.skip_hold.max(1),
                });
            }
            Screen::Fight | Screen::Sandbox => {
                let h = &w.hero;
                out["hud"] = json!({
                    "health": h.health.max(0) * 100 / w.rules.hero.health,
                    "stamina": h.stamina * 100 / w.rules.stamina.max,
                    "health_label": self.t("hud.health"),
                    "stamina_label": self.t("hud.stamina"),
                    "lock": h.lock.is_some(),
                    "lock_label": self.t("hud.lock"),
                    "recovery": h.combo.recovery > 0,
                });
                if self.in_sandbox {
                    out["buttons"] = json!([
                        btn("sandbox.spawn_dog", "spawn", "dog"),
                        btn("sandbox.spawn_pup", "spawn", "pup"),
                        btn("sandbox.spawn_big", "spawn", "big_dog"),
                        btn("sandbox.clear", "clear", ""),
                        btn("menu", "menu", ""),
                    ]);
                }
                if let Some(a) = self.tutorial.asking() {
                    out["prompt"] = json!({
                        "action": a,
                        "label": self.t(&format!("action.{a}")),
                        "button": self.button(a),
                        "paused": matches!(self.tutorial.phase, crate::tutorial::Phase::Paused { .. }),
                        "hint": if self.tutorial.hint() { self.t("hint") } else { String::new() },
                    });
                }
                if !self.tutorial.strip.is_empty() && self.screen == Screen::Fight {
                    let at = self.tutorial.strip_at(w);
                    let broken = at == 0 && !w.hero.combo.presses.is_empty();
                    out["strip"] = json!({
                        "items": self.tutorial.strip.iter().enumerate().map(|(i, p)| json!({
                            "label": self.t(&format!("action.{p}")), "button": self.button(p),
                            "done": i < at, "next": i == at,
                        })).collect::<Vec<_>>(),
                        "note": if broken { self.t("strip.again") } else { String::new() },
                    });
                }
            }
            Screen::Won => {
                let last = self.arena + 1 >= self.arenas.len();
                out["lines"] = json!([self.t(if last { "won.last" } else { "won" })]);
                out["buttons"] = json!([btn("continue", "continue", ""), btn("menu", "menu", "")]);
            }
            Screen::Lost => {
                out["lines"] = json!([self.t("lost")]);
                out["buttons"] = json!([btn("retry", "retry", ""), btn("menu", "menu", "")]);
            }
            Screen::Paused => {
                out["heading"] = json!(self.t("paused"));
                // T3: Escape frees the mouse and pauses, so the sandbox's
                // spawn tools sit in the pause card where a free mouse can reach them.
                let mut buttons = vec![btn("resume", "resume", "")];
                if self.paused_from == Screen::Sandbox {
                    buttons.extend([
                        btn("sandbox.spawn_dog", "spawn", "dog"),
                        btn("sandbox.spawn_pup", "spawn", "pup"),
                        btn("sandbox.spawn_big", "spawn", "big_dog"),
                        btn("sandbox.clear", "clear", ""),
                    ]);
                }
                buttons.push(btn("menu", "menu", ""));
                out["buttons"] = json!(buttons);
            }
        }
        out
    }

    /// The state the browser check and the playtest runners read.
    pub fn probe(&self) -> serde_json::Value {
        serde_json::json!({
            "screen": self.screen.name(),
            "tick": self.world.tick,
            "enemies": self.world.enemies.len(),
            "alive": self.world.enemies.iter().filter(|e| e.state != State::Dead).count(),
            "hero_clip": self.world.hero.anim.clip,
            "device": self.device,
            "arena": self.arena,
            "instances": self.dynamic.0.len() / render::INSTANCE_VALUES + self.scenery.0.len() / render::INSTANCE_VALUES,
        })
    }
}

/// Every stride and offset the page needs to upload and draw, so `gl.js`
/// holds no layout of its own (after Protection of the Spheres).
pub fn numbers() -> String {
    use crate::mesh::VERTEX_VALUES;
    use crate::render::*;
    let b = BYTES_PER_VALUE;
    serde_json::json!({
        "vertex_values": VERTEX_VALUES,
        "vertex": {"stride": VERTEX_VALUES * b, "attributes": [["aPos", 3, 0], ["aNormal", 3, 3 * b]]},
        "instance": {"stride": INSTANCE_VALUES * b, "attributes": [["iA", 4, 0], ["iB", 4, 4 * b], ["iC", 4, 8 * b], ["iD", 4, 12 * b]]},
        "buffers": {"scenery": BUF_SCENERY, "dynamic": BUF_DYNAMIC},
        "passes": {"solid": PASS_SOLID, "ink": PASS_INK, "tint": PASS_TINT},
        "draw_values": DRAW_VALUES,
        "mat_values": MAT_VALUES,
        "view_values": 16,
        "mesh_count": MESH_COUNT,
        "uniforms": [["uLightDir", 3], ["uSky", 3], ["uFog", 3], ["uFogRange", 2], ["uLight", 3], ["uInk", 3], ["uInkSize", 3], ["uBands", 4], ["uAmbient", 3], ["uGround", 3], ["uQ", 1], ["uEps", 1], ["uPasses", 4], ["uGlow", 2], ["uStripe", 4], ["uStripeStyle", 2], ["uTau", 1]],
        "ui_every_ms": 100,
        "noise_samples": 48000,
    })
    .to_string()
}

/// White noise for the page's sounds, from a fixed seed, as Q16 in
/// [-1, 1]: the core makes the randomness so the page needs none (D3).
pub fn noise(n: usize) -> Vec<i32> {
    let mut r = crate::rng::Rng::new(0x5EED);
    (0..n).map(|_| r.between(-ONE, ONE)).collect()
}

/// A vertex list for one mesh, for the page to upload once.
pub fn mesh(id: u32) -> Vec<i32> {
    crate::mesh::Mesh::ALL
        .get(id as usize)
        .map(|m| crate::mesh::vertices(*m))
        .unwrap_or_default()
}

/// `{"ok": true}` or `{"error": "..."}`, the template's convention.
pub fn answer(r: Result<(), String>) -> String {
    match r {
        Ok(()) => "{\"ok\":true}".to_string(),
        Err(e) => serde_json::json!({ "error": e }).to_string(),
    }
}

/// The events as JSON for the page: kind, name, and a sound's rate and
/// gain as Q16 (the page multiplies by its one Q16 step).
pub fn events_json(ev: &[Event]) -> String {
    serde_json::to_string(
        &ev.iter()
            .map(|e| serde_json::json!({"kind": e.kind, "name": e.name, "rate": e.a, "gain": e.b}))
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| "[]".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_files_load() {
        let g = Game::new();
        assert_eq!(g.arenas.len(), 4, "the prologue's four arenas");
        assert!(g.text.contains_key("title"));
    }

    /// A PS5 DualSense and an Xbox pad name their buttons by their own
    /// glyphs (accessibility: prompts name the button for the device).
    #[test]
    fn a_pad_is_known_by_its_id() {
        assert_eq!(pad_kind("054c-0ce6-DualSense Wireless Controller"), "playstation");
        assert_eq!(
            pad_kind("Xbox Wireless Controller (STANDARD GAMEPAD Vendor: 045e)"),
            "xbox"
        );
    }

    /// CS-14: standard-mapping pad buttons map to actions with no set-up.
    #[test]
    fn pad_buttons_map_to_actions() {
        let mut g = Game::new();
        let mut b = vec![false; 17];
        b[2] = true; // Square / X
        b[4] = true; // L1 / LB
        g.pad(b, vec![0; 4], "054c DualSense");
        assert_eq!(g.bits() & (B_LIGHT | B_MOD), B_LIGHT | B_MOD);
        assert_eq!(g.device, "playstation");
        let mut b = vec![false; 17];
        b[3] = true; // Triangle / Y
        g.pad(b, vec![0; 4], "xbox");
        assert_ne!(g.bits() & B_HEAVY, 0);
    }

    /// T3: Escape pauses and frees the mouse, so the sandbox's pause card
    /// carries the spawn tools; spawning there keeps the game paused, and
    /// the arena's pause card does not offer them.
    #[test]
    fn sandbox_pause_card_spawns() {
        let mut g = Game::new();
        g.act("sandbox", "").unwrap();
        g.act("pause", "").unwrap();
        let ui = g.ui();
        let acts: Vec<&str> = ui["buttons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["act"].as_str().unwrap())
            .collect();
        assert_eq!(acts, ["resume", "spawn", "spawn", "spawn", "clear", "menu"]);
        let before = g.world.enemies.len();
        g.act("spawn", "dog").unwrap();
        assert_eq!(g.world.enemies.len(), before + 1);
        assert_eq!(g.screen, Screen::Paused);
        g.act("resume", "").unwrap();
        assert_eq!(g.screen, Screen::Sandbox);
        g.act("arena", "0").unwrap();
        g.act("skip", "").unwrap();
        g.act("pause", "").unwrap();
        assert_eq!(g.ui()["buttons"].as_array().unwrap().len(), 2);
    }

    /// T1, worked by hand: at the origin facing -z (camera yaw 0, pitch 0
    /// for the check), the pivot is 1.2 m to the right (+x) and the eye is
    /// 6 m behind it (+z) and 1.5 m above the 1.5 m chest.
    #[test]
    fn the_camera_sits_up_and_to_the_side() {
        let mut g = Game::new();
        g.act("sandbox", "").unwrap();
        g.world.cam_yaw = 0;
        g.world.cam_pitch = 0;
        let cam = g.camera();
        assert_eq!(cam.eye.x, mm(1200));
        assert_eq!(cam.eye.z, mm(6000));
        assert_eq!(cam.eye.y, mm(1500) + mm(1500));
    }

    /// T1: the mouse or right stick turns the camera even while locked on;
    /// lock-on steers it back only after `camera_follow_after` quiet ticks.
    #[test]
    fn look_input_holds_the_camera_against_lock_on() {
        let mut g = Game::new();
        g.act("sandbox", "").unwrap();
        g.world.spawn("dog", v3(fx::int(8), 0, 0), 0).unwrap();
        g.world.cam_yaw = 0;
        let turned = fx::deg(-40);
        g.step(Input {
            look_yaw: turned,
            ..Input::default()
        });
        let after = g.world.cam_yaw;
        let quiet = g.world.rules.camera_follow_after;
        for _ in 0..quiet - 2 {
            g.step(Input::default());
        }
        assert_eq!(g.world.cam_yaw, after, "the player's turn holds");
        for _ in 0..60 {
            g.step(Input::default());
        }
        assert_ne!(g.world.cam_yaw, after, "then lock-on brings the target back");
    }

    fn fight(g: &mut Game) {
        g.act("arena", "0").unwrap();
        g.act("skip", "").unwrap();
        assert_eq!(g.screen, Screen::Fight);
    }

    /// CS-13: a checkpoint just before each fight; a retry puts the world
    /// back to it.
    #[test]
    fn checkpoint_before_each_fight() {
        for i in 0..g_arenas() {
            let mut g = Game::new();
            g.act("arena", &i.to_string()).unwrap();
            assert!(g.checkpoint.is_none());
            g.act("skip", "").unwrap();
            let cp = g.checkpoint.as_ref().expect("taken at the start of the fight");
            assert_eq!(cp.tick, g.world.tick, "taken just before the fight");
            assert_eq!(cp.enemies.len(), g.arenas[i].spawn.len());
        }
        let mut g = Game::new();
        fight(&mut g);
        for _ in 0..200 {
            g.step(Input::bits(0));
        }
        g.world.hero.health = 0;
        g.step(Input::bits(0));
        assert_eq!(g.screen, Screen::Lost);
        g.act("retry", "").unwrap();
        assert_eq!(g.world.tick, g.checkpoint.as_ref().unwrap().tick);
        assert_eq!(g.world.hero.health, g.world.rules.hero.health);
    }

    fn g_arenas() -> usize {
        Game::new().arenas.len()
    }

    /// Arena 4 (Sam, 2026-11-18): four dogs and one large dog, all in the
    /// fight at once, and the large one fights back (unlike Arena 3's
    /// injured dog).
    #[test]
    fn arena4_is_four_dogs_and_a_large_dog_at_once() {
        let mut g = Game::new();
        g.act("arena", "3").unwrap();
        g.act("skip", "").unwrap();
        let mut kinds: Vec<&str> = g.world.enemies.iter().map(|e| e.kind.as_str()).collect();
        kinds.sort();
        assert_eq!(kinds, ["dog", "dog", "dog", "dog", "large_dog"]);
        let big = g.world.enemies.iter().find(|e| e.kind == "large_dog").unwrap();
        assert_ne!(
            big.state,
            crate::world::State::Stunned,
            "the large dog is not lying injured"
        );
        let spec = &g.world.rules.enemies["large_dog"];
        assert!(spec.attacks && !spec.injured && spec.bite > 0);
        assert!(spec.scale_pct > g.world.rules.enemies["dog"].scale_pct);
        // all five are in lock range from the start, so all join at once
        let range = mm(g.world.rules.hero.lock_range_mm);
        for e in &g.world.enemies {
            assert!(
                e.pos.sub(g.world.hero.pos).len_xz() < range,
                "{} starts out of range",
                e.kind
            );
        }
    }

    /// CS-13: every cutscene can be skipped by holding skip.
    #[test]
    fn every_cutscene_skips() {
        for i in 0..g_arenas() {
            let mut g = Game::new();
            g.act("arena", &i.to_string()).unwrap();
            let hold = g.pads.skip_hold;
            for _ in 0..hold {
                g.step(Input::bits(B_SKIP));
            }
            assert_eq!(g.screen, Screen::Fight, "arena {i}");
        }
    }

    /// A scripted player clears each arena (M7.3): it locks on and strikes
    /// whenever it can, answering each tutorial prompt with its button.
    #[test]
    fn bot_clears_each_arena() {
        for i in 0..g_arenas() {
            let mut g = Game::new();
            g.act("arena", &i.to_string()).unwrap();
            g.act("skip", "").unwrap();
            let strip: Vec<u32> = g.arenas[i]
                .strip
                .iter()
                .map(|p| crate::tutorial::bits_for(p))
                .collect();
            let mut k = 0usize;
            for t in 0..9000 {
                if g.screen != Screen::Fight {
                    break;
                }
                let ask = g.tutorial.asking().map(crate::tutorial::bits_for);
                let ready = match &g.world.hero.act {
                    None => true,
                    Some(a) => {
                        let c = &g.world.clips[&a.name];
                        c.events.cancel.is_some_and(|x| g.world.hero.anim.tick >= x)
                            || c.done(g.world.hero.anim.tick)
                    }
                };
                let near = g.world.hero.lock.and_then(|id| g.world.enemy(id)).map(|e| {
                    e.pos.sub(g.world.hero.pos).len_xz()
                        < fx::mul(fx::int(3), e.scale(&g.world.rules)) + fx::int(1)
                });
                let mut bits = 0;
                if let Some(b) = ask {
                    if t % 2 == 0 {
                        bits = b;
                    }
                } else if !strip.is_empty() {
                    if near == Some(true) && ready && t % 2 == 0 && g.world.hero.combo.recovery == 0 {
                        k = g.tutorial.strip_at(&g.world);
                        bits = strip[k % strip.len()];
                    }
                } else if near == Some(true) && ready && t % 2 == 0 {
                    bits = if g.world.hero.combo.presses.len() % 3 == 2 {
                        B_HEAVY
                    } else {
                        B_LIGHT
                    };
                } else if near != Some(true) {
                    bits = B_FWD;
                }
                g.step(Input::bits(bits));
            }
            assert_eq!(g.screen, Screen::Won, "arena {i}: {}", g.world.describe());
            let _ = k;
        }
    }
}
