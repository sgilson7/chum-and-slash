//! What the page draws: the bodies each frame (between the last two ticks,
//! M1.1) and the arena's backdrop once (M7.1).
//!
//! The backdrop is rings round the arena (CS-Q24; vistas review): the park on
//! its hilltop and trees (to 150 m), the lake with the skyline's reflection
//! (to 2.1 km), the city across the water (2.1 to 5 km), and hills (8 to
//! 15 km), all fading into the haze. The skyline's placement is Long Time
//! Comin's `arena.rs` `skyline` (MIT, Sam Gilson): rings of (inner, outer,
//! count), each building jittered ±4° and set at a random radius, its height
//! growing with distance, a third of them with a narrower tower on top; here
//! scaled out to kilometres and kept to the arc across the lake.

use crate::fx::{self, ONE};
use crate::geom::{v3, Affine, V3};
use crate::mesh::Mesh;
use crate::model::{mm, rgb255, STYLE_PLAIN};
use crate::render::{standing, Batch, Rgb, LAYER_INKED, LAYER_PLAIN};
use crate::rng::Rng;
use crate::world::{State, World};

pub fn m(n: i32) -> i32 {
    fx::int(n)
}

fn lerp_v(a: V3, b: V3, t: i32) -> V3 {
    a.lerp(b, t)
}

/// A body's place for drawing: between the last tick and this one.
fn root(prev: V3, now: V3, prev_yaw: i32, yaw: i32, alpha: i32, lift: i32) -> Affine {
    let p = lerp_v(prev, now, alpha);
    let y = fx::lerp_angle(prev_yaw, yaw, alpha);
    Affine::translate(v3(p.x, p.y + lift, p.z)).then(&Affine::rot_y(y))
}

fn pose_between(a: &[[i32; 3]], b: &[[i32; 3]], alpha: i32) -> Vec<[i32; 3]> {
    a.iter()
        .zip(b.iter())
        .map(|(p, q)| [fx::lerp_angle(p[0], q[0], alpha), fx::lerp_angle(p[1], q[1], alpha), fx::lerp_angle(p[2], q[2], alpha)])
        .collect()
}

/// The bodies, drawn between the last two ticks at `alpha` (Q16).
pub fn bodies(w: &World, alpha: i32, out: &mut Batch) {
    let hm = &w.models["hero"];
    let now = w.hero.anim.pose(&w.clips, hm);
    let pose = pose_between(&w.hero.prev_pose, &now, alpha);
    let r = root(w.hero.prev_pos, w.hero.pos, w.hero.prev_yaw, w.hero.yaw, alpha, 0);
    for (i, a) in hm.parts_at(&r, &pose, ONE) {
        let p = &hm.parts[i];
        out.push(LAYER_INKED, p.mesh, &a, p.rgb, p.style);
    }
    let dm = &w.models["dog"];
    for e in &w.enemies {
        let now = e.anim.pose(&w.clips, dm);
        let pose = pose_between(&e.prev_pose, &now, alpha);
        // a hit that does not stagger shakes the body in place (cue 123)
        let shake = if e.shake > 0 { if e.shake % 2 == 0 { mm(25) } else { -mm(25) } } else { 0 };
        let c = &w.clips[&e.anim.clip];
        let (_, up) = c.root_at(e.anim.tick);
        let scale = e.scale(&w.rules);
        let lift = fx::mul(up, scale);
        let r = root(e.prev_pos, e.pos, e.prev_yaw, e.yaw, alpha, lift).then(&Affine::translate(v3(shake, 0, 0)));
        let dim = e.state == State::Dead;
        for (i, a) in dm.parts_at(&r, &pose, scale) {
            let p = &dm.parts[i];
            let rgb = if dim { [p.rgb[0] / 2, p.rgb[1] / 2, p.rgb[2] / 2] } else { p.rgb };
            out.push(LAYER_INKED, p.mesh, &a, rgb, if dim { STYLE_PLAIN } else { p.style });
        }
    }
    // the lock-on mark: a ring on the ground under the target
    if let Some(t) = w.hero.lock.and_then(|id| w.enemy(id)) {
        let s = t.radius(&w.rules) * 2 + mm(300);
        let p = lerp_v(t.prev_pos, t.pos, alpha);
        let a = Affine::translate(v3(p.x, mm(20), p.z)).then(&Affine::scale(v3(s, mm(30), s)));
        out.tint(Mesh::Ring, &a, rgb255([255, 238, 120]), ONE * 3 / 4);
    }
}

