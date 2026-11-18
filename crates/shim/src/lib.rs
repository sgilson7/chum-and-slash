//! tool_shim: moves values between the page and `tool_core::game`.
//!
//! Builder requirement: the shim decides nothing. Each function is one call
//! into the core, with its arguments and result converted (milliseconds to
//! microseconds, floats to Q16 and back). `crates/core/tests/boundary.rs`
//! fails if a branch, a loop or the `?` operator appears here. (After
//! Protection of the Spheres' shim.)

use tool_core::fx::ONE;
use tool_core::game;
use wasm_bindgen::prelude::*;

fn floats(v: Vec<i32>) -> Vec<f32> {
    v.into_iter().map(|x| x as f32 / ONE as f32).collect()
}

fn q16(v: Vec<f32>) -> Vec<i32> {
    v.into_iter().map(|x| (x * ONE as f32) as i32).collect()
}

#[wasm_bindgen]
pub struct Game {
    inner: game::Game,
}

#[wasm_bindgen]
impl Game {
    #[allow(clippy::new_without_default)]
    #[wasm_bindgen(constructor)]
    pub fn new() -> Game {
        Game {
            inner: game::Game::new(),
        }
    }

    pub fn frame(&mut self, now_ms: f64) {
        self.inner.frame((now_ms * 1000.0) as i64)
    }

    pub fn key(&mut self, code: &str, down: bool) {
        self.inner.key(code, down)
    }

    pub fn mouse(&mut self, dx: f64, dy: f64) {
        self.inner.mouse(dx as i32, dy as i32)
    }

    pub fn pad(&mut self, buttons: Vec<u8>, axes: Vec<f32>, id: &str) {
        self.inner
            .pad(buttons.into_iter().map(|b| b != 0).collect(), q16(axes), id)
    }

    pub fn act(&mut self, name: &str, value: &str) -> String {
        game::answer(self.inner.act(name, value))
    }

    pub fn ui(&self) -> String {
        self.inner.ui().to_string()
    }

    pub fn probe(&self) -> String {
        self.inner.probe().to_string()
    }

    pub fn events(&mut self) -> String {
        game::events_json(&self.inner.take_events())
    }

    pub fn render(&mut self) {
        self.inner.render()
    }

    pub fn scene(&self) -> Vec<i32> {
        self.inner.dynamic.0.clone()
    }

    pub fn scenery(&self) -> Vec<i32> {
        self.inner.scenery.0.clone()
    }

    pub fn scenery_version(&self) -> u32 {
        self.inner.scenery_version
    }

    pub fn draws(&self) -> Vec<u32> {
        self.inner.draws()
    }

    pub fn mats(&self) -> Vec<f32> {
        floats(self.inner.mats())
    }

    pub fn uniforms(&self) -> Vec<f32> {
        floats(self.inner.uniforms())
    }

    pub fn clear_color(&self) -> Vec<f32> {
        floats(self.inner.clear_color())
    }
}

#[wasm_bindgen]
pub fn numbers() -> String {
    game::numbers()
}

#[wasm_bindgen]
pub fn mesh(id: u32) -> Vec<i32> {
    game::mesh(id)
}

#[wasm_bindgen]
pub fn noise(n: u32) -> Vec<f32> {
    floats(game::noise(n as usize))
}

/// One Q16 step as a float, so the page can scale the core's integers
/// without a number of its own.
#[wasm_bindgen]
pub fn unit() -> f32 {
    1.0 / ONE as f32
}
