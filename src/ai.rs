use crate::bubble::{Bubble, BubbleKind};
use crate::config::{
    AI_FLEE_RATIO, AI_FORAGE_MAX, AI_FORAGE_MIN, AI_HUNTER_PLAYER_COOLDOWN_MAX,
    AI_HUNTER_PLAYER_COOLDOWN_MIN, AI_INTENT_INTERVAL, AI_MAX_TURN_RATE, AI_MISTAKE_RATE,
    AI_SENSE_RADIUS, AI_SMART_INTENT_INTERVAL, AI_SMART_MISTAKE_RATE, AI_SMART_SENSE_RADIUS,
    AI_SPIN_CHANCE, AI_SPIN_MAX, AI_SPIN_MIN, AI_VISIBLE_RADIUS, AI_VIEWPORT_PULL_INNER,
    AI_VIEWPORT_PULL_OUTER, AI_WALL_BAND_EXTRA, AI_WALL_CENTER_BONUS, AI_WALL_DEPART_CHANCE_BASE,
    AI_WALL_DEPART_CHANCE_MAX, AI_WALL_DEPART_INTENT_MIN, AI_WALL_HUG_FORCE_AFTER,
    AI_WALL_HUG_SLIDE_DOT, AI_WALL_INWARD_BONUS, AI_WALL_NEAR, AI_WALL_PENALTY_MAX,
    AI_WALL_SLIDE_BLEND_MAX, AI_WALL_SLIDE_BLEND_MIN, AI_WALL_SLIDE_PENALTY,
    AI_WALL_VIEWPORT_DAMP, EAT_RATIO, UNIT_ENERGY,
};
use crate::expel::{ai_should_expel, apply_expel_to_bubble, clear_expelling, tick_expel};
use crate::physics::{apply_movement_smoothed, apply_spin_in_place, camera_offset};
use crate::spawn::ai_expel_when_trapped;
use crate::world::World;
use rand::Rng;

#[derive(Clone, Copy)]
struct AiDecision {
    dir_x: f64,
    dir_y: f64,
    expel: bool,
    spinning: bool,
    max_turn_rate: f64,
}

struct AiSnapshot {
    id: u32,
    x: f64,
    y: f64,
    radius: f64,
    eaten_non_unit: bool,
    suicide: bool,
    smart: bool,
    hunter: bool,
    forage_timer: f64,
    spin_timer: f64,
    intent_x: f64,
    intent_y: f64,
    wall_hug_timer: f64,
}

pub fn tick_ai(world: &mut World, dt: f64, rng: &mut impl Rng) {
    let all: Vec<(u32, f64, f64, f64, BubbleKind, bool)> = world
        .bubbles
        .iter()
        .map(|b| (b.id, b.x, b.y, b.radius(), b.kind, b.eaten_non_unit))
        .collect();

    let player_pos = world.player().map(|p| (p.x, p.y, p.radius()));
    let map_w = world.map_width;
    let map_h = world.map_height;
    let viewport_w = world.viewport_w;
    let viewport_h = world.viewport_h;

    tick_wall_hug_timers(world, dt, map_w, map_h);

    for b in world.bubbles.iter_mut().filter(|b| b.kind == BubbleKind::Ai) {
        b.ai_intent_timer -= dt;
        if b.ai_spin_timer > 0.0 {
            b.ai_spin_timer -= dt;
        }
        if b.ai_forage_timer > 0.0 {
            b.ai_forage_timer -= dt;
        }
    }

    let snapshots: Vec<AiSnapshot> = world
        .bubbles
        .iter()
        .filter(|b| b.kind == BubbleKind::Ai)
        .map(|b| AiSnapshot {
            id: b.id,
            x: b.x,
            y: b.y,
            radius: b.radius(),
            eaten_non_unit: b.eaten_non_unit,
            suicide: b.suicide,
            smart: b.ai_smart,
            hunter: b.ai_hunter,
            forage_timer: b.ai_forage_timer,
            spin_timer: b.ai_spin_timer,
            intent_x: b.ai_intent_x,
            intent_y: b.ai_intent_y,
            wall_hug_timer: b.ai_wall_hug_timer,
        })
        .collect();

    let mut decisions: Vec<(u32, AiDecision)> = Vec::new();

    for snap in &snapshots {
        let decision = if snap.suicide {
            let mut dec = compute_suicide_decision(snap.x, snap.y, player_pos, rng, snap.radius);
            finalize_wall_direction(&mut dec, snap, world, map_w, map_h, rng);
            dec
        } else if snap.spin_timer > 0.0 {
            AiDecision {
                dir_x: snap.intent_x,
                dir_y: snap.intent_y,
                expel: false,
                spinning: true,
                max_turn_rate: AI_MAX_TURN_RATE,
            }
        } else if snap.forage_timer <= 0.0 && should_hunt_player(snap, player_pos) {
            if tick_chase_interest(world, snap.id, dt, rng, snap.hunter) {
                let mut dec = compute_hunt_player_decision(snap, player_pos, rng);
                finalize_wall_direction(&mut dec, snap, world, map_w, map_h, rng);
                dec
            } else {
                update_and_get_intent(
                    world,
                    snap,
                    &all,
                    player_pos,
                    map_w,
                    map_h,
                    viewport_w,
                    viewport_h,
                    dt,
                    rng,
                )
            }
        } else {
            update_and_get_intent(
                world,
                snap,
                &all,
                player_pos,
                map_w,
                map_h,
                viewport_w,
                viewport_h,
                dt,
                rng,
            )
        };
        decisions.push((snap.id, decision));
    }

    for (id, dec) in decisions {
        if dec.spinning {
            if let Some(b) = world.bubbles.iter_mut().find(|b| b.id == id) {
                apply_spin_in_place(b, dt, map_w, map_h);
            }
            continue;
        }
        if dec.expel {
            if let Some(b) = world.bubbles.iter_mut().find(|b| b.id == id) {
                apply_movement_smoothed(
                    b,
                    dec.dir_x,
                    dec.dir_y,
                    dt,
                    true,
                    map_w,
                    map_h,
                    dec.max_turn_rate,
                );
            }
            if let Some(imp) = tick_expel(world, id, dec.dir_x, dec.dir_y, dt, rng) {
                if let Some(b) = world.bubbles.iter_mut().find(|b| b.id == id) {
                    apply_expel_to_bubble(b, imp);
                    crate::physics::clamp_bubble_speed(b, true);
                }
            }
        } else {
            clear_expelling(world, id);
            if let Some(b) = world.bubbles.iter_mut().find(|b| b.id == id) {
                apply_movement_smoothed(
                    b,
                    dec.dir_x,
                    dec.dir_y,
                    dt,
                    false,
                    map_w,
                    map_h,
                    dec.max_turn_rate,
                );
            }
        }
    }
}