/// An arena's backdrop recipe (data/arenas.json).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Backdrop {
    pub seed: u64,
    /// which way the city lies across the lake, degrees
    pub city_deg: i32,
    /// the arc the city fills, degrees either side
    pub city_arc_deg: i32,
    pub grass: [i32; 3],
    pub hill: [i32; 3],
    pub water: [i32; 3],
    pub towers: Vec<[i32; 3]>,
    pub far_hills: [i32; 3],
    pub trees: [i32; 3],
    pub leaves: [i32; 3],
    /// the park's floor radius, m
    pub floor_m: i32,
    /// how far below the park the lake lies, m
    pub drop_m: i32,
}

fn shade(c: Rgb, pct: i32) -> Rgb {
    [c[0] * pct / 100, c[1] * pct / 100, c[2] * pct / 100]
}

/// The backdrop, built once per arena.
pub fn backdrop(b: &Backdrop, out: &mut Batch) {
    let mut rng = Rng::new(b.seed);
    let grass = rgb255(b.grass);
    let hill = rgb255(b.hill);
    let drop = m(b.drop_m);
    // ring 0: the park on its hilltop, the hill falling away to the lake
    out.push(LAYER_PLAIN, Mesh::Cyl, &Affine::translate(v3(0, -m(1) / 2, 0)).then(&Affine::scale(v3(m(b.floor_m * 2 + 6), m(1), m(b.floor_m * 2 + 6)))), grass, STYLE_PLAIN);
    out.push(LAYER_PLAIN, Mesh::Cone, &Affine::translate(v3(0, -m(1) - drop / 2, 0)).then(&Affine::scale(v3(m(320), drop, m(320)))), hill, STYLE_PLAIN);
    let trunk = rgb255(b.trees);
    let leaves = rgb255(b.leaves);
    for k in 0..28 {
        let a = fx::TURN / 28 * k + fx::deg(rng.between(-5, 5));
        // keep the view toward the city open
        if fx::wrap(a - fx::deg(b.city_deg)).abs() < fx::deg(b.city_arc_deg) {
            continue;
        }
        let r = m(b.floor_m + 4) + m(rng.between(0, 30));
        let (x, z) = (fx::mul(fx::cos(a), r), fx::mul(fx::sin(a), r));
        let ground = -fx::mul(fx::div(r - m(b.floor_m), m(160)), drop).min(0).abs();
        let h = m(rng.between(5, 9));
        out.push(LAYER_INKED, Mesh::Cyl, &standing(v3(x, -ground, z), v3(m(1) / 2, h, m(1) / 2)), trunk, STYLE_PLAIN);
        let s = m(rng.between(4, 7));
        out.push(LAYER_INKED, Mesh::Facet, &Affine::translate(v3(x, h - ground, z)).then(&Affine::scale(v3(s, s * 4 / 5, s))), leaves, STYLE_PLAIN);
    }
    // ring 1: the lake, below the hill
    let water = rgb255(b.water);
    out.push(LAYER_PLAIN, Mesh::Cyl, &Affine::translate(v3(0, -drop - m(1), 0)).then(&Affine::scale(v3(m(9000), m(1), m(9000)))), water, STYLE_PLAIN);
    // ring 2: the skyline across the water (Long Time Comin's rings)
    let city = fx::deg(b.city_deg);
    let arc = fx::deg(b.city_arc_deg);
    let rings = [(m(2100), m(2600), 40), (m(2600), m(3600), 36), (m(3600), m(5000), 30)];
    for (lo, hi, n) in rings {
        for k in 0..n {
            let a = city - arc + (2 * arc / n) * k + fx::deg(rng.between(-4, 4));
            let r = lo + m(rng.between(0, (hi - lo) / ONE));
            let (x, z) = (fx::mul(fx::sin(a), r), -fx::mul(fx::cos(a), r));
            // downtown rises toward the middle of the arc (Pursuer's rule)
            let centre = ONE - fx::div(fx::wrap(a - city).abs(), arc).min(ONE);
            let w = m(rng.between(25, 60));
            let d = m(rng.between(25, 60));
            let h = m(rng.between(30, 90)) + fx::mul(centre, m(rng.between(60, 200))) + (r - lo) / 12;
            let tone = b.towers[rng.below(b.towers.len() as u32) as usize];
            let c = rgb255(tone);
            out.push(LAYER_PLAIN, Mesh::Cube, &standing(v3(x, -drop, z), v3(w, h, d)), c, STYLE_PLAIN);
            // its reflection in the lake, darker, upside down
            out.push(LAYER_PLAIN, Mesh::Cube, &standing(v3(x, -drop - h, z), v3(w, h, d)), shade(c, 55), STYLE_PLAIN);
            if rng.chance(30, 100) {
                let extra = m(rng.between(20, 80));
                out.push(LAYER_PLAIN, Mesh::Cube, &standing(v3(x, -drop + h, z), v3(w * 6 / 10, extra, d * 6 / 10)), shade(c, 90), STYLE_PLAIN);
            }
        }
    }
    // the water over the reflections: a sheet just above them
    out.tint(Mesh::Cyl, &Affine::translate(v3(0, -drop - m(1) / 2, 0)).then(&Affine::scale(v3(m(9000), m(1) / 4, m(9000)))), water, ONE * 7 / 10);
    // ring 3: far hills all round
    let far = rgb255(b.far_hills);
    for k in 0..18 {
        let a = fx::TURN / 18 * k + fx::deg(rng.between(-6, 6));
        let r = m(rng.between(8000, 15000));
        let (x, z) = (fx::mul(fx::sin(a), r), -fx::mul(fx::cos(a), r));
        let s = m(rng.between(2500, 5000));
        let h = m(rng.between(400, 1100));
        out.push(LAYER_PLAIN, Mesh::Cone, &standing(v3(x, -drop, z), v3(s, h, s)), far, STYLE_PLAIN);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe() -> Backdrop {
        Backdrop {
            seed: 1,
            city_deg: 0,
            city_arc_deg: 50,
            grass: [80, 140, 70],
            hill: [60, 110, 60],
            water: [40, 90, 120],
            towers: vec![[200, 200, 210]],
            far_hills: [120, 140, 170],
            trees: [90, 60, 40],
            leaves: [70, 150, 70],
            floor_m: 24,
            drop_m: 40,
        }
    }

    /// M7.1: the backdrop stays inside the budget the vistas review set
    /// for software renderers: under 400 instances (it costs per instance).
    #[test]
    fn backdrop_stays_within_budget() {
        let mut b = Batch::new();
        backdrop(&recipe(), &mut b);
        let n = b.count();
        assert!(n < 400, "{n} instances");
        assert!(n > 150, "{n} instances: the skyline is there");
    }

    /// The same seed draws the same backdrop (a still view, the same pixels).
    #[test]
    fn the_backdrop_is_the_same_every_time() {
        let (mut a, mut b) = (Batch::new(), Batch::new());
        backdrop(&recipe(), &mut a);
        backdrop(&recipe(), &mut b);
        assert_eq!(a.finish(), b.finish());
    }
}
