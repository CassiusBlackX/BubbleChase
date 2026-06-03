use crate::bubble::{Bubble, BubbleKind};
use crate::collision::partition_drop_energy;
use crate::config::{
    AI_FLEE_EXPEL_CHANCE, AI_FISSION_LARGE_MAX_RATIO, AI_FISSION_LARGE_MIN_RATIO,
    AI_FISSION_ENERGY_RETAIN, AI_FISSION_MAX, AI_FISSION_MIN, AI_FISSION_RADIUS_THRESHOLD,
    AI_FISSION_REMAIN_LARGE_CHANCE,
    AI_FISSION_REPULSE_BASE, AI_FISSION_REPULSE_SCALE, AI_FISSION_SMALL_MAX_RATIO,
    AI_FISSION_SMALL_MIN_RATIO, AI_HARD_MAX, AI_HUNTER_SPAWN_CHANCE, AI_SPAWN_INTERVAL_MAX,
    AI_SPAWN_INTERVAL_MIN, AI_SOFT_MAX, AI_SOFT_MIN, AI_SURROUND_EXPEL_CHANCE, DEATH_DROP_MAX,
    DEATH_DROP_MIN, INITIAL_RADIUS, MIN_DROP_ENERGY, MIN_AI, UNIT_CLUSTER_SPAWN_BATCH,
    UNIT_CLUSTER_SPREAD, UNIT_CLUSTER_TIGHT_SPREAD, UNIT_ENERGY, UNIT_NEW_CLUSTER_CHANCE,
    UNIT_SCATTER_SPAWN_BATCH, UNIT_SCATTER_SPAWN_CHANCE, UNIT_SPAWN_INTERVAL,
};
use crate::world::World;
use rand::Rng;

pub fn roll_ai_spawn_delay(rng: &mut impl Rng) -> f64 {
    rng.gen_range(AI_SPAWN_INTERVAL_MIN..AI_SPAWN_INTERVAL_MAX)
}

pub fn tick_ai_spawning(world: &mut World, dt: f64, rng: &mut impl Rng) {
    world.ai_spawn_timer -= dt;
    if world.ai_spawn_timer > 0.0 {
        return;
    }
    world.ai_spawn_timer = roll_ai_spawn_delay(rng);

    let count = world.ai_count();
    if count >= AI_HARD_MAX {
        return;
    }

    if rng.gen_bool(AI_HUNTER_SPAWN_CHANCE * 0.55) && count < AI_HARD_MAX {
        world.spawn_hunter_ai(rng);
    }

    let chance = ai_spawn_chance(count, rng);
    if !rng.gen_bool(chance) {
        return;
    }

    let batch = ai_spawn_batch(count, rng);
    for _ in 0..batch {
        if world.ai_count() >= AI_HARD_MAX {
            break;
        }
        world.spawn_ai_bubble(rng);
    }
}

fn ai_spawn_chance(count: usize, rng: &mut impl Rng) -> f64 {
    if count < AI_SOFT_MIN {
        return 1.0;
    }
    if count < MIN_AI {
        return 0.9;
    }
    if count >= AI_SOFT_MAX {
        return rng.gen_range(0.28..0.52);
    }
    let span = (AI_SOFT_MAX - MIN_AI).max(1) as f64;
    let t = ((count - MIN_AI) as f64 / span).clamp(0.0, 1.0);
    let base = 0.85 * (1.0 - t * 0.45);
    (base + rng.gen_range(-0.06..0.10)).clamp(0.18, 0.96)
}

fn ai_spawn_batch(count: usize, rng: &mut impl Rng) -> usize {
    if count < AI_SOFT_MIN {
        return rng.gen_range(2..=4);
    }
    if count < MIN_AI {
        return if rng.gen_bool(0.55) { 2 } else { 1 };
    }
    if count < AI_SOFT_MAX && rng.gen_bool(0.32) {
        return 2;
    }
    1
}