fn tick_chase_interest(
    world: &mut World,
    id: u32,
    dt: f64,
    rng: &mut impl Rng,
    is_hunter: bool,
) -> bool {
    if let Some(b) = world.bubbles.iter_mut().find(|b| b.id == id) {
        b.ai_interest_elapsed += dt;
        if b.ai_interest_elapsed >= b.ai_interest_limit {
            b.ai_forage_timer = if is_hunter {
                rng.gen_range(AI_HUNTER_PLAYER_COOLDOWN_MIN..AI_HUNTER_PLAYER_COOLDOWN_MAX)
            } else {
                rng.gen_range(AI_FORAGE_MIN..AI_FORAGE_MAX)
            };
            b.ai_interest_elapsed = 0.0;
            b.ai_interest_limit = Bubble::roll_interest_limit(rng);
            b.ai_hunter_player_cooldown = b.ai_forage_timer;
            return false;
        }
    }
    true
}

fn should_hunt_player(snap: &AiSnapshot, player_pos: Option<(f64, f64, f64)>) -> bool {
    let Some((_, _, pr)) = player_pos else {
        return false;
    };
    if snap.forage_timer > 0.0 {
        return false;
    }
    if snap.hunter {
        return snap.radius > pr * EAT_RATIO;
    }
    snap.smart
        && snap.eaten_non_unit
        && snap.radius > pr * 1.12
        && snap.radius < pr * 1.55
}

fn compute_hunt_player_decision(
    snap: &AiSnapshot,
    player_pos: Option<(f64, f64, f64)>,
    rng: &mut impl Rng,
) -> AiDecision {
    let Some((px, py, _)) = player_pos else {
        return AiDecision {
            dir_x: 1.0,
            dir_y: 0.0,
            expel: true,
            spinning: false,
            max_turn_rate: AI_MAX_TURN_RATE * 1.2,
        };
    };
    let dx = px - snap.x;
    let dy = py - snap.y;
    let dist = (dx * dx + dy * dy).sqrt().max(0.001);
    let base_angle = dy.atan2(dx);
    let flank = ((snap.id % 5) as f64 - 2.0) * 0.42;
    let angle = base_angle + flank;
    let dir_x = angle.cos();
    let dir_y = angle.sin();
    let close = dist < snap.radius * 3.5;
    AiDecision {
        dir_x,
        dir_y,
        expel: close && rng.gen_bool(0.35),
        spinning: false,
        max_turn_rate: AI_MAX_TURN_RATE * 1.28,
    }
}

