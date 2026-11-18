//! Tutorials that teach through play (CS-12, M7.2; tutorials review).
//!
//! When a beat's moment comes, the game slows to a fifth of its speed over
//! 10 ticks, holds there for 30, then stops and names one button. Only that
//! input starts it again, and the same press performs the move, so the
//! player learns the move by doing it (cues 131-135). Wrong presses are
//! ignored; after three of them, or ten seconds, one line of text helps. A
//! beat once passed, by the prompt or by doing the move unprompted, never
//! comes back, even after a retry from the checkpoint. All the numbers here
//! are the tutorials review's estimates.

use crate::fx::ONE;
use crate::world::{Input, State, World, B_HEAVY, B_JUMP, B_LAUNCH, B_LIGHT, B_MOD};
use serde::Deserialize;
use std::collections::BTreeSet;

pub const SLOW: i32 = ONE / 5;
pub const RAMP: u32 = 10;
pub const HOLD: u32 = 30;
pub const HINT_WRONG: u32 = 3;
pub const HINT_TICKS: u32 = 600;

/// When a beat begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    /// a dog's lunge reaches the end of its crouch within 5 m
    DogLeaps,
    /// the fight has just begun
    Start,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Beat {
    pub id: String,
    /// the move to ask for: light, heavy, launcher, jump or whirl
    pub action: String,
    pub trigger: Trigger,
    /// the pack's volley this many ticks into the fight (Arena 2)
    #[serde(default)]
    pub volley_at: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Off,
    Slowing(u32),
    Holding(u32),
    Paused { age: u32, wrong: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tutorial {
    pub beats: Vec<Beat>,
    pub phase: Phase,
    pub current: Option<usize>,
    pub passed: BTreeSet<String>,
    /// the combo strip to show (Arena 3), as press names
    pub strip: Vec<String>,
    pub fight_t: u32,
}

/// The buttons an action needs: the Whirlwind Strike is the ability
/// modifier held with light (L1 + Square, CS-Q17).
pub fn bits_for(action: &str) -> u32 {
    match action {
        "light" => B_LIGHT,
        "heavy" => B_HEAVY,
        "launcher" => B_LAUNCH,
        "jump" => B_JUMP,
        "whirl" => B_MOD | B_LIGHT,
        _ => 0,
    }
}

/// The move a hero's act shows the beat was done unprompted.
fn done_by(action: &str, act: &str) -> bool {
    match action {
        "light" => act.starts_with("light"),
        "heavy" => act.starts_with("heavy"),
        "whirl" => act == "whirl",
        "launcher" => act == "launcher",
        _ => false,
    }
}

impl Tutorial {
    pub fn new(beats: Vec<Beat>, strip: Vec<String>, passed: BTreeSet<String>) -> Tutorial {
        Tutorial {
            beats,
            phase: Phase::Off,
            current: None,
            passed,
            strip,
            fight_t: 0,
        }
    }

    pub fn none() -> Tutorial {
        Tutorial::new(Vec::new(), Vec::new(), BTreeSet::new())
    }

    /// The action being asked for, while a prompt shows.
    pub fn asking(&self) -> Option<&str> {
        match (self.phase, self.current) {
            (Phase::Off, _) | (_, None) => None,
            (_, Some(i)) => Some(self.beats[i].action.as_str()),
        }
    }

    pub fn hint(&self) -> bool {
        matches!(self.phase, Phase::Paused { age, wrong } if wrong >= HINT_WRONG || age >= HINT_TICKS)
    }

    fn triggered(&self, b: &Beat, w: &World) -> bool {
        match b.trigger {
            Trigger::Start => true,
            Trigger::DogLeaps => w.enemies.iter().any(|e| {
                e.state == State::Idle
                    && e.anim.clip == "dog_lunge"
                    && e.anim.tick >= 16
                    && e.pos.sub(w.hero.pos).len_xz() <= crate::fx::mul(crate::fx::int(5), e.scale(&w.rules))
            }),
        }
    }

    /// One tick of the fight, with the tutorial in front: it may slow or
    /// pause the world, and while paused only the asked-for press reaches
    /// it. Returns the world's events.
    pub fn tick(&mut self, w: &mut World, input: Input, pressed: u32) -> Vec<crate::world::Event> {
        self.fight_t += 1;
        // a volley the beat has scheduled
        for b in &self.beats {
            if b.volley_at == Some(self.fight_t) && !self.passed.contains(&b.id) {
                w.pack.volley = true;
            }
        }
        // a beat done before its moment is passed for good
        if let Some(a) = w.hero.act.as_ref() {
            for b in &self.beats {
                if done_by(&b.action, &a.name) && self.phase == Phase::Off {
                    self.passed.insert(b.id.clone());
                }
            }
        }
        if self.phase == Phase::Off {
            let next = self
                .beats
                .iter()
                .position(|b| !self.passed.contains(&b.id) && self.triggered(b, w));
            if let Some(i) = next {
                self.current = Some(i);
                self.phase = Phase::Slowing(0);
            }
        }
        let want = self.current.map(|i| bits_for(&self.beats[i].action)).unwrap_or(0);
        let answered = want != 0 && (pressed & want) != 0 && (input.bits & want) == want;
        let mut events = Vec::new();
        match self.phase {
            Phase::Off => {
                w.speed = ONE;
                return w.tick(input);
            }
            Phase::Slowing(age) | Phase::Holding(age) if answered => {
                let _ = age;
            }
            Phase::Slowing(age) => {
                w.speed = ONE - (ONE - SLOW) * (age as i32 + 1) / RAMP as i32;
                self.phase = if age + 1 >= RAMP {
                    Phase::Holding(0)
                } else {
                    Phase::Slowing(age + 1)
                };
                return w.tick(input);
            }
            Phase::Holding(age) => {
                w.speed = SLOW;
                self.phase = if age + 1 >= HOLD {
                    Phase::Paused { age: 0, wrong: 0 }
                } else {
                    Phase::Holding(age + 1)
                };
                return w.tick(input);
            }
            Phase::Paused { age, wrong } if !answered => {
                w.speed = 0;
                let wrong = wrong + if pressed != 0 { 1 } else { 0 };
                self.phase = Phase::Paused { age: age + 1, wrong };
                // the world does not move, and remembers no press
                w.hero.prev_bits = input.bits;
                return events;
            }
            Phase::Paused { .. } => {}
        }
        // answered: time runs again and the same press performs the move
        if let Some(i) = self.current.take() {
            self.passed.insert(self.beats[i].id.clone());
            events.push(crate::world::Event {
                kind: "tutorial",
                name: self.beats[i].id.clone(),
                a: ONE,
                b: ONE,
            });
        }
        self.phase = Phase::Off;
        w.speed = ONE;
        w.speed_acc = 0;
        events.extend(w.tick(input));
        events
    }

    /// Where the hero is in the combo strip (Arena 3): how many of its
    /// presses the current string matches from the start.
    pub fn strip_at(&self, w: &World) -> usize {
        let p = &w.hero.combo.presses;
        if p.len() <= self.strip.len() && self.strip.iter().zip(p.iter()).all(|(a, b)| a == b) {
            p.len()
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::v3;
    use crate::model::mm;

    fn arena1() -> (World, Tutorial) {
        let mut w = World::new();
        w.spawn("dog", v3(0, 0, -mm(3500)), 0).unwrap();
        let beats = vec![Beat {
            id: "a1.light".into(),
            action: "light".into(),
            trigger: Trigger::DogLeaps,
            volley_at: None,
        }];
        (w, Tutorial::new(beats, Vec::new(), BTreeSet::new()))
    }

    fn until_paused(w: &mut World, t: &mut Tutorial) {
        for _ in 0..2000 {
            t.tick(w, Input::bits(0), 0);
            if matches!(t.phase, Phase::Paused { .. }) {
                return;
            }
        }
        panic!("the prompt never came");
    }

    /// CS-12: while paused, a wrong press changes nothing; the named press
    /// starts time again and performs its move on the same tick.
    #[test]
    fn prompt_resumes_only_on_its_input_and_performs_it() {
        let (mut w, mut t) = arena1();
        until_paused(&mut w, &mut t);
        assert_eq!(t.asking(), Some("light"));
        let tick = w.tick;
        t.tick(&mut w, Input::bits(B_HEAVY), B_HEAVY);
        t.tick(&mut w, Input::bits(0), 0);
        assert_eq!(w.tick, tick, "the world stays still");
        assert!(w.hero.act.is_none(), "the heavy did not start");
        t.tick(&mut w, Input::bits(B_LIGHT), B_LIGHT);
        assert_eq!(w.tick, tick + 1, "time runs again");
        assert_eq!(
            w.hero.act.as_ref().map(|a| a.name.as_str()),
            Some("light_1"),
            "the press performed the light attack"
        );
        assert_eq!(t.phase, Phase::Off);
    }

    /// The slow-down reaches a fifth of the speed in 10 ticks, holds 30,
    /// then stops.
    #[test]
    fn the_game_slows_then_holds_then_pauses() {
        let (mut w, mut t) = arena1();
        while !matches!(t.phase, Phase::Slowing(_)) {
            t.tick(&mut w, Input::bits(0), 0);
        }
        // the tick that began the slow-down took its first step
        for _ in 1..RAMP {
            t.tick(&mut w, Input::bits(0), 0);
        }
        assert_eq!(w.speed, SLOW);
        assert!(matches!(t.phase, Phase::Holding(0)));
        for _ in 0..HOLD {
            t.tick(&mut w, Input::bits(0), 0);
        }
        assert!(matches!(t.phase, Phase::Paused { .. }));
    }

    /// A passed beat never comes back, even after a retry.
    #[test]
    fn passed_prompt_never_returns() {
        let (mut w, mut t) = arena1();
        until_paused(&mut w, &mut t);
        t.tick(&mut w, Input::bits(B_LIGHT), B_LIGHT);
        let passed = t.passed.clone();
        let (mut w2, _) = arena1();
        let mut t2 = Tutorial::new(t.beats.clone(), Vec::new(), passed);
        for _ in 0..1500 {
            t2.tick(&mut w2, Input::bits(0), 0);
            assert_eq!(t2.phase, Phase::Off);
        }
    }

    /// Three wrong presses bring the line of help.
    #[test]
    fn three_wrong_presses_bring_help() {
        let (mut w, mut t) = arena1();
        until_paused(&mut w, &mut t);
        for _ in 0..HINT_WRONG {
            assert!(!t.hint());
            t.tick(&mut w, Input::bits(B_JUMP), B_JUMP);
            t.tick(&mut w, Input::bits(0), 0);
        }
        assert!(t.hint());
    }
}
