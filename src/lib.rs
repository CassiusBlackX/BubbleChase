pub mod ai;
pub mod bubble;
pub mod collision;
pub mod config;
pub mod expel;
pub mod game;
pub mod physics;
pub mod render;
pub mod spawn;
pub mod world;

use wasm_bindgen::prelude::*;

static mut GAME: Option<game::Game> = None;

#[wasm_bindgen]
pub fn init(canvas_id: &str, width: f64, height: f64) -> Result<(), JsValue> {
    let game = game::Game::new(canvas_id, width, height)?;
    unsafe {
        GAME = Some(game);
    }
    Ok(())
}

#[wasm_bindgen]
pub fn resize(width: f64, height: f64) {
    unsafe {
        if let Some(game) = GAME.as_mut() {
            game.resize(width, height);
        }
    }
}

#[wasm_bindgen]
pub fn update(dt_ms: f32) {
    unsafe {
        if let Some(game) = GAME.as_mut() {
            game.update(dt_ms);
        }
    }
}

#[wasm_bindgen]
pub fn set_input(dx: f32, dy: f32, expel: bool, active: bool) {
    unsafe {
        if let Some(game) = GAME.as_mut() {
            game.set_input(dx, dy, expel, active);
        }
    }
}

#[wasm_bindgen]
pub fn restart() {
    unsafe {
        if let Some(game) = GAME.as_mut() {
            game.restart();
        }
    }
}

#[wasm_bindgen]
pub fn get_game_status() -> u8 {
    unsafe {
        GAME.as_ref()
            .map(|g| g.status() as u8)
            .unwrap_or(0)
    }
}