fn update_and_get_intent(
    world: &mut World,
    snap: &AiSnapshot,
    all: &[(u32, f64, f64, f64, BubbleKind, bool)],
    player_pos: Option<(f64, f64, f64)>,
    map_w: f64,
    map_h: f64,
    viewport_w: f64,
    viewport_h: f64,
    dt: f64,
    rng: &mut impl Rng,
) -> AiDecision {
    let sense = if snap.smart {
        AI_SMART_SENSE_RADIUS
    } else {
        AI_SENSE_RADIUS
    };
    let flee = compute_flee_vector(snap.id, snap.x, snap.y, snap.radius, all, sense, snap.smart);
    let flee_mag = (flee.0 * flee.0 + flee.1 * flee.1).sqrt();
    let surrounded = is_surrounded_by_larger(snap.id, snap.x, snap.y, snap.radius, all, sense);
    let flee_threshold = if snap.smart { 0.35 } else { 0.5 };

    if flee_mag > flee_threshold || surrounded {
        let (fx, fy) = if flee_mag > 0.001 {
            (flee.0 / flee_mag, flee.1 / flee_mag)
        } else {
            escape_from_crowd_direction(snap.x, snap.y, snap.radius, all, snap.id)
        };
        if let Some(b) = world.bubbles.iter_mut().find(|b| b.id == snap.id) {
            b.ai_intent_x = fx;
            b.ai_intent_y = fy;
            b.ai_intent_timer = AI_INTENT_INTERVAL * 0.3;
        }
        let mut dec = AiDecision {
            dir_x: fx,
            dir_y: fy,
            expel: snap.eaten_non_unit
                && (ai_expel_when_trapped(rng, surrounded, true, snap.smart)
                    || ai_should_expel(rng, snap.radius, true, snap.eaten_non_unit)),
            spinning: false,
            max_turn_rate: AI_MAX_TURN_RATE * if snap.smart { 1.3 } else { 1.15 },
        };
        finalize_wall_direction(&mut dec, snap, world, map_w, map_h, rng);
        return dec;
    }

    if snap.forage_timer > 0.0 {
        if let Some(dec) = try_forage_decision(
            snap,
            all,
            player_pos,
            map_w,
            map_h,
            viewport_w,
            viewport_h,
            world,
            rng,
        ) {
            return dec;
        }
    }

    let prey = compute_hunt_prey_vector(snap, all);
    let prey_mag = (prey.0 * prey.0 + prey.1 * prey.1).sqrt();
    let prey_threshold = 0.45;
    if snap.forage_timer <= 0.0 && prey_mag > prey_threshold && snap.eaten_non_unit {
        if tick_chase_interest(world, snap.id, dt, rng, snap.hunter) {
            let (hx, hy) = (prey.0 / prey_mag, prey.1 / prey_mag);
            let mut dec = AiDecision {
                dir_x: hx,
                dir_y: hy,
                expel: false,
                spinning: false,
                max_turn_rate: AI_MAX_TURN_RATE,
            };
            finalize_wall_direction(&mut dec, snap, world, map_w, map_h, rng);
            return dec;
        }
        if let Some(dec) = try_forage_decision(
            snap,
            all,
            player_pos,
            map_w,
            map_h,
            viewport_w,
            viewport_h,
            world,
            rng,
        ) {
            return dec;
        }
    }

    let intent_interval = if snap.smart {
        AI_SMART_INTENT_INTERVAL
    } else {
        AI_INTENT_INTERVAL
    };

    let needs_refresh = world
        .bubbles
        .iter()
        .find(|b| b.id == snap.id)
        .map(|b| b.ai_intent_timer <= 0.0)
        .unwrap_or(true);

    if needs_refresh {
        let (ix, iy) = if snap.wall_hug_timer > 0.0 && should_depart_wall(snap.wall_hug_timer, rng) {
            depart_wall_direction(snap.x, snap.y, snap.radius, map_w, map_h)
        } else {
            compute_purposeful_intent(
                snap.id,
                snap.x,
                snap.y,
                snap.radius,
                snap.eaten_non_unit,
                snap.smart,
                snap.forage_timer > 0.0,
                all,
                map_w,
                map_h,
                viewport_w,
                viewport_h,
                player_pos,
                snap.forage_timer <= 0.0,
                rng,
            )
        };
        if let Some(b) = world.bubbles.iter_mut().find(|b| b.id == snap.id) {
            let blend = if snap.smart { 0.62 } else { 0.72 };
            let nx = b.ai_intent_x * (1.0 - blend) + ix * blend;
            let ny = b.ai_intent_y * (1.0 - blend) + iy * blend;
            let nlen = (nx * nx + ny * ny).sqrt().max(0.001);
            b.ai_intent_x = nx / nlen;
            b.ai_intent_y = ny / nlen;
            b.ai_intent_timer = intent_interval + rng.gen_range(-0.2..0.35);

            if !snap.smart && rng.gen_bool(AI_SPIN_CHANCE) {
                b.ai_spin_timer = rng.gen_range(AI_SPIN_MIN..AI_SPIN_MAX);
            }
        }
    }

    let (ix, iy) = world
        .bubbles
        .iter()
        .find(|b| b.id == snap.id)
        .map(|b| (b.ai_intent_x, b.ai_intent_y))
        .unwrap_or((1.0, 0.0));

    let mut dx = ix;
    let mut dy = iy;
    if flee_mag > 0.05 {
        let blend = flee_mag.min(if snap.smart { 0.5 } else { 0.35 });
        dx = ix * (1.0 - blend) + flee.0 * blend;
        dy = iy * (1.0 - blend) + flee.1 * blend;
    }

    let (vx, vy, vp_w) = viewport_pull_direction(
        snap.id,
        snap.x,
        snap.y,
        player_pos,
        viewport_w,
        viewport_h,
        map_w,
        map_h,
    );
    if vp_w > 0.0 {
        let left = snap.x - snap.radius;
        let right = map_w - snap.x - snap.radius;
        let top = snap.y - snap.radius;
        let bottom = map_h - snap.y - snap.radius;
        let nearest = left.min(right).min(top).min(bottom);
        let effective_vp = if nearest < AI_WALL_NEAR {
            vp_w * AI_WALL_VIEWPORT_DAMP
        } else {
            vp_w
        };
        dx = dx * (1.0 - effective_vp) + vx * effective_vp;
        dy = dy * (1.0 - effective_vp) + vy * effective_vp;
    }

    let len = (dx * dx + dy * dy).sqrt().max(0.001);

    let mut dec = AiDecision {
        dir_x: dx / len,
        dir_y: dy / len,
        expel: snap.eaten_non_unit && ai_should_expel(rng, snap.radius, flee_mag > 0.15, snap.eaten_non_unit),
        spinning: false,
        max_turn_rate: AI_MAX_TURN_RATE,
    };
    finalize_wall_direction(&mut dec, snap, world, map_w, map_h, rng);
    dec
}

