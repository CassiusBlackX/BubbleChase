use crate::bubble::{Bubble, BubbleKind};
use crate::config::{
    AI_DENSITY, AI_SPAWN_IN_VIEW_CHANCE, INITIAL_RADIUS, MAP_SCALE, MAX_AI, MIN_AI,
    SUICIDE_SPAWN_CHANCE, UNIT_CLUSTER_COUNT, UNIT_CLUSTER_INITIAL, UNIT_CLUSTER_TIGHT_SPREAD,
    UNIT_SCATTER_INITIAL, WIN_DIAMETER_RATIO,
};
use crate::physics::camera_offset;
use crate::spawn::roll_ai_spawn_delay;
use rand::Rng;

pub struct World {
    pub map_width: f64,
    pub map_height: f64,
    pub viewport_w: f64,
    pub viewport_h: f64,
    pub bubbles: Vec<Bubble>,
    pub next_id: u32,
    pub unit_spawn_timer: f64,
    /// 距离下次尝试刷新 AI 的倒计时
    pub ai_spawn_timer: f64,
    /// 单位泡泡富集区域中心（类似尸体掉落物聚集点）
    pub unit_clusters: Vec<(f64, f64)>,
}

impl World {
    pub fn new(viewport_w: f64, viewport_h: f64, rng: &mut impl Rng) -> Self {
        let short = viewport_w.min(viewport_h);
        let map_width = short * MAP_SCALE;
        let map_height = short * MAP_SCALE;
        let mut world = Self {
            map_width,
            map_height,
            viewport_w: viewport_w,
            viewport_h: viewport_h,
            bubbles: Vec::new(),
            next_id: 1,
            unit_spawn_timer: 0.0,
            ai_spawn_timer: roll_ai_spawn_delay(rng),
            unit_clusters: Vec::new(),
        };

        world.init_clusters(rng);

        let px = map_width / 2.0;
        let py = map_height / 2.0;
        let player_id = world.alloc_id();
        world.bubbles.push(Bubble::new_player(player_id, px, py));
        let player_radius = world.player().map(|p| p.radius()).unwrap_or(INITIAL_RADIUS);
        let max_radius = world.player_win_radius();

        let ai_count = world.compute_ai_count();
        for _ in 0..ai_count {
            let (x, y) = world.random_point(rng, 80.0);
            let id = world.alloc_id();
            world
                .bubbles
                .push(Bubble::new_ai(id, x, y, player_radius, max_radius, rng));
        }

        if rng.gen_bool(SUICIDE_SPAWN_CHANCE) {
            world.spawn_suicide_ai(px, py, player_radius, rng);
        }

        for _ in 0..UNIT_SCATTER_INITIAL {
            let (x, y) = world.random_point(rng, 20.0);
            let id = world.alloc_id();
            world.bubbles.push(Bubble::new_unit(id, x, y, rng));
        }

        for &(cx, cy) in &world.unit_clusters.clone() {
            for _ in 0..UNIT_CLUSTER_INITIAL {
                let (x, y) = world.cluster_offset(cx, cy, UNIT_CLUSTER_TIGHT_SPREAD, rng);
                let id = world.alloc_id();
                world.bubbles.push(Bubble::new_unit(id, x, y, rng));
            }
        }

        world
    }

    fn init_clusters(&mut self, rng: &mut impl Rng) {
        self.unit_clusters.clear();
        for _ in 0..UNIT_CLUSTER_COUNT {
            let (x, y) = self.random_point(rng, 60.0);
            self.unit_clusters.push((x, y));
        }
    }

    pub fn compute_ai_count(&self) -> usize {
        let area = self.map_width * self.map_height;
        let count = (area / AI_DENSITY).round() as usize;
        count.clamp(MIN_AI, MAX_AI)
    }

    pub fn ai_count(&self) -> usize {
        self.bubbles
            .iter()
            .filter(|b| b.kind == BubbleKind::Ai)
            .count()
    }

    pub fn player_win_radius(&self) -> f64 {
        self.viewport_w.min(self.viewport_h) * WIN_DIAMETER_RATIO / 2.0
    }

    pub fn spawn_ai_bubble(&mut self, rng: &mut impl Rng) {
        let player_radius = self.player().map(|p| p.radius()).unwrap_or(INITIAL_RADIUS);
        let max_radius = self.player_win_radius();
        let (x, y) = self.random_ai_spawn_point(rng);
        let id = self.alloc_id();
        if rng.gen_bool(crate::config::AI_HUNTER_SPAWN_CHANCE) {
            self.bubbles
                .push(Bubble::new_hunter_ai(id, x, y, player_radius, max_radius, rng));
        } else {
            self.bubbles
                .push(Bubble::new_ai(id, x, y, player_radius, max_radius, rng));
        }
    }

    pub fn spawn_hunter_ai(&mut self, rng: &mut impl Rng) {
        let player_radius = self.player().map(|p| p.radius()).unwrap_or(INITIAL_RADIUS);
        let max_radius = self.player_win_radius();
        let (x, y) = self.random_ai_spawn_point(rng);
        let id = self.alloc_id();
        self.bubbles
            .push(Bubble::new_hunter_ai(id, x, y, player_radius, max_radius, rng));
    }

    fn visible_bounds(&self) -> (f64, f64, f64, f64) {
        let (px, py) = self
            .player()
            .map(|p| (p.x, p.y))
            .unwrap_or((self.map_width / 2.0, self.map_height / 2.0));
        let (cam_x, cam_y) = camera_offset(
            px,
            py,
            self.viewport_w,
            self.viewport_h,
            self.map_width,
            self.map_height,
        );
        (
            cam_x,
            cam_y,
            cam_x + self.viewport_w,
            cam_y + self.viewport_h,
        )
    }

