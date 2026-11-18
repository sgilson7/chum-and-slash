//! The text harness (M0.3): plays a scripted fight through `World::tick`,
//! the same function the page calls, and prints what each tick did.
//!
//!     cargo run -q --example harness -- data/scenarios/first_combo.txt
//!     cargo run -q --example harness -- data/scenarios/first_combo.txt --json out.json
//!
//! A script is one command a line: `spawn KIND X_MM Z_MM`, `tree INDEX NAME`,
//! `press TICK BUTTONS` (held for one tick), `hold FROM TO BUTTONS`,
//! `volley TICK`, `end TICK`. Buttons: light heavy launcher jump mod lock
//! sprint fwd back left right, joined with `+`. The hero faces -z.
//! Each output line: the tick, the hero's clip, frame, phase, hit-stop,
//! stamina, health and string; each enemy's state, clip, frame, hit-stop,
//! shake, health, tree node and stun; then the tick's events. This file
//! decides nothing: every word comes from the core (CLAUDE.md rule 2).

use std::fs;
use tool_core::geom::v3;
use tool_core::model::mm;
use tool_core::world::*;

fn bits(s: &str) -> u32 {
    s.split('+')
        .map(|b| match b {
            "light" => B_LIGHT,
            "heavy" => B_HEAVY,
            "launcher" => B_LAUNCH,
            "jump" => B_JUMP,
            "mod" => B_MOD,
            "lock" => B_LOCK,
            "sprint" => B_SPRINT,
            "fwd" => B_FWD,
            "back" => B_BACK,
            "left" => B_LEFT,
            "right" => B_RIGHT,
            "none" => 0,
            other => panic!("no button called {other}"),
        })
        .fold(0, |a, b| a | b)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("a scenario file");
    let json_out = args
        .iter()
        .position(|a| a == "--json")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let text = fs::read_to_string(path).expect("read the scenario");
    let mut w = World::new();
    let mut presses: Vec<(u32, u32)> = Vec::new();
    let mut holds: Vec<(u32, u32, u32)> = Vec::new();
    let mut volleys: Vec<u32> = Vec::new();
    let mut end = 600;
    for line in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let p: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| p[i].parse::<i32>().expect("a number");
        match p[0] {
            "spawn" => {
                w.spawn(p[1], v3(mm(n(2)), 0, mm(n(3))), 0).expect("spawn");
            }
            "tree" => {
                let i = n(1) as usize;
                w.enemies[i].brain.with_tree(p[2], i as u64);
            }
            "press" => presses.push((n(1) as u32, bits(p[2]))),
            "hold" => holds.push((n(1) as u32, n(2) as u32, bits(p[3]))),
            "volley" => volleys.push(n(1) as u32),
            "end" => end = n(1) as u32,
            other => panic!("unknown command {other}"),
        }
    }
    let mut steps = Vec::new();
    for t in 0..end {
        let mut b = presses
            .iter()
            .filter(|(at, _)| *at == t)
            .fold(0, |a, (_, b)| a | b);
        b |= holds
            .iter()
            .filter(|(f, to, _)| t >= *f && t <= *to)
            .fold(0, |a, (_, _, b)| a | b);
        if volleys.contains(&t) {
            w.pack.volley = true;
        }
        let ev = w.tick(Input::bits(b));
        let said: Vec<String> = ev.iter().map(|e| format!("{}:{}", e.kind, e.name)).collect();
        let line = format!(
            "{}{}",
            w.describe(),
            if said.is_empty() {
                String::new()
            } else {
                format!(" || {}", said.join(" "))
            }
        );
        println!("{line}");
        if json_out.is_some() {
            steps.push(w.snapshot(&ev));
        }
    }
    if let Some(out) = json_out {
        let doc = serde_json::json!({"made_by": format!("cargo run -q --example harness -- {path} --json {out}"), "scenario": path, "steps": steps});
        fs::write(&out, serde_json::to_string(&doc).unwrap()).expect("write json");
        eprintln!("wrote {out}");
    }
}