pub fn tick_unit_spawning(world: &mut World, dt: f64, rng: &mut impl Rng) {
    world.unit_spawn_timer += dt;
    while world.unit_spawn_timer >= UNIT_SPAWN_INTERVAL {
        world.unit_spawn_timer -= UNIT_SPAWN_INTERVAL;

        if rng.gen_bool(UNIT_SCATTER_SPAWN_CHANCE) {
            let count = UNIT_SCATTER_SPAWN_BATCH + rng.gen_range(0..2);
            for _ in 0..count {
                world.spawn_scattered_unit(rng);
            }
        } else if rng.gen_bool(UNIT_NEW_CLUSTER_CHANCE) {
            let (cx, cy) = world.random_point(rng, 40.0);
            world.add_cluster_at(cx, cy);
            spawn_cluster_batch(
                world,
                cx,
                cy,
                UNIT_CLUSTER_SPAWN_BATCH + 1,
                UNIT_CLUSTER_TIGHT_SPREAD,
                rng,
            );
        } else {
            let (cx, cy) = world.pick_cluster(rng);
            spawn_cluster_batch(
                world,
                cx,
                cy,
                UNIT_CLUSTER_SPAWN_BATCH,
                UNIT_CLUSTER_SPREAD,
                rng,
            );
        }
    }
}

fn spawn_cluster_batch(
    world: &mut World,
    cx: f64,
    cy: f64,
    count: usize,
    spread: f64,
    rng: &mut impl Rng,
) {
    for _ in 0..count {
        let (x, y) = world.cluster_offset(cx, cy, spread, rng);
        let id = world.alloc_id();
        world.bubbles.push(Bubble::new_unit(id, x, y, rng));
    }
}

/// AI 被更大泡泡围堵时是否应排空加速逃离
pub fn ai_expel_when_trapped(rng: &mut impl Rng, surrounded: bool, fleeing: bool, smart: bool) -> bool {
    if surrounded {
        let p = if smart {
            AI_SURROUND_EXPEL_CHANCE + 0.12
        } else {
            AI_SURROUND_EXPEL_CHANCE
        };
        return rng.gen_bool(p.clamp(0.0, 0.95));
    }
    if fleeing {
        let p = if smart {
            AI_FLEE_EXPEL_CHANCE + 0.15
        } else {
            AI_FLEE_EXPEL_CHANCE
        };
        return rng.gen_bool(p.clamp(0.0, 0.9));
    }
    false
}

/// 达到玩家最终大小的 AI 立刻裂解为多个子 AI，并赋予斥力初速。
pub fn fission_oversized_ai(world: &mut World, rng: &mut impl Rng) {
    let max_radius = world.player_win_radius();
    let threshold = max_radius * AI_FISSION_RADIUS_THRESHOLD;
    let ids: Vec<u32> = world
        .bubbles
        .iter()
        .filter(|b| b.kind == BubbleKind::Ai && !b.suicide && b.radius() >= threshold)
        .map(|b| b.id)
        .collect();

    for id in ids {
        fission_one_ai(world, id, max_radius, rng);
    }
}

fn fission_one_ai(world: &mut World, parent_id: u32, max_radius: f64, rng: &mut impl Rng) {
    let parent = match world.bubbles.iter().find(|b| b.id == parent_id) {
        Some(b) => b.clone(),
        None => return,
    };

    let player_radius = world
        .player()
        .map(|p| p.radius())
        .unwrap_or(INITIAL_RADIUS)
        .max(INITIAL_RADIUS * 0.5);

    let output_energy = parent.energy * AI_FISSION_ENERGY_RETAIN;
    let count = rng.gen_range(AI_FISSION_MIN..=AI_FISSION_MAX);
    let (portions, leftover) =
        partition_fission_children(output_energy, count, player_radius, max_radius, rng);
    let mut portions = portions;
    if portions.len() < 2 {
        portions = vec![output_energy * 0.45, output_energy * 0.55];
    }

    world.remove_by_id(parent_id);

    if leftover > MIN_DROP_ENERGY {
        spawn_fission_remains(
            world,
            parent.x,
            parent.y,
            parent.radius(),
            leftover,
            parent.color,
            rng,
        );
    }

    let parent_radius = parent.radius();
    let repulse = AI_FISSION_REPULSE_BASE + parent_radius * AI_FISSION_REPULSE_SCALE;
    let base_angle = rng.gen_range(0.0..std::f64::consts::TAU);
    let n = portions.len();
    let mut dirs: Vec<(f64, f64)> = Vec::with_capacity(n);
    for i in 0..n {
        let angle = base_angle + std::f64::consts::TAU * i as f64 / n as f64 + rng.gen_range(-0.32..0.32);
        dirs.push((angle.cos(), angle.sin()));
    }

    for (i, &energy) in portions.iter().enumerate() {
        let mut push_x = dirs[i].0 * 1.6;
        let mut push_y = dirs[i].1 * 1.6;
        for j in 0..n {
            if i == j {
                continue;
            }
            push_x += dirs[i].0 - dirs[j].0;
            push_y += dirs[i].1 - dirs[j].1;
        }
        let plen = (push_x * push_x + push_y * push_y).sqrt().max(0.001);
        push_x /= plen;
        push_y /= plen;

        let child_r = (energy / std::f64::consts::PI).sqrt();
        let spawn_dist = parent_radius * 0.24 + child_r * 0.72;
        let x = parent.x + dirs[i].0 * spawn_dist;
        let y = parent.y + dirs[i].1 * spawn_dist;

        let speed = repulse * rng.gen_range(0.92..1.18);
        let vx = push_x * speed;
        let vy = push_y * speed;

        let id = world.alloc_id();
        let mut child = Bubble::new_ai_from_fission(id, x, y, energy, max_radius, &parent, rng);
        child.vx = vx;
        child.vy = vy;
        child.steer_x = push_x;
        child.steer_y = push_y;
        child.ai_intent_x = push_x;
        child.ai_intent_y = push_y;
        World::clamp_position(&mut child, world.map_width, world.map_height);
        world.bubbles.push(child);
    }
}