fn tick_wall_hug_timers(world: &mut World, dt: f64, map_w: f64, map_h: f64) {
    for b in world.bubbles.iter_mut().filter(|b| b.kind == BubbleKind::Ai) {
        let slen = (b.steer_x * b.steer_x + b.steer_y * b.steer_y).sqrt();
        let hugging = if slen > 0.001 {
            is_wall_hugging(
                b.x,
                b.y,
                b.radius(),
                b.steer_x / slen,
                b.steer_y / slen,
                map_w,
                map_h,
            )
        } else {
            false
        };
        if hugging {
            b.ai_wall_hug_timer += dt;
        } else {
            b.ai_wall_hug_timer = 0.0;
        }
    }
}

fn is_wall_hugging(
    x: f64,
    y: f64,
    radius: f64,
    dir_x: f64,
    dir_y: f64,
    map_w: f64,
    map_h: f64,
) -> bool {
    let left = x - radius;
    let right = map_w - x - radius;
    let top = y - radius;
    let bottom = map_h - y - radius;
    let nearest = left.min(right).min(top).min(bottom);
    if nearest > AI_WALL_NEAR {
        return false;
    }
    let (in_x, in_y) = inward_normal(left, right, top, bottom);
    dir_x * in_x + dir_y * in_y < AI_WALL_HUG_SLIDE_DOT
}

fn should_depart_wall(hug_timer: f64, rng: &mut impl Rng) -> bool {
    if hug_timer <= 0.0 {
        return false;
    }
    if hug_timer >= AI_WALL_HUG_FORCE_AFTER {
        return true;
    }
    let t = (hug_timer / AI_WALL_HUG_FORCE_AFTER).clamp(0.0, 1.0);
    let chance =
        AI_WALL_DEPART_CHANCE_BASE + t * (AI_WALL_DEPART_CHANCE_MAX - AI_WALL_DEPART_CHANCE_BASE);
    rng.gen_bool(chance)
}

fn depart_wall_direction(x: f64, y: f64, radius: f64, map_w: f64, map_h: f64) -> (f64, f64) {
    let left = x - radius;
    let right = map_w - x - radius;
    let top = y - radius;
    let bottom = map_h - y - radius;

    let edge_push = |dist: f64| -> f64 { (1.0 - dist / AI_WALL_NEAR).clamp(0.0, 1.0) };

    let mut wx = 0.0;
    let mut wy = 0.0;
    if left < AI_WALL_NEAR {
        wx += edge_push(left);
    }
    if right < AI_WALL_NEAR {
        wx -= edge_push(right);
    }
    if top < AI_WALL_NEAR {
        wy += edge_push(top);
    }
    if bottom < AI_WALL_NEAR {
        wy -= edge_push(bottom);
    }

    let cx = map_w * 0.5 - x;
    let cy = map_h * 0.5 - y;
    let clen = (cx * cx + cy * cy).sqrt().max(1.0);

    let mx = wx * 0.72 + (cx / clen) * 0.28;
    let my = wy * 0.72 + (cy / clen) * 0.28;
    let mlen = (mx * mx + my * my).sqrt().max(0.001);
    (mx / mlen, my / mlen)
}

fn finalize_wall_direction(
    dec: &mut AiDecision,
    snap: &AiSnapshot,
    world: &mut World,
    map_w: f64,
    map_h: f64,
    rng: &mut impl Rng,
) {
    let hugging = is_wall_hugging(
        snap.x,
        snap.y,
        snap.radius,
        dec.dir_x,
        dec.dir_y,
        map_w,
        map_h,
    );

    if hugging {
        let depart = if snap.wall_hug_timer > 0.0 {
            should_depart_wall(snap.wall_hug_timer, rng)
        } else {
            rng.gen_bool(AI_WALL_DEPART_CHANCE_BASE)
        };
        if depart {
            let (dx, dy) = depart_wall_direction(snap.x, snap.y, snap.radius, map_w, map_h);
            dec.dir_x = dx;
            dec.dir_y = dy;
            dec.max_turn_rate = dec.max_turn_rate.max(AI_MAX_TURN_RATE * 1.35);
            if let Some(b) = world.bubbles.iter_mut().find(|b| b.id == snap.id) {
                b.ai_intent_x = dx;
                b.ai_intent_y = dy;
                b.ai_intent_timer = AI_WALL_DEPART_INTENT_MIN + rng.gen_range(0.0..0.5);
                b.ai_wall_hug_timer = 0.0;
                b.steer_x = dx;
                b.steer_y = dy;
            }
            return;
        }
    }

    anti_wall_slide(dec, snap.x, snap.y, snap.radius, map_w, map_h);
}

