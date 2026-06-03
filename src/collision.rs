use crate::bubble::{Bubble, BubbleKind};
use crate::config::{
    COVER_DEATH_RATIO, DEATH_DROP_FRACTION, DEATH_DROP_MAX, DEATH_DROP_MIN,
    DEATH_SPRAY_BASE, DEATH_SPRAY_LATERAL_BASE, DEATH_SPRAY_LATERAL_RADIUS_SCALE,
    DEATH_SPRAY_RADIUS_SCALE, DROP_ENERGY_WEIGHT_MAX, DROP_ENERGY_WEIGHT_MIN, EAT_RATIO,
    EAT_REMAIN_FORWARD_SPREAD, EAT_REMAIN_LATERAL_SPREAD, EAT_REMAIN_MIN_GAP, MIN_DROP_ENERGY,
    UNIT_ENERGY,
};
use rand::Rng;

pub fn circle_intersection_area(r1: f64, r2: f64, d: f64) -> f64 {
    if d >= r1 + r2 {
        return 0.0;
    }
    if d <= (r1 - r2).abs() {
        return std::f64::consts::PI * r2.min(r1).powi(2);
    }
    let r1_sq = r1 * r1;
    let r2_sq = r2 * r2;
    let alpha = ((d * d + r1_sq - r2_sq) / (2.0 * d * r1)).clamp(-1.0, 1.0).acos();
    let beta = ((d * d + r2_sq - r1_sq) / (2.0 * d * r2)).clamp(-1.0, 1.0).acos();
    0.5 * r1_sq * (2.0 * alpha - (2.0 * alpha).sin())
        + 0.5 * r2_sq * (2.0 * beta - (2.0 * beta).sin())
}

pub fn is_covered_death(big: &Bubble, small: &Bubble) -> bool {
    if big.radius() <= small.radius() {
        return false;
    }
    let dx = big.x - small.x;
    let dy = big.y - small.y;
    let d = (dx * dx + dy * dy).sqrt();
    let small_area = std::f64::consts::PI * small.radius().powi(2);
    if small_area <= 0.0 {
        return false;
    }
    let inter = circle_intersection_area(big.radius(), small.radius(), d);
    inter / small_area >= COVER_DEATH_RATIO
}

pub struct EatEvent {
    pub eater_id: u32,
    pub eaten_id: u32,
    pub energy: f64,
    pub eaten_kind: BubbleKind,
    pub eaten_x: f64,
    pub eaten_y: f64,
    pub eaten_color: (u8, u8, u8),
    pub eater_x: f64,
    pub eater_y: f64,
    pub eater_radius: f64,
}

pub struct DeathEvent {
    pub bubble_id: u32,
    pub x: f64,
    pub y: f64,
    pub energy: f64,
    pub radius: f64,
    pub kind: BubbleKind,
    pub color: (u8, u8, u8),
    pub vx: f64,
    pub vy: f64,
    pub steer_x: f64,
    pub steer_y: f64,
}

pub fn detect_eats(bubbles: &[Bubble]) -> Vec<EatEvent> {
    let mut events = Vec::new();
    for i in 0..bubbles.len() {
        for j in 0..bubbles.len() {
            if i == j {
                continue;
            }
            let a = &bubbles[i];
            let b = &bubbles[j];
            if !a.can_eat(b) {
                continue;
            }
            let dx = a.x - b.x;
            let dy = a.y - b.y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < a.radius() - b.radius() * 0.3 {
                events.push(EatEvent {
                    eater_id: a.id,
                    eaten_id: b.id,
                    energy: b.energy,
                    eaten_kind: b.kind,
                    eaten_x: b.x,
                    eaten_y: b.y,
                    eaten_color: b.color,
                    eater_x: a.x,
                    eater_y: a.y,
                    eater_radius: a.radius(),
                });
            }
        }
    }
    events.sort_by_key(|e| e.eaten_id);
    events.dedup_by_key(|e| e.eaten_id);
    events
}

