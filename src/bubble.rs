use crate::config::{INITIAL_RADIUS, PLAYER_COLOR, UNIT_ENERGY, AI_INTEREST_MAX, AI_INTEREST_MIN};
use rand::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BubbleKind {
    Player,
    Ai,
    Unit,
    /// 死亡或吞噬后残留的静止能量块
    Remains,
}

#[derive(Clone, Debug)]
pub struct Bubble {
    pub id: u32,
    pub x: f64,
    pub y: f64,
    pub energy: f64,
    pub kind: BubbleKind,
    pub color: (u8, u8, u8),
    pub vx: f64,
    pub vy: f64,
    pub eaten_non_unit: bool,
    pub expelling: bool,
    pub ai_cooldown: f64,
    /// AI 平滑转向用的当前方向
    pub steer_x: f64,
    pub steer_y: f64,
    /// 自杀式 AI：比玩家小，持续冲向玩家
    pub suicide: bool,
    /// AI 长期移动意图（方向 + 剩余持续时间）
    pub ai_intent_x: f64,
    pub ai_intent_y: f64,
    pub ai_intent_timer: f64,
    /// > 0 时在原地缓慢打转
    pub ai_spin_timer: f64,
    /// 更擅长躲避与发育的 AI
    pub ai_smart: bool,
    /// 比玩家大的围剿型 AI，主动追猎玩家
    pub ai_hunter: bool,
    /// Hunter 本轮已追击玩家的时间
    pub ai_hunter_chase_elapsed: f64,
    /// Hunter 本轮追击玩家的时间上限
    pub ai_hunter_chase_limit: f64,
    /// > 0 时表示对玩家失去兴趣，改追其它目标
    pub ai_hunter_player_cooldown: f64,
    /// 当前追击目标已持续的时间
    pub ai_interest_elapsed: f64,
    /// 本轮追击时间上限
    pub ai_interest_limit: f64,
    /// > 0 时放弃追击，专注吃单位/尸体
    pub ai_forage_timer: f64,
    /// 沿地图边界滑行的累计时间（秒）
    pub ai_wall_hug_timer: f64,
}

impl Bubble {
    pub fn radius(&self) -> f64 {
        (self.energy / std::f64::consts::PI).sqrt()
    }

    pub fn diameter(&self) -> f64 {
        self.radius() * 2.0
    }

    pub fn new_player(id: u32, x: f64, y: f64) -> Self {
        let energy = std::f64::consts::PI * INITIAL_RADIUS * INITIAL_RADIUS;
        Self {
            id,
            x,
            y,
            energy,
            kind: BubbleKind::Player,
            color: PLAYER_COLOR,
            vx: 0.0,
            vy: 0.0,
            eaten_non_unit: false,
            expelling: false,
            ai_cooldown: 0.0,
            steer_x: 0.0,
            steer_y: 0.0,
            suicide: false,
            ai_intent_x: 1.0,
            ai_intent_y: 0.0,
            ai_intent_timer: 0.0,
            ai_spin_timer: 0.0,
            ai_smart: false,
            ai_hunter: false,
            ai_hunter_chase_elapsed: 0.0,
            ai_hunter_chase_limit: 0.0,
            ai_hunter_player_cooldown: 0.0,
            ai_interest_elapsed: 0.0,
            ai_interest_limit: 0.0,
            ai_forage_timer: 0.0,
            ai_wall_hug_timer: 0.0,
        }
    }

    pub fn clamp_to_max_radius(&mut self, max_radius: f64) {
        let max_energy = std::f64::consts::PI * max_radius * max_radius;
        if self.energy > max_energy {
            self.energy = max_energy;
        }
    }

    pub fn roll_ai_radius(player_radius: f64, max_radius: f64, rng: &mut impl Rng) -> f64 {
        let roll = rng.gen::<f64>();
        let r = if roll < crate::config::AI_SPAWN_LARGE_CHANCE {
            player_radius
                * rng.gen_range(crate::config::AI_SPAWN_LARGE_MIN..crate::config::AI_SPAWN_LARGE_MAX)
        } else if roll < 0.38 {
            INITIAL_RADIUS * rng.gen_range(0.55..0.92)
        } else {
            INITIAL_RADIUS * rng.gen_range(0.72..1.32)
        };
        r.min(max_radius)
    }

