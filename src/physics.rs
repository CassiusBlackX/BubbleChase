use crate::bubble::Bubble;
use crate::config::{
    AI_SPIN_TURN_RATE, AI_STEER_RATE, BASE_SPEED, DRIVE_ACCEL_ALPHA, DRIVE_ACCEL_BASE,
    EXPEL_DRIVE_ACCEL_MULT, EXPEL_MAX_SPEED_MULT, INITIAL_RADIUS, MIN_SPEED_RATIO, SPEED_ALPHA,
};

pub fn initial_energy() -> f64 {
    std::f64::consts::PI * INITIAL_RADIUS * INITIAL_RADIUS
}

pub fn bubble_mass(bubble: &Bubble) -> f64 {
    bubble.energy.max(initial_energy() * 0.25)
}

/// 小泡泡巡航快，大泡泡巡航慢
pub fn cruise_speed(bubble: &Bubble) -> f64 {
    let r = bubble.radius().max(1.0);
    let ratio = (INITIAL_RADIUS / r).powf(SPEED_ALPHA).max(MIN_SPEED_RATIO);
    let mut speed = BASE_SPEED * ratio;
    if bubble.suicide {
        speed *= 1.12;
    }
    speed
}

/// 质量越小，加速越快
pub fn drive_accel(bubble: &Bubble) -> f64 {
    let m0 = initial_energy();
    let m = bubble_mass(bubble);
    DRIVE_ACCEL_BASE * (m0 / m).powf(DRIVE_ACCEL_ALPHA)
}

pub fn max_speed(bubble: &Bubble, expelling: bool) -> f64 {
    let cruise = cruise_speed(bubble);
    if expelling {
        cruise * EXPEL_MAX_SPEED_MULT
    } else {
        cruise
    }
}

fn movement_targets(bubble: &Bubble, dir_x: f64, dir_y: f64, expelling: bool) -> (f64, f64, f64) {
    let len = (dir_x * dir_x + dir_y * dir_y).sqrt();
    if len < 0.001 {
        return (0.0, 0.0, drive_accel(bubble));
    }
    let cruise = cruise_speed(bubble);
    let speed = if expelling {
        cruise * EXPEL_MAX_SPEED_MULT
    } else {
        cruise
    };
    let accel = if expelling {
        drive_accel(bubble) * EXPEL_DRIVE_ACCEL_MULT
    } else {
        drive_accel(bubble)
    };
    (dir_x / len * speed, dir_y / len * speed, accel)
}

pub fn apply_expel_impulse(bubble: &mut Bubble, dv_x: f64, dv_y: f64) {
    bubble.vx += dv_x;
    bubble.vy += dv_y;
}

fn clamp_speed(bubble: &mut Bubble, limit: f64) {
    let speed = (bubble.vx * bubble.vx + bubble.vy * bubble.vy).sqrt();
    if speed > limit && speed > 0.001 {
        let scale = limit / speed;
        bubble.vx *= scale;
        bubble.vy *= scale;
    }
}

pub fn clamp_bubble_speed(bubble: &mut Bubble, expelling: bool) {
    clamp_speed(bubble, max_speed(bubble, expelling));
}

/// 玩家：吃过任意食物（能量高于初始）即可排空；AI：需吃过非单位泡泡
pub fn can_use_expel(bubble: &Bubble) -> bool {
    use crate::bubble::BubbleKind;
    match bubble.kind {
        BubbleKind::Player => bubble.energy > initial_energy() * 1.01,
        BubbleKind::Ai => bubble.eaten_non_unit,
        _ => false,
    }
}

fn accelerate_toward(
    bubble: &mut Bubble,
    target_vx: f64,
    target_vy: f64,
    accel: f64,
    dt: f64,
) {
    let dvx = target_vx - bubble.vx;
    let dvy = target_vy - bubble.vy;
    let dv_len = (dvx * dvx + dvy * dvy).sqrt();
    if dv_len < 0.001 {
        return;
    }
    let max_dv = accel * dt;
    let scale = (max_dv / dv_len).min(1.0);
    bubble.vx += dvx * scale;
    bubble.vy += dvy * scale;
}

fn integrate(bubble: &mut Bubble, dt: f64, map_w: f64, map_h: f64) {
    bubble.x += bubble.vx * dt;
    bubble.y += bubble.vy * dt;
    crate::world::World::clamp_position(bubble, map_w, map_h);
}

pub fn apply_player_movement(
    bubble: &mut Bubble,
    dir_x: f64,
    dir_y: f64,
    dt: f64,
    expelling: bool,
    map_w: f64,
    map_h: f64,
) {
    let len = (dir_x * dir_x + dir_y * dir_y).sqrt();
    if expelling {
        if len > 0.001 {
            let (target_vx, target_vy, accel) = movement_targets(bubble, dir_x, dir_y, true);
            accelerate_toward(bubble, target_vx, target_vy, accel, dt);
        }
        clamp_bubble_speed(bubble, true);
    } else if len > 0.001 {
        let speed = cruise_speed(bubble);
        bubble.vx = dir_x / len * speed;
        bubble.vy = dir_y / len * speed;
    } else {
        bubble.vx = 0.0;
        bubble.vy = 0.0;
    }
    integrate(bubble, dt, map_w, map_h);
}