pub fn detect_cover_deaths(bubbles: &[Bubble]) -> Vec<DeathEvent> {
    let mut events = Vec::new();
    for i in 0..bubbles.len() {
        for j in 0..bubbles.len() {
            if i == j {
                continue;
            }
            let big = &bubbles[i];
            let small = &bubbles[j];
            if big.radius() <= small.radius() * EAT_RATIO {
                continue;
            }
            if is_covered_death(big, small) {
                events.push(DeathEvent {
                    bubble_id: small.id,
                    x: small.x,
                    y: small.y,
                    energy: small.energy,
                    radius: small.radius(),
                    kind: small.kind,
                    color: small.color,
                    vx: small.vx,
                    vy: small.vy,
                    steer_x: small.steer_x,
                    steer_y: small.steer_y,
                });
            }
        }
    }
    events.sort_by_key(|e| e.bubble_id);
    events.dedup_by_key(|e| e.bubble_id);
    events
}

fn drop_count_for_energy(total_energy: f64, rng: &mut impl Rng) -> usize {
    if total_energy <= MIN_DROP_ENERGY {
        return 0;
    }
    if total_energy <= UNIT_ENERGY * 2.5 {
        return rng.gen_range(1..=3);
    }
    rng.gen_range(DEATH_DROP_MIN..=DEATH_DROP_MAX)
}

pub fn partition_drop_energy(total_energy: f64, count: usize, rng: &mut impl Rng) -> Vec<f64> {
    if count == 0 {
        return Vec::new();
    }
    let weights: Vec<f64> = (0..count)
        .map(|_| rng.gen_range(DROP_ENERGY_WEIGHT_MIN..DROP_ENERGY_WEIGHT_MAX))
        .collect();
    let sum: f64 = weights.iter().sum();
    let mut portions: Vec<f64> = weights
        .into_iter()
        .map(|w| total_energy * w / sum)
        .filter(|e| *e >= MIN_DROP_ENERGY)
        .collect();
    if portions.is_empty() {
        return vec![total_energy];
    }
    let allocated: f64 = portions.iter().sum();
    if allocated < total_energy {
        if let Some((idx, _)) = portions
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        {
            portions[idx] += total_energy - allocated;
        }
    }
    portions
}

fn movement_direction(vx: f64, vy: f64, steer_x: f64, steer_y: f64, rng: &mut impl Rng) -> (f64, f64) {
    let speed = (vx * vx + vy * vy).sqrt();
    if speed > 8.0 {
        return (vx / speed, vy / speed);
    }
    let slen = (steer_x * steer_x + steer_y * steer_y).sqrt();
    if slen > 0.001 {
        return (steer_x / slen, steer_y / slen);
    }
    let angle = rng.gen_range(0.0..std::f64::consts::TAU);
    (angle.cos(), angle.sin())
}

fn spray_forward_bands(count: usize, rng: &mut impl Rng) -> Vec<f64> {
    let presets = [0.14, 0.26, 0.4, 0.55, 0.7, 0.84, 0.94];
    let mut bands: Vec<f64> = (0..count)
        .map(|i| {
            let base = presets[i % presets.len()];
            base + rng.gen_range(-0.06..0.06)
        })
        .collect();
    bands.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    for i in 0..bands.len() {
        bands[i] = bands[i].clamp(0.08, 0.98);
    }
    bands
}

