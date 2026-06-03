use crate::ai::tick_ai;
use crate::bubble::BubbleKind;
use crate::collision::{
    detect_cover_deaths, detect_eats, spawn_death_drops, spawn_eat_remains, DeathEvent, EatEvent,
};
use crate::config::{EAT_ABSORB_FRACTION, EAT_REMAIN_FRACTION, WIN_DIAMETER_RATIO};
use crate::config::UNIT_ENERGY_GAIN;
use crate::expel::{apply_expel_to_bubble, clear_expelling, tick_expel};
use crate::physics::apply_player_movement;
use crate::render::Renderer;
use crate::spawn::{fission_oversized_ai, tick_ai_spawning, tick_unit_spawning};
use crate::world::World;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use wasm_bindgen::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameStatus {
    Playing = 0,
    Won = 1,
    Lost = 2,
}

pub struct InputState {
    pub dx: f64,
    pub dy: f64,
    pub expel: bool,
    pub active: bool,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            dx: 0.0,
            dy: 0.0,
            expel: false,
            active: false,
        }
    }
}

pub struct Game {
    world: World,
    renderer: Renderer,
    rng: SmallRng,
    viewport_w: f64,
    viewport_h: f64,
    input: InputState,
    status: GameStatus,
}

impl Game {
    pub fn new(canvas_id: &str, width: f64, height: f64) -> Result<Self, JsValue> {
        let mut rng = SmallRng::from_entropy();
        let renderer = Renderer::new(canvas_id, width, height)?;
        let world = World::new(width, height, &mut rng);
        Ok(Self {
            world,
            renderer,
            rng,
            viewport_w: width,
            viewport_h: height,
            input: InputState::default(),
            status: GameStatus::Playing,
        })
    }

    pub fn resize(&mut self, width: f64, height: f64) {
        self.viewport_w = width;
        self.viewport_h = height;
        self.renderer.resize(width, height);
        let short = width.min(height);
        let map_w = short * crate::config::MAP_SCALE;
        let map_h = short * crate::config::MAP_SCALE;
        self.world.map_width = map_w;
        self.world.map_height = map_h;
        self.world.viewport_w = width;
        self.world.viewport_h = height;
    }

    pub fn set_input(&mut self, dx: f32, dy: f32, expel: bool, active: bool) {
        self.input.dx = dx as f64;
        self.input.dy = dy as f64;
        self.input.expel = expel;
        self.input.active = active;
    }

    pub fn status(&self) -> GameStatus {
        self.status
    }

    pub fn restart(&mut self) {
        self.world.reset(self.viewport_w, self.viewport_h, &mut self.rng);
        self.status = GameStatus::Playing;
        self.input = InputState::default();
    }

    pub fn update(&mut self, dt_ms: f32) {
        let dt = (dt_ms as f64 / 1000.0).min(0.05);

        if self.status != GameStatus::Playing {
            self.render();
            return;
        }

        self.tick_player(dt);
        tick_ai(&mut self.world, dt, &mut self.rng);
        tick_ai_spawning(&mut self.world, dt, &mut self.rng);
        tick_unit_spawning(&mut self.world, dt, &mut self.rng);

        self.resolve_collisions();
        fission_oversized_ai(&mut self.world, &mut self.rng);

        if let Some(player) = self.world.player() {
            let target = self.viewport_w.min(self.viewport_h) * WIN_DIAMETER_RATIO;
            if player.diameter() >= target {
                self.status = GameStatus::Won;
            }
        } else {
            self.status = GameStatus::Lost;
        }

        self.render();
    }

    fn tick_player(&mut self, dt: f64) {
        let player_id = match self.world.player() {
            Some(p) => p.id,
            None => return,
        };

        let (dx, dy, expel, active) = (
            self.input.dx,
            self.input.dy,
            self.input.expel,
            self.input.active,
        );

        let map_w = self.world.map_width;
        let map_h = self.world.map_height;
        let can_expel = self
            .world
            .player()
            .map(crate::physics::can_use_expel)
            .unwrap_or(false);
        let expelling = expel && active && can_expel;

        if active {
            if let Some(p) = self.world.player_mut() {
                apply_player_movement(p, dx, dy, dt, expelling, map_w, map_h);
            }
        }

        if expelling {
            if let Some(imp) = tick_expel(
                &mut self.world,
                player_id,
                dx,
                dy,
                dt,
                &mut self.rng,
            ) {
                if let Some(p) = self.world.player_mut() {
                    apply_expel_to_bubble(p, imp);
                    crate::physics::clamp_bubble_speed(p, true);
                }
            }
        } else {
            clear_expelling(&mut self.world, player_id);
        }
    }

    fn resolve_collisions(&mut self) {
        let eats = detect_eats(&self.world.bubbles);
        for ev in eats {
            self.apply_eat(ev);
        }

        let deaths = detect_cover_deaths(&self.world.bubbles);
        for ev in deaths {
            self.apply_death(ev);
        }
    }

    fn apply_eat(&mut self, ev: EatEvent) {
        if !self.world.bubbles.iter().any(|b| b.id == ev.eaten_id) {
            return;
        }
        if !self.world.bubbles.iter().any(|b| b.id == ev.eater_id) {
            return;
        }
        self.world.remove_by_id(ev.eaten_id);

        if let Some(eater) = self.world.bubbles.iter_mut().find(|b| b.id == ev.eater_id) {
            let gained = match ev.eaten_kind {
                BubbleKind::Unit => ev.energy * UNIT_ENERGY_GAIN,
                BubbleKind::Remains => ev.energy,
                BubbleKind::Player | BubbleKind::Ai => ev.energy * EAT_ABSORB_FRACTION,
            };
            eater.add_energy(gained);
            if matches!(ev.eaten_kind, BubbleKind::Player | BubbleKind::Ai) {
                eater.eaten_non_unit = true;
            }
        }

        if matches!(ev.eaten_kind, BubbleKind::Player | BubbleKind::Ai) {
            spawn_eat_remains(
                &mut self.world,
                ev.eater_x,
                ev.eater_y,
                ev.eater_radius,
                ev.eaten_x,
                ev.eaten_y,
                ev.energy * EAT_REMAIN_FRACTION,
                ev.eaten_color,
                &mut self.rng,
            );
        }

        if ev.eaten_kind == BubbleKind::Player {
            self.status = GameStatus::Lost;
        }
    }

    fn apply_death(&mut self, ev: DeathEvent) {
        if !self.world.bubbles.iter().any(|b| b.id == ev.bubble_id) {
            return;
        }
        self.world.remove_by_id(ev.bubble_id);
        spawn_death_drops(
            &mut self.world,
            ev.x,
            ev.y,
            ev.energy,
            ev.radius,
            ev.color,
            ev.vx,
            ev.vy,
            ev.steer_x,
            ev.steer_y,
            &mut self.rng,
        );
        self.world.add_cluster_at(ev.x, ev.y);
        if ev.kind == BubbleKind::Player {
            self.status = GameStatus::Lost;
        }
    }

    fn render(&self) {
        let progress = if let Some(p) = self.world.player() {
            let target = self.viewport_w.min(self.viewport_h) * WIN_DIAMETER_RATIO;
            (p.diameter() / target).min(1.0)
        } else {
            0.0
        };
        self.renderer.draw(
            &self.world,
            self.viewport_w,
            self.viewport_h,
            self.status,
            progress,
        );
    }
}