    fn random_ai_spawn_point(&self, rng: &mut impl Rng) -> (f64, f64) {
        if rng.gen_bool(AI_SPAWN_IN_VIEW_CHANCE) {
            self.random_point_in_view(rng, 40.0)
        } else {
            self.random_point_outside_view(rng, 40.0)
        }
    }

    fn random_point_in_view(&self, rng: &mut impl Rng, margin: f64) -> (f64, f64) {
        let (vx0, vy0, vx1, vy1) = self.visible_bounds();
        let min_x = (vx0 + margin).max(margin);
        let max_x = (vx1 - margin).max(min_x + 1.0).min(self.map_width - margin);
        let min_y = (vy0 + margin).max(margin);
        let max_y = (vy1 - margin).max(min_y + 1.0).min(self.map_height - margin);
        let x = rng.gen_range(min_x..max_x);
        let y = rng.gen_range(min_y..max_y);
        (x, y)
    }

    fn random_point_outside_view(&self, rng: &mut impl Rng, margin: f64) -> (f64, f64) {
        let (vx0, vy0, vx1, vy1) = self.visible_bounds();
        let pad = 20.0;
        for _ in 0..24 {
            let (x, y) = self.random_point(rng, margin);
            if x < vx0 - pad || x > vx1 + pad || y < vy0 - pad || y > vy1 + pad {
                return (x, y);
            }
        }
        let away = self.viewport_w.max(self.viewport_h) * 0.55;
        self.random_point_away_from_player(rng, away.max(120.0))
    }

    fn random_point_away_from_player(
        &self,
        rng: &mut impl Rng,
        min_dist: f64,
    ) -> (f64, f64) {
        let (px, py) = self
            .player()
            .map(|p| (p.x, p.y))
            .unwrap_or((self.map_width / 2.0, self.map_height / 2.0));

        for _ in 0..12 {
            let (x, y) = self.random_point(rng, 50.0);
            let dx = x - px;
            let dy = y - py;
            if dx * dx + dy * dy >= min_dist * min_dist {
                return (x, y);
            }
        }
        self.random_point(rng, 50.0)
    }

    pub fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn random_point(&self, rng: &mut impl Rng, margin: f64) -> (f64, f64) {
        let x = rng.gen_range(margin..(self.map_width - margin));
        let y = rng.gen_range(margin..(self.map_height - margin));
        (x, y)
    }

    pub fn spawn_scattered_unit(&mut self, rng: &mut impl Rng) {
        let (x, y) = self.random_point(rng, 15.0);
        let id = self.alloc_id();
        self.bubbles.push(Bubble::new_unit(id, x, y, rng));
    }

    pub fn pick_cluster(&self, rng: &mut impl Rng) -> (f64, f64) {
        if self.unit_clusters.is_empty() {
            return self.random_point(rng, 20.0);
        }
        let idx = rng.gen_range(0..self.unit_clusters.len());
        self.unit_clusters[idx]
    }

    pub fn cluster_offset(
        &self,
        cx: f64,
        cy: f64,
        spread: f64,
        rng: &mut impl Rng,
    ) -> (f64, f64) {
        let angle = rng.gen_range(0.0..std::f64::consts::TAU);
        let dist = rng.gen::<f64>().powf(0.5) * spread;
        let x = (cx + angle.cos() * dist).clamp(10.0, self.map_width - 10.0);
        let y = (cy + angle.sin() * dist).clamp(10.0, self.map_height - 10.0);
        (x, y)
    }

    pub fn add_cluster_at(&mut self, x: f64, y: f64) {
        self.unit_clusters.push((x, y));
        if self.unit_clusters.len() > UNIT_CLUSTER_COUNT * 3 {
            self.unit_clusters.remove(0);
        }
    }

    fn spawn_suicide_ai(&mut self, px: f64, py: f64, player_radius: f64, rng: &mut impl Rng) {
        let angle = rng.gen_range(0.0..std::f64::consts::TAU);
        let dist = (self.map_width.min(self.map_height) * 0.35).max(120.0);
        let x = (px + angle.cos() * dist).clamp(40.0, self.map_width - 40.0);
        let y = (py + angle.sin() * dist).clamp(40.0, self.map_height - 40.0);
        let id = self.alloc_id();
        self.bubbles
            .push(Bubble::new_suicide_ai(id, x, y, player_radius, rng));
    }

    pub fn player_index(&self) -> Option<usize> {
        self.bubbles
            .iter()
            .position(|b| b.kind == BubbleKind::Player)
    }

    pub fn player(&self) -> Option<&Bubble> {
        self.bubbles.iter().find(|b| b.kind == BubbleKind::Player)
    }

    pub fn player_mut(&mut self) -> Option<&mut Bubble> {
        self.bubbles.iter_mut().find(|b| b.kind == BubbleKind::Player)
    }

    pub fn clamp_position(bubble: &mut Bubble, map_w: f64, map_h: f64) {
        let r = bubble.radius();
        bubble.x = bubble.x.clamp(r, map_w - r);
        bubble.y = bubble.y.clamp(r, map_h - r);
    }

    pub fn remove_by_id(&mut self, id: u32) {
        self.bubbles.retain(|b| b.id != id);
    }

    pub fn reset(&mut self, viewport_w: f64, viewport_h: f64, rng: &mut impl Rng) {
        *self = World::new(viewport_w, viewport_h, rng);
    }
}