fn normalize_angle(mut a: f64) -> f64 {
    while a > std::f64::consts::PI {
        a -= std::f64::consts::TAU;
    }
    while a < -std::f64::consts::PI {
        a += std::f64::consts::TAU;
    }
    a
}

fn rotate_toward(
    current_x: f64,
    current_y: f64,
    target_x: f64,
    target_y: f64,
    max_turn: f64,
) -> (f64, f64) {
    let clen = (current_x * current_x + current_y * current_y).sqrt();
    let (cx, cy) = if clen > 0.001 {
        (current_x / clen, current_y / clen)
    } else {
        (1.0, 0.0)
    };

    let tlen = (target_x * target_x + target_y * target_y).sqrt();
    if tlen < 0.001 {
        return (cx, cy);
    }
    let tx = target_x / tlen;
    let ty = target_y / tlen;

    let cur_a = cy.atan2(cx);
    let tgt_a = ty.atan2(tx);
    let delta = normalize_angle(tgt_a - cur_a);
    let turn = delta.clamp(-max_turn, max_turn);
    let na = cur_a + turn;
    (na.cos(), na.sin())
}

/// AI 专用：限速转向 + 质量感加速
pub fn apply_movement_smoothed(
    bubble: &mut Bubble,
    target_dir_x: f64,
    target_dir_y: f64,
    dt: f64,
    expelling: bool,
    map_w: f64,
    map_h: f64,
    max_turn_rate: f64,
) {
    let tlen = (target_dir_x * target_dir_x + target_dir_y * target_dir_y).sqrt();
    let (tdx, tdy) = if tlen > 0.001 {
        (target_dir_x / tlen, target_dir_y / tlen)
    } else {
        let slen = (bubble.steer_x * bubble.steer_x + bubble.steer_y * bubble.steer_y).sqrt();
        if slen > 0.001 {
            (bubble.steer_x / slen, bubble.steer_y / slen)
        } else {
            let damp = (1.0 - 4.0 * dt).max(0.0);
            bubble.vx *= damp;
            bubble.vy *= damp;
            clamp_speed(bubble, max_speed(bubble, expelling));
            integrate(bubble, dt, map_w, map_h);
            return;
        }
    };

    let max_turn = max_turn_rate * dt;
    let (sx, sy) = rotate_toward(bubble.steer_x, bubble.steer_y, tdx, tdy, max_turn);

    let alpha = 1.0 - (-AI_STEER_RATE * dt).exp();
    bubble.steer_x += (sx - bubble.steer_x) * alpha;
    bubble.steer_y += (sy - bubble.steer_y) * alpha;

    let slen = (bubble.steer_x * bubble.steer_x + bubble.steer_y * bubble.steer_y).sqrt();
    if slen < 0.001 {
        return;
    }

    let speed = cruise_speed(bubble);
    let target_speed = if expelling {
        speed * EXPEL_MAX_SPEED_MULT
    } else {
        speed
    };
    let accel = if expelling {
        drive_accel(bubble) * EXPEL_DRIVE_ACCEL_MULT
    } else {
        drive_accel(bubble)
    };
    let target_vx = bubble.steer_x / slen * target_speed;
    let target_vy = bubble.steer_y / slen * target_speed;
    accelerate_toward(bubble, target_vx, target_vy, accel, dt);

    clamp_speed(bubble, max_speed(bubble, expelling));
    integrate(bubble, dt, map_w, map_h);
}

pub fn apply_spin_in_place(bubble: &mut Bubble, dt: f64, map_w: f64, map_h: f64) {
    let slen = (bubble.steer_x * bubble.steer_x + bubble.steer_y * bubble.steer_y).sqrt();
    let (cx, cy) = if slen > 0.001 {
        (bubble.steer_x / slen, bubble.steer_y / slen)
    } else {
        (1.0, 0.0)
    };
    let cur_a = cy.atan2(cx);
    let na = cur_a + AI_SPIN_TURN_RATE * dt;
    bubble.steer_x = na.cos();
    bubble.steer_y = na.sin();
    bubble.ai_intent_x = bubble.steer_x;
    bubble.ai_intent_y = bubble.steer_y;

    let speed = cruise_speed(bubble) * 0.25;
    let target_vx = bubble.steer_x * speed;
    let target_vy = bubble.steer_y * speed;
    accelerate_toward(bubble, target_vx, target_vy, drive_accel(bubble) * 0.6, dt);

    clamp_speed(bubble, speed);
    integrate(bubble, dt, map_w, map_h);
}

pub fn camera_offset(
    player_x: f64,
    player_y: f64,
    viewport_w: f64,
    viewport_h: f64,
    map_w: f64,
    map_h: f64,
) -> (f64, f64) {
    let mut cam_x = player_x - viewport_w / 2.0;
    let mut cam_y = player_y - viewport_h / 2.0;
    cam_x = cam_x.clamp(0.0, (map_w - viewport_w).max(0.0));
    cam_y = cam_y.clamp(0.0, (map_h - viewport_h).max(0.0));
    (cam_x, cam_y)
}