/// 检测贴边滑行：靠近边界且移动方向与“进地图内部”几乎垂直 → 改向地图内部
fn anti_wall_slide(
    dec: &mut AiDecision,
    x: f64,
    y: f64,
    radius: f64,
    map_w: f64,
    map_h: f64,
) {
    let left = x - radius;
    let right = map_w - x - radius;
    let top = y - radius;
    let bottom = map_h - y - radius;
    let nearest = left.min(right).min(top).min(bottom);

    if nearest > AI_WALL_NEAR {
        return;
    }

    let (in_x, in_y) = inward_normal(left, right, top, bottom);
    let inward_dot = dec.dir_x * in_x + dec.dir_y * in_y;

    if inward_dot > 0.62 {
        return;
    }

    let slide_strength = (1.0 - nearest / AI_WALL_NEAR).clamp(0.0, 1.0);
    let blend_range = AI_WALL_SLIDE_BLEND_MAX - AI_WALL_SLIDE_BLEND_MIN;
    let blend = (AI_WALL_SLIDE_BLEND_MIN + slide_strength * blend_range).min(AI_WALL_SLIDE_BLEND_MAX);
    let cx = map_w * 0.5 - x;
    let cy = map_h * 0.5 - y;
    let clen = (cx * cx + cy * cy).sqrt().max(1.0);
    let mx = in_x * 0.68 + (cx / clen) * 0.32;
    let my = in_y * 0.68 + (cy / clen) * 0.32;
    let mlen = (mx * mx + my * my).sqrt().max(0.001);

    let nx = dec.dir_x * (1.0 - blend) + (mx / mlen) * blend;
    let ny = dec.dir_y * (1.0 - blend) + (my / mlen) * blend;
    let nlen = (nx * nx + ny * ny).sqrt().max(0.001);
    dec.dir_x = nx / nlen;
    dec.dir_y = ny / nlen;
}

fn inward_normal(left: f64, right: f64, top: f64, bottom: f64) -> (f64, f64) {
    let nearest = left.min(right).min(top).min(bottom);
    if nearest == left {
        (1.0, 0.0)
    } else if nearest == right {
        (-1.0, 0.0)
    } else if nearest == top {
        (0.0, 1.0)
    } else {
        (0.0, -1.0)
    }
}

fn is_surrounded_by_larger(
    self_id: u32,
    x: f64,
    y: f64,
    radius: f64,
    all: &[(u32, f64, f64, f64, BubbleKind, bool)],
    sense: f64,
) -> bool {
    let mut quadrants = [false; 4];
    for &(oid, ox, oy, or, kind, _) in all {
        if oid == self_id {
            continue;
        }
        if or <= radius * AI_FLEE_RATIO {
            continue;
        }
        let dx = ox - x;
        let dy = oy - y;
        let d = (dx * dx + dy * dy).sqrt();
        if d > sense * 0.75 || d < 0.001 {
            continue;
        }
        let q = if dx >= 0.0 && dy >= 0.0 {
            0
        } else if dx < 0.0 && dy >= 0.0 {
            1
        } else if dx < 0.0 && dy < 0.0 {
            2
        } else {
            3
        };
        quadrants[q] = true;
        let _ = kind;
    }
    quadrants.iter().filter(|&&v| v).count() >= 2
}

fn escape_from_crowd_direction(
    x: f64,
    y: f64,
    radius: f64,
    all: &[(u32, f64, f64, f64, BubbleKind, bool)],
    self_id: u32,
) -> (f64, f64) {
    let mut fx = 0.0;
    let mut fy = 0.0;
    for &(oid, ox, oy, or, _, _) in all {
        if oid == self_id || or <= radius * AI_FLEE_RATIO {
            continue;
        }
        let dx = x - ox;
        let dy = y - oy;
        let d = (dx * dx + dy * dy).sqrt();
        if d < 0.001 {
            continue;
        }
        fx += dx / d;
        fy += dy / d;
    }
    let len = (fx * fx + fy * fy).sqrt();
    if len > 0.001 {
        (fx / len, fy / len)
    } else {
        (1.0, 0.0)
    }
}

fn dist_to_viewport_rect(x: f64, y: f64, vx0: f64, vy0: f64, vw: f64, vh: f64) -> f64 {
    let dx = if x < vx0 {
        vx0 - x
    } else if x > vx0 + vw {
        x - vx0 - vw
    } else {
        0.0
    };
    let dy = if y < vy0 {
        vy0 - y
    } else if y > vy0 + vh {
        y - vy0 - vh
    } else {
        0.0
    };
    (dx * dx + dy * dy).sqrt()
}

