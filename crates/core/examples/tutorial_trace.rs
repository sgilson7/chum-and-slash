//! The tutorial's slow, pause and prompt, traced tick by tick for the
//! journal (CS-15): Arena 1 from the end of its cutscene; the player waits
//! until the prompt has shown for a second, then presses the asked-for
//! button. Every value comes from the core's `Game`.
//!
//!     cargo run -q --example tutorial_trace -- out.json
use tool_core::fx::ONE;
use tool_core::game::Game;
use tool_core::tutorial::{bits_for, Phase};
use tool_core::world::Input;

fn main() {
    let out = std::env::args().nth(1).expect("an output path");
    let mut g = Game::new();
    g.act("arena", "0").unwrap();
    g.act("skip", "").unwrap();
    let mut steps = Vec::new();
    let mut paused_for = 0;
    for t in 0..2400 {
        let asking = g.tutorial.asking().map(str::to_string);
        let paused = matches!(g.tutorial.phase, Phase::Paused { .. });
        if paused {
            paused_for += 1;
        }
        let bits = if paused && paused_for == 60 { asking.as_deref().map(bits_for).unwrap_or(0) } else { 0 };
        g.step(Input::bits(bits));
        let phase = match g.tutorial.phase {
            Phase::Off => "off",
            Phase::Slowing(_) => "slowing",
            Phase::Holding(_) => "holding",
            Phase::Paused { .. } => "paused",
        };
        steps.push(serde_json::json!({
            "t": t, "world_tick": g.world.tick, "speed_pct": g.world.speed * 100 / ONE, "phase": phase,
            "asking": asking, "pressed": bits != 0, "hero_clip": g.world.hero.anim.clip,
            "dog_clip": g.world.enemies.first().map(|e| e.anim.clip.clone()),
        }));
        if paused_for > 60 && phase == "off" && steps.len() > 0 && g.world.hero.act.is_none() && t > 0 && paused_for > 120 {
            break;
        }
        if paused_for >= 60 && phase == "off" {
            paused_for += 1;
        }
    }
    let doc = serde_json::json!({"made_by": format!("cargo run -q --example tutorial_trace -- {out}"), "steps": steps});
    std::fs::write(&out, serde_json::to_string(&doc).unwrap()).unwrap();
    eprintln!("wrote {out} ({} ticks)", steps.len());
}
