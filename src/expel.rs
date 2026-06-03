use crate::bubble::Bubble;
use crate::config::{
    EXPEL_RATE_BASE, EXPEL_RATE_SIZE_POWER, EXPEL_THRUST_COEFF, INITIAL_RADIUS, UNIT_ENERGY,
};
use crate::physics::{apply_expel_impulse, bubble_mass, can_use_expel};
use crate::world::World;
use rand::Rng;

/// 排空能量并返回本帧推力带来的速度增量（未排空则返回 None）
pub fn tick_expel(
    world: &mut World,
    bubble_id: u32,
    dir_x: f64,
    dir_y: f64,
    dt: f64,
    rng: &mut impl Rng,
) -> Option<(f64, f64)> {
    let idx = match world.bubbles.iter().position(|b| b.id == bubble_id) {
        Some(i) => i,
        None => return None,
    };

    if !can_use_expel(&world.bubbles[idx]) {
        world.bubbles[idx].expelling = false;
        return None;
    }

    let radius = world.bubbles[idx].radius();
    let size_ratio = (radius / INITIAL_RADIUS).max(0.1);
    let rate = EXPEL_RATE_BASE * size_ratio.powf(EXPEL_RATE_SIZE_POWER);
    let drain = rate * dt;
    if world.bubbles[idx].energy - drain < UNIT_ENERGY * 2.0 {
        world.bubbles[idx].expelling = false;
        return None;
    }

    world.bubbles[idx].energy -= drain;
    world.bubbles[idx].expelling = true;

    let len = (dir_x * dir_x + dir_y * dir_y).sqrt();
    let (fx, fy) = if len > 0.001 {
        (dir_x / len, dir_y / len)
    } else {
        (1.0, 0.0)
    };

    let (bx, by) = (-fx, -fy);
    let behind_dist = radius + crate::config::UNIT_RADIUS + 2.0;
    let ux = world.bubbles[idx].x + bx * behind_dist + rng.gen_range(-3.0..3.0);
    let uy = world.bubbles[idx].y + by * behind_dist + rng.gen_range(-3.0..3.0);
    let mut unit = Bubble::new_unit(world.alloc_id(), ux, uy, rng);
    unit.energy = drain.min(UNIT_ENERGY * 3.0);
    World::clamp_position(&mut unit, world.map_width, world.map_height);
    world.bubbles.push(unit);

    let mass = bubble_mass(&world.bubbles[idx]);
    let impulse = EXPEL_THRUST_COEFF * drain / mass;
    Some((fx * impulse, fy * impulse))
}

pub fn apply_expel_to_bubble(bubble: &mut Bubble, impulse: (f64, f64)) {
    apply_expel_impulse(bubble, impulse.0, impulse.1);
}

pub fn clear_expelling(world: &mut World, bubble_id: u32) {
    if let Some(b) = world.bubbles.iter_mut().find(|b| b.id == bubble_id) {
        b.expelling = false;
    }
}

pub fn ai_should_expel(rng: &mut impl Rng, radius: f64, fleeing: bool, eaten_non_unit: bool) -> bool {
    if !eaten_non_unit {
        return false;
    }
    if fleeing {
        return rng.gen_bool(0.15);
    }
    let size_factor = (radius / INITIAL_RADIUS).min(4.0);
    rng.gen_bool(0.02 * size_factor)
}