fn viewport_pull_direction(
    id: u32,
    x: f64,
    y: f64,
    player_pos: Option<(f64, f64, f64)>,
    viewport_w: f64,
    viewport_h: f64,
    map_w: f64,
    map_h: f64,
) -> (f64, f64, f64) {
    let Some((px, py, _)) = player_pos else {
        return (0.0, 0.0, 0.0);
    };
    let (cam_x, cam_y) = camera_offset(px, py, viewport_w, viewport_h, map_w, map_h);
    let ox = ((id % 7) as f64 / 7.0 - 0.5) * viewport_w * 0.5;
    let oy = (((id / 7) % 5) as f64 / 5.0 - 0.5) * viewport_h * 0.5;
    let tx = cam_x + viewport_w * 0.5 + ox;
    let ty = cam_y + viewport_h * 0.5 + oy;

    let in_view =
        x >= cam_x && x <= cam_x + viewport_w && y >= cam_y && y <= cam_y + viewport_h;
    let dx = tx - x;
    let dy = ty - y;
    let dist = (dx * dx + dy * dy).sqrt().max(1.0);

    let weight = if in_view {
        AI_VIEWPORT_PULL_INNER
    } else {
        let outside = dist_to_viewport_rect(x, y, cam_x, cam_y, viewport_w, viewport_h);
        (0.18 + outside / viewport_w.max(viewport_h) * 0.55).min(AI_VIEWPORT_PULL_OUTER)
    };
    (dx / dist, dy / dist, weight)
}

fn compute_forage_vector(
    snap: &AiSnapshot,
    all: &[(u32, f64, f64, f64, BubbleKind, bool)],
) -> (f64, f64) {
    let sense = if snap.smart {
        AI_SMART_SENSE_RADIUS
    } else {
        AI_SENSE_RADIUS
    };
    let mut fx = 0.0;
    let mut fy = 0.0;
    for &(oid, ox, oy, or, kind, _) in all {
        if oid == snap.id {
            continue;
        }
        match kind {
            BubbleKind::Unit => {}
            BubbleKind::Remains => {
                if snap.radius <= or * EAT_RATIO {
                    continue;
                }
            }
            _ => continue,
        }
        let dx = ox - snap.x;
        let dy = oy - snap.y;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist > sense || dist < 0.001 {
            continue;
        }
        let energy = match kind {
            BubbleKind::Unit => UNIT_ENERGY,
            BubbleKind::Remains => or * or * std::f64::consts::PI,
            _ => 0.0,
        };
        let proximity = 1.0 - dist / sense;
        let w = (energy / dist) * proximity * if snap.forage_timer > 0.0 { 1.35 } else { 1.0 };
        fx += dx / dist * w;
        fy += dy / dist * w;
    }
    (fx, fy)
}

fn try_forage_decision(
    snap: &AiSnapshot,
    all: &[(u32, f64, f64, f64, BubbleKind, bool)],
    player_pos: Option<(f64, f64, f64)>,
    map_w: f64,
    map_h: f64,
    viewport_w: f64,
    viewport_h: f64,
    world: &mut World,
    rng: &mut impl Rng,
) -> Option<AiDecision> {
    let forage = compute_forage_vector(snap, all);
    let forage_mag = (forage.0 * forage.0 + forage.1 * forage.1).sqrt();
    let (vx, vy, vp_w) = viewport_pull_direction(
        snap.id,
        snap.x,
        snap.y,
        player_pos,
        viewport_w,
        viewport_h,
        map_w,
        map_h,
    );

    let (mut dx, mut dy) = if forage_mag > 0.08 {
        (forage.0 / forage_mag, forage.1 / forage_mag)
    } else if vp_w > 0.0 {
        (vx, vy)
    } else {
        return None;
    };

    if forage_mag > 0.08 && vp_w > 0.0 {
        let food_w = if snap.forage_timer > 0.0 { 0.78 } else { 0.62 };
        let left = snap.x - snap.radius;
        let right = map_w - snap.x - snap.radius;
        let top = snap.y - snap.radius;
        let bottom = map_h - snap.y - snap.radius;
        let nearest = left.min(right).min(top).min(bottom);
        let effective_vp = if nearest < AI_WALL_NEAR {
            vp_w * AI_WALL_VIEWPORT_DAMP
        } else {
            vp_w
        };
        dx = dx * food_w + vx * effective_vp * (1.0 - food_w);
        dy = dy * food_w + vy * effective_vp * (1.0 - food_w);
    }

    let len = (dx * dx + dy * dy).sqrt().max(0.001);
    let mut dec = AiDecision {
        dir_x: dx / len,
        dir_y: dy / len,
        expel: snap.eaten_non_unit && ai_should_expel(rng, snap.radius, false, snap.eaten_non_unit),
        spinning: false,
        max_turn_rate: AI_MAX_TURN_RATE,
    };
    finalize_wall_direction(&mut dec, snap, world, map_w, map_h, rng);
    Some(dec)
}

fn compute_hunt_prey_vector(
    snap: &AiSnapshot,
    all: &[(u32, f64, f64, f64, BubbleKind, bool)],
) -> (f64, f64) {
    if !snap.eaten_non_unit {
        return (0.0, 0.0);
    }
    let mut best = (0.0, 0.0, f64::MAX);
    for &(oid, ox, oy, or, kind, _) in all {
        if oid == snap.id {
            continue;
        }
        if kind == BubbleKind::Unit || kind == BubbleKind::Remains {
            continue;
        }
        if snap.radius <= or * EAT_RATIO {
            continue;
        }
        let dx = ox - snap.x;
        let dy = oy - snap.y;
        let d = dx * dx + dy * dy;
        if d < best.2 {
            best = (dx, dy, d);
        }
    }
    if best.2 >= f64::MAX * 0.5 {
        return (0.0, 0.0);
    }
    let len = (best.0 * best.0 + best.1 * best.1).sqrt().max(0.001);
    (best.0 / len, best.1 / len)
}