fn spawn_directional_spray(
    world: &mut crate::world::World,
    origin_x: f64,
    origin_y: f64,
    dir_x: f64,
    dir_y: f64,
    source_radius: f64,
    total_energy: f64,
    source_color: (u8, u8, u8),
    rng: &mut impl Rng,
) {
    if total_energy <= MIN_DROP_ENERGY {
        return;
    }
    let count = drop_count_for_energy(total_energy, rng);
    let portions = partition_drop_energy(total_energy, count, rng);
    if portions.is_empty() {
        return;
    }

    let perp_x = -dir_y;
    let perp_y = dir_x;
    let max_spray = DEATH_SPRAY_BASE + source_radius * DEATH_SPRAY_RADIUS_SCALE;
    let lateral_base = DEATH_SPRAY_LATERAL_BASE + source_radius * DEATH_SPRAY_LATERAL_RADIUS_SCALE;
    let forward_bands = spray_forward_bands(portions.len(), rng);

    for (i, energy) in portions.iter().enumerate() {
        let forward_t = forward_bands[i] * rng.gen_range(0.86..1.14);
        let forward = forward_t * max_spray;

        let along = (forward / max_spray).clamp(0.0, 1.0);
        let cone = (along * std::f64::consts::PI).sin().max(0.12);
        let lateral =
            rng.gen_range(-1.0..1.0) * lateral_base * cone * rng.gen_range(0.35..1.55);
        let curl = rng.gen_range(-0.65..0.65) * source_radius * cone;
        let jitter_f = rng.gen_range(-source_radius * 0.2..source_radius * 0.24);

        let ox = dir_x * (forward + jitter_f) + perp_x * (lateral + curl);
        let oy = dir_y * (forward + jitter_f) + perp_y * (lateral + curl);

        push_remains(world, origin_x + ox, origin_y + oy, *energy, source_color);
    }
}

pub fn spawn_death_drops(
    world: &mut crate::world::World,
    x: f64,
    y: f64,
    total_energy: f64,
    source_radius: f64,
    source_color: (u8, u8, u8),
    vx: f64,
    vy: f64,
    steer_x: f64,
    steer_y: f64,
    rng: &mut impl Rng,
) {
    let drop_energy = total_energy * DEATH_DROP_FRACTION;
    let (dir_x, dir_y) = movement_direction(vx, vy, steer_x, steer_y, rng);
    spawn_directional_spray(
        world,
        x,
        y,
        dir_x,
        dir_y,
        source_radius,
        drop_energy,
        source_color,
        rng,
    );
}

pub fn spawn_eat_remains(
    world: &mut crate::world::World,
    eater_x: f64,
    eater_y: f64,
    eater_radius: f64,
    eaten_x: f64,
    eaten_y: f64,
    remain_energy: f64,
    source_color: (u8, u8, u8),
    rng: &mut impl Rng,
) {
    if remain_energy <= MIN_DROP_ENERGY {
        return;
    }

    let dx = eaten_x - eater_x;
    let dy = eaten_y - eater_y;
    let dist = (dx * dx + dy * dy).sqrt().max(1.0);
    let dir_x = dx / dist;
    let dir_y = dy / dist;
    let perp_x = -dir_y;
    let perp_y = dir_x;

    let base_dist = eater_radius + EAT_REMAIN_MIN_GAP + rng.gen_range(8.0..22.0);
    let base_x = eater_x + dir_x * base_dist;
    let base_y = eater_y + dir_y * base_dist;

    let count = drop_count_for_energy(remain_energy, rng);
    let portions = partition_drop_energy(remain_energy, count, rng);
    for energy in portions {
        let forward = rng.gen_range(0.0..EAT_REMAIN_FORWARD_SPREAD);
        let lateral = rng.gen_range(-EAT_REMAIN_LATERAL_SPREAD..EAT_REMAIN_LATERAL_SPREAD);
        let ox = dir_x * forward + perp_x * lateral;
        let oy = dir_y * forward + perp_y * lateral;
        push_remains(world, base_x + ox, base_y + oy, energy, source_color);
    }
}

fn push_remains(
    world: &mut crate::world::World,
    x: f64,
    y: f64,
    energy: f64,
    source_color: (u8, u8, u8),
) {
    let id = world.alloc_id();
    let mut remains = Bubble::new_remains(id, x, y, energy, source_color);
    crate::world::World::clamp_position(&mut remains, world.map_width, world.map_height);
    world.bubbles.push(remains);
}