/// 按相对玩家体型分配裂解子 AI 能量；多数偏小，小概率保留一个比玩家大的威胁子体。
fn partition_fission_children(
    total_energy: f64,
    count: usize,
    player_radius: f64,
    max_radius: f64,
    rng: &mut impl Rng,
) -> (Vec<f64>, f64) {
    if count == 0 {
        return (Vec::new(), total_energy);
    }

    let remain_large = rng.gen_bool(AI_FISSION_REMAIN_LARGE_CHANCE);
    let large_idx = if remain_large {
        Some(rng.gen_range(0..count))
    } else {
        None
    };
    let fission_cap = max_radius * AI_FISSION_RADIUS_THRESHOLD * 0.97;

    let mut portions = Vec::with_capacity(count);
    for i in 0..count {
        let radius = if Some(i) == large_idx {
            let min_r = player_radius * AI_FISSION_LARGE_MIN_RATIO;
            let max_r = (player_radius * AI_FISSION_LARGE_MAX_RATIO).min(fission_cap);
            if min_r < max_r {
                rng.gen_range(min_r..max_r)
            } else {
                player_radius * rng.gen_range(AI_FISSION_SMALL_MIN_RATIO..AI_FISSION_SMALL_MAX_RATIO)
            }
        } else {
            player_radius * rng.gen_range(AI_FISSION_SMALL_MIN_RATIO..AI_FISSION_SMALL_MAX_RATIO)
        }
        .min(fission_cap);
        let jitter = rng.gen_range(0.92..1.06);
        let energy = std::f64::consts::PI * radius * radius * jitter;
        portions.push(energy.max(MIN_DROP_ENERGY));
    }

    let allocated: f64 = portions.iter().sum();
    if allocated > total_energy {
        let scale = total_energy / allocated;
        for portion in &mut portions {
            *portion *= scale;
        }
        return (portions, 0.0);
    }
    (portions, total_energy - allocated)
}

fn spawn_fission_remains(
    world: &mut World,
    x: f64,
    y: f64,
    radius: f64,
    total_energy: f64,
    color: (u8, u8, u8),
    rng: &mut impl Rng,
) {
    let count = if total_energy <= UNIT_ENERGY * 3.0 {
        rng.gen_range(2..=4)
    } else {
        rng.gen_range(DEATH_DROP_MIN..=DEATH_DROP_MAX)
    };
    let portions = partition_drop_energy(total_energy, count, rng);
    for energy in portions {
        let angle = rng.gen_range(0.0..std::f64::consts::TAU);
        let dist = radius * rng.gen_range(0.35..1.15);
        let id = world.alloc_id();
        let mut remains = Bubble::new_remains(
            id,
            x + angle.cos() * dist,
            y + angle.sin() * dist,
            energy,
            color,
        );
        World::clamp_position(&mut remains, world.map_width, world.map_height);
        world.bubbles.push(remains);
    }
}