fn compute_flee_vector(
    self_id: u32,
    x: f64,
    y: f64,
    radius: f64,
    all: &[(u32, f64, f64, f64, BubbleKind, bool)],
    sense_radius: f64,
    smart: bool,
) -> (f64, f64) {
    let mut fx = 0.0;
    let mut fy = 0.0;
    for &(oid, ox, oy, or, _, _) in all {
        if oid == self_id {
            continue;
        }
        if or <= radius * AI_FLEE_RATIO {
            continue;
        }
        let dx = x - ox;
        let dy = y - oy;
        let d = (dx * dx + dy * dy).sqrt();
        if d > sense_radius || d < 0.001 {
            continue;
        }
        let threat_pow = if smart { 1.7 } else { 1.5 };
        let w = ((sense_radius - d) / sense_radius).powi(2) * (or / radius).powf(threat_pow);
        let boost = if smart && or > radius * 1.2 { 1.4 } else { 1.0 };
        fx += dx / d * w * boost;
        fy += dy / d * w * boost;
    }
    (fx, fy)
}

fn compute_purposeful_intent(
    ai_id: u32,
    x: f64,
    y: f64,
    radius: f64,
    eaten_non_unit: bool,
    smart: bool,
    foraging: bool,
    all: &[(u32, f64, f64, f64, BubbleKind, bool)],
    map_w: f64,
    map_h: f64,
    viewport_w: f64,
    viewport_h: f64,
    player_pos: Option<(f64, f64, f64)>,
    attract_player: bool,
    rng: &mut impl Rng,
) -> (f64, f64) {
    let mistake_rate = if smart {
        AI_SMART_MISTAKE_RATE
    } else {
        AI_MISTAKE_RATE
    };
    if !smart && rng.gen_bool(mistake_rate) {
        let angle = rng.gen_range(0.0..std::f64::consts::TAU);
        return (angle.cos(), angle.sin());
    }

    let sectors = 8usize;
    let mut scores: Vec<(f64, f64, f64)> = Vec::with_capacity(sectors);

    for i in 0..sectors {
        let angle = i as f64 * std::f64::consts::TAU / sectors as f64;
        let dir_x = angle.cos();
        let dir_y = angle.sin();
        let score = score_direction(
            ai_id,
            x,
            y,
            radius,
            eaten_non_unit,
            smart,
            foraging,
            dir_x,
            dir_y,
            all,
            map_w,
            map_h,
            viewport_w,
            viewport_h,
            player_pos,
            attract_player,
        );
        scores.push((score, dir_x, dir_y));
    }

    scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let pick = if smart {
        weighted_pick_top(&scores[..scores.len().min(3)], rng)
    } else if rng.gen_bool(0.32) && scores.len() > 1 {
        let idx = rng.gen_range(1..scores.len().min(4));
        scores[idx]
    } else {
        scores[0]
    };

    let jitter = if smart { 0.12 } else { 0.18 };
    let ja: f64 = rng.gen_range(-jitter..jitter);
    let ca = ja.cos();
    let sa = ja.sin();
    let rx = pick.1 * ca - pick.2 * sa;
    let ry = pick.1 * sa + pick.2 * ca;
    let len = (rx * rx + ry * ry).sqrt().max(0.001);
    (rx / len, ry / len)
}

fn weighted_pick_top(candidates: &[(f64, f64, f64)], rng: &mut impl Rng) -> (f64, f64, f64) {
    if candidates.is_empty() {
        return (0.0, 1.0, 0.0);
    }
    let total: f64 = candidates.iter().map(|(s, _, _)| s.max(0.01)).sum();
    let mut roll = rng.gen::<f64>() * total;
    for c in candidates {
        roll -= c.0.max(0.01);
        if roll <= 0.0 {
            return *c;
        }
    }
    candidates[0]
}