    pub fn new_hunter_ai(
        id: u32,
        x: f64,
        y: f64,
        player_radius: f64,
        max_radius: f64,
        rng: &mut impl Rng,
    ) -> Self {
        let r = (player_radius
            * rng.gen_range(crate::config::AI_HUNTER_MIN..crate::config::AI_HUNTER_MAX))
        .min(max_radius);
        let energy = std::f64::consts::PI * r * r;
        let angle = rng.gen_range(0.0..std::f64::consts::TAU);
        Self {
            id,
            x,
            y,
            energy,
            kind: BubbleKind::Ai,
            color: (220, 90, 70),
            vx: 0.0,
            vy: 0.0,
            eaten_non_unit: true,
            expelling: false,
            ai_cooldown: 0.0,
            steer_x: angle.cos(),
            steer_y: angle.sin(),
            suicide: false,
            ai_intent_x: angle.cos(),
            ai_intent_y: angle.sin(),
            ai_intent_timer: 0.0,
            ai_spin_timer: 0.0,
            ai_smart: true,
            ai_hunter: true,
            ai_hunter_chase_elapsed: 0.0,
            ai_hunter_chase_limit: rng.gen_range(
                crate::config::AI_HUNTER_CHASE_MIN..crate::config::AI_HUNTER_CHASE_MAX,
            ),
            ai_hunter_player_cooldown: 0.0,
            ai_interest_elapsed: 0.0,
            ai_interest_limit: Self::roll_interest_limit(rng),
            ai_forage_timer: 0.0,
            ai_wall_hug_timer: 0.0,
        }
    }

    pub fn new_ai(
        id: u32,
        x: f64,
        y: f64,
        player_radius: f64,
        max_radius: f64,
        rng: &mut impl Rng,
    ) -> Self {
        let r = Self::roll_ai_radius(player_radius, max_radius, rng);
        let energy = std::f64::consts::PI * r * r;
        let angle = rng.gen_range(0.0..std::f64::consts::TAU);
        let smart = rng.gen_bool(crate::config::AI_SMART_CHANCE);
        Self {
            id,
            x,
            y,
            energy,
            kind: BubbleKind::Ai,
            color: random_ai_color(rng),
            vx: 0.0,
            vy: 0.0,
            eaten_non_unit: r > player_radius * crate::config::EAT_RATIO,
            expelling: false,
            ai_cooldown: rng.gen_range(0.0..0.5),
            steer_x: angle.cos(),
            steer_y: angle.sin(),
            suicide: false,
            ai_intent_x: angle.cos(),
            ai_intent_y: angle.sin(),
            ai_intent_timer: rng.gen_range(0.0..0.5),
            ai_spin_timer: 0.0,
            ai_smart: smart,
            ai_hunter: false,
            ai_hunter_chase_elapsed: 0.0,
            ai_hunter_chase_limit: 0.0,
            ai_hunter_player_cooldown: 0.0,
            ai_interest_elapsed: 0.0,
            ai_interest_limit: Self::roll_interest_limit(rng),
            ai_forage_timer: 0.0,
            ai_wall_hug_timer: 0.0,
        }
    }

    pub fn new_ai_from_fission(
        id: u32,
        x: f64,
        y: f64,
        energy: f64,
        max_radius: f64,
        parent: &Bubble,
        rng: &mut impl Rng,
    ) -> Self {
        let capped = energy.min(std::f64::consts::PI * max_radius * max_radius * 0.94);
        let angle = rng.gen_range(0.0..std::f64::consts::TAU);
        Self {
            id,
            x,
            y,
            energy: capped,
            kind: BubbleKind::Ai,
            color: parent.color,
            vx: 0.0,
            vy: 0.0,
            eaten_non_unit: parent.eaten_non_unit,
            expelling: false,
            ai_cooldown: rng.gen_range(0.0..0.35),
            steer_x: angle.cos(),
            steer_y: angle.sin(),
            suicide: false,
            ai_intent_x: angle.cos(),
            ai_intent_y: angle.sin(),
            ai_intent_timer: rng.gen_range(0.0..0.4),
            ai_spin_timer: 0.0,
            ai_smart: parent.ai_smart,
            ai_hunter: false,
            ai_hunter_chase_elapsed: 0.0,
            ai_hunter_chase_limit: 0.0,
            ai_hunter_player_cooldown: 0.0,
            ai_interest_elapsed: 0.0,
            ai_interest_limit: Self::roll_interest_limit(rng),
            ai_forage_timer: rng.gen_range(0.0..1.2),
            ai_wall_hug_timer: 0.0,
        }
    }

    pub fn new_suicide_ai(id: u32, x: f64, y: f64, player_radius: f64, rng: &mut impl Rng) -> Self {
        let scale = rng.gen_range(
            crate::config::SUICIDE_RADIUS_MIN..crate::config::SUICIDE_RADIUS_MAX,
        );
        let r = player_radius * scale;
        let energy = std::f64::consts::PI * r * r;
        let angle = rng.gen_range(0.0..std::f64::consts::TAU);
        Self {
            id,
            x,
            y,
            energy,
            kind: BubbleKind::Ai,
            color: (255, 80, 80),
            vx: 0.0,
            vy: 0.0,
            eaten_non_unit: true,
            expelling: false,
            ai_cooldown: 0.0,
            steer_x: angle.cos(),
            steer_y: angle.sin(),
            suicide: true,
            ai_intent_x: angle.cos(),
            ai_intent_y: angle.sin(),
            ai_intent_timer: 0.0,
            ai_spin_timer: 0.0,
            ai_smart: false,
            ai_hunter: false,
            ai_hunter_chase_elapsed: 0.0,
            ai_hunter_chase_limit: 0.0,
            ai_hunter_player_cooldown: 0.0,
            ai_interest_elapsed: 0.0,
            ai_interest_limit: 0.0,
            ai_forage_timer: 0.0,
            ai_wall_hug_timer: 0.0,
        }
    }

    pub fn new_unit(id: u32, x: f64, y: f64, rng: &mut impl Rng) -> Self {
        Self {
            id,
            x,
            y,
            energy: UNIT_ENERGY,
            kind: BubbleKind::Unit,
            color: random_unit_color(rng),
            vx: 0.0,
            vy: 0.0,
            eaten_non_unit: false,
            expelling: false,
            ai_cooldown: 0.0,
            steer_x: 0.0,
            steer_y: 0.0,
            suicide: false,
            ai_intent_x: 0.0,
            ai_intent_y: 0.0,
            ai_intent_timer: 0.0,
            ai_spin_timer: 0.0,
            ai_smart: false,
            ai_hunter: false,
            ai_hunter_chase_elapsed: 0.0,
            ai_hunter_chase_limit: 0.0,
            ai_hunter_player_cooldown: 0.0,
            ai_interest_elapsed: 0.0,
            ai_interest_limit: 0.0,
            ai_forage_timer: 0.0,
            ai_wall_hug_timer: 0.0,
        }
    }

    pub fn new_remains(id: u32, x: f64, y: f64, energy: f64, source_color: (u8, u8, u8)) -> Self {
        Self {
            id,
            x,
            y,
            energy,
            kind: BubbleKind::Remains,
            color: remains_color_from(source_color),
            vx: 0.0,
            vy: 0.0,
            eaten_non_unit: false,
            expelling: false,
            ai_cooldown: 0.0,
            steer_x: 0.0,
            steer_y: 0.0,
            suicide: false,
            ai_intent_x: 0.0,
            ai_intent_y: 0.0,
            ai_intent_timer: 0.0,
            ai_spin_timer: 0.0,
            ai_smart: false,
            ai_hunter: false,
            ai_hunter_chase_elapsed: 0.0,
            ai_hunter_chase_limit: 0.0,
            ai_hunter_player_cooldown: 0.0,
            ai_interest_elapsed: 0.0,
            ai_interest_limit: 0.0,
            ai_forage_timer: 0.0,
            ai_wall_hug_timer: 0.0,
        }
    }

    pub fn roll_interest_limit(rng: &mut impl Rng) -> f64 {
        rng.gen_range(AI_INTEREST_MIN..AI_INTEREST_MAX)
    }

    pub fn add_energy(&mut self, amount: f64) {
        self.energy += amount;
    }

    pub fn can_eat(&self, other: &Bubble) -> bool {
        if self.id == other.id {
            return false;
        }
        if other.kind == BubbleKind::Unit {
            return true;
        }
        if other.kind == BubbleKind::Remains {
            return self.radius() > other.radius() * crate::config::EAT_RATIO;
        }
        if self.kind == BubbleKind::Player && !self.eaten_non_unit {
            return false;
        }
        if self.kind == BubbleKind::Player && other.kind == BubbleKind::Ai {
            return self.radius() > other.radius();
        }
        if self.kind == BubbleKind::Ai && other.kind == BubbleKind::Player {
            return self.radius() > other.radius() * crate::config::AI_EAT_PLAYER_RATIO;
        }
        self.radius() > other.radius() * crate::config::EAT_RATIO
    }
}

pub fn remains_color_from(source: (u8, u8, u8)) -> (u8, u8, u8) {
    let (r, g, b) = source;
    let lum = (r as f64 * 0.299 + g as f64 * 0.587 + b as f64 * 0.114) as u8;
    (
        ((r as u16 + lum as u16 + 40) / 2).min(255) as u8,
        ((g as u16 + lum as u16 + 40) / 2).min(255) as u8,
        ((b as u16 + lum as u16 + 55) / 2).min(255) as u8,
    )
}

pub fn random_unit_color(rng: &mut impl Rng) -> (u8, u8, u8) {
    let h = rng.gen_range(0.0..360.0);
    hsl_to_rgb(h, 0.7, 0.55)
}

pub fn random_ai_color(rng: &mut impl Rng) -> (u8, u8, u8) {
    loop {
        let h = rng.gen_range(0.0..360.0);
        let (r, g, b) = hsl_to_rgb(h, 0.65, 0.5);
        let dist = color_distance((r, g, b), PLAYER_COLOR);
        let bg_dist = color_distance((r, g, b), crate::config::BG_COLOR);
        if dist > 80.0 && bg_dist > 60.0 {
            return (r, g, b);
        }
    }
}

fn color_distance(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    let dr = a.0 as f64 - b.0 as f64;
    let dg = a.1 as f64 - b.1 as f64;
    let db = a.2 as f64 - b.2 as f64;
    (dr * dr + dg * dg + db * db).sqrt()
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
    let (r1, g1, b1) = if h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = l - c / 2.0;
    (
        ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}