fn score_direction(
    ai_id: u32,
    x: f64,
    y: f64,
    radius: f64,
    eaten_non_unit: bool,
    smart: bool,
    foraging: bool,
    dir_x: f64,
    dir_y: f64,
    all: &[(u32, f64, f64, f64, BubbleKind, bool)],
    map_w: f64,
    map_h: f64,
    viewport_w: f64,
    viewport_h: f64,
    player_pos: Option<(f64, f64, f64)>,
    attract_player: bool,
) -> f64 {
    let mut score = wall_penalty(x, y, radius, dir_x, dir_y, map_w, map_h);
    let visible = if smart {
        AI_SMART_SENSE_RADIUS
    } else {
        AI_VISIBLE_RADIUS
    };

    for &(oid, ox, oy, or, kind, _) in all {
        let dx = ox - x;
        let dy = oy - y;
        let dist_sq = dx * dx + dy * dy;
        if dist_sq < 1.0 {
            continue;
        }
        let dist = dist_sq.sqrt();
        if dist > visible {
            continue;
        }
        let alignment = (dx * dir_x + dy * dir_y) / dist;
        if alignment <= 0.05 {
            continue;
        }
        let proximity = 1.0 - dist / visible;

        match kind {
            BubbleKind::Unit | BubbleKind::Remains => {
                let food_w = if smart { 1.5 } else { 1.0 };
                let forage_boost = if foraging { 1.85 } else { 1.0 };
                let unit_ref = if kind == BubbleKind::Unit {
                    UNIT_ENERGY
                } else {
                    or * or * std::f64::consts::PI
                };
                score += alignment * proximity * food_w * forage_boost * (unit_ref / dist).min(80.0);
            }
            BubbleKind::Ai | BubbleKind::Player => {
                if or > radius * AI_FLEE_RATIO {
                    let threat_w = if smart { 3.0 } else { 2.1 };
                    score -= alignment * proximity * threat_w * (or / radius).powf(1.5);
                } else if eaten_non_unit && radius > or * EAT_RATIO {
                    score += alignment * proximity * 0.55 * (radius / or.max(1.0));
                }
            }
        }
        let _ = oid;
    }

    if attract_player {
        if let Some((px, py, pr)) = player_pos {
            if radius > pr * EAT_RATIO && eaten_non_unit {
                let dx = px - x;
                let dy = py - y;
                let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                if dist < visible {
                    let alignment = (dx * dir_x + dy * dir_y) / dist;
                    if alignment > 0.0 {
                        score += alignment * (1.0 - dist / visible) * 0.35;
                    }
                }
            }
        }
    }

    let (vx, vy, vp_w) = viewport_pull_direction(
        ai_id,
        x,
        y,
        player_pos,
        viewport_w,
        viewport_h,
        map_w,
        map_h,
    );
    if vp_w > 0.0 {
        let align = vx * dir_x + vy * dir_y;
        if align > 0.0 {
            score += align * vp_w * if foraging { 2.4 } else { 2.0 };
        }
    }

    let cx = map_w * 0.5 - x;
    let cy = map_h * 0.5 - y;
    let clen = (cx * cx + cy * cy).sqrt().max(1.0);
    let center_align = (cx * dir_x + cy * dir_y) / clen;
    if center_align > 0.0 {
        let left = x - radius;
        let right = map_w - x - radius;
        let top = y - radius;
        let bottom = map_h - y - radius;
        let nearest = left.min(right).min(top).min(bottom);
        let near_wall = (1.0 - nearest / AI_WALL_NEAR).clamp(0.0, 1.0);
        let center_w = if smart { 0.12 } else { 0.06 };
        score += center_align * (center_w + near_wall * AI_WALL_CENTER_BONUS);
    }

    score
}

fn wall_penalty(
    x: f64,
    y: f64,
    radius: f64,
    dir_x: f64,
    dir_y: f64,
    map_w: f64,
    map_h: f64,
) -> f64 {
    let mut score = 0.0;
    let left = x - radius;
    let right = map_w - x - radius;
    let top = y - radius;
    let bottom = map_h - y - radius;
    let band = AI_WALL_NEAR + AI_WALL_BAND_EXTRA;

    let edge_factor = |dist: f64| -> f64 { (1.0 - dist / band).clamp(0.0, 1.0) };

    if left < band {
        let s = edge_factor(left);
        if dir_x < 0.0 {
            score -= s * AI_WALL_PENALTY_MAX;
        } else {
            score += s * dir_x * AI_WALL_INWARD_BONUS;
        }
        if dir_x.abs() < 0.48 {
            score -= s * AI_WALL_SLIDE_PENALTY;
        }
    }
    if right < band {
        let s = edge_factor(right);
        if dir_x > 0.0 {
            score -= s * AI_WALL_PENALTY_MAX;
        } else {
            score += s * (-dir_x) * AI_WALL_INWARD_BONUS;
        }
        if dir_x.abs() < 0.48 {
            score -= s * AI_WALL_SLIDE_PENALTY;
        }
    }
    if top < band {
        let s = edge_factor(top);
        if dir_y < 0.0 {
            score -= s * AI_WALL_PENALTY_MAX;
        } else {
            score += s * dir_y * AI_WALL_INWARD_BONUS;
        }
        if dir_y.abs() < 0.48 {
            score -= s * AI_WALL_SLIDE_PENALTY;
        }
    }
    if bottom < band {
        let s = edge_factor(bottom);
        if dir_y > 0.0 {
            score -= s * AI_WALL_PENALTY_MAX;
        } else {
            score += s * (-dir_y) * AI_WALL_INWARD_BONUS;
        }
        if dir_y.abs() < 0.48 {
            score -= s * AI_WALL_SLIDE_PENALTY;
        }
    }
    score
}

fn compute_suicide_decision(
    x: f64,
    y: f64,
    player_pos: Option<(f64, f64, f64)>,
    rng: &mut impl Rng,
    radius: f64,
) -> AiDecision {
    let Some((px, py, _)) = player_pos else {
        return AiDecision {
            dir_x: 1.0,
            dir_y: 0.0,
            expel: false,
            spinning: false,
            max_turn_rate: AI_MAX_TURN_RATE * 1.2,
        };
    };
    let dx = px - x;
    let dy = py - y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.001 {
        return AiDecision {
            dir_x: 1.0,
            dir_y: 0.0,
            expel: false,
            spinning: false,
            max_turn_rate: AI_MAX_TURN_RATE * 1.2,
        };
    }
    AiDecision {
        dir_x: dx / len,
        dir_y: dy / len,
        expel: ai_should_expel(rng, radius, true, true),
        spinning: false,
        max_turn_rate: AI_MAX_TURN_RATE * 1.35,
    }
}
