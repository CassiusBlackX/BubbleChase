use crate::bubble::{Bubble, BubbleKind};
use crate::config::{BG_COLOR, REMAINS_ALPHA, STATIC_UNIT_ALPHA, WIN_DIAMETER_RATIO};
use crate::game::GameStatus;
use crate::physics::camera_offset;
use crate::world::World;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement};

pub struct Renderer {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    dpr: f64,
}

impl Renderer {
    pub fn new(canvas_id: &str, width: f64, height: f64) -> Result<Self, JsValue> {
        let window = web_sys::window().ok_or("no window")?;
        let document = window.document().ok_or("no document")?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or("canvas not found")?
            .dyn_into::<HtmlCanvasElement>()?;
        let ctx = canvas
            .get_context("2d")?
            .ok_or("no 2d context")?
            .dyn_into::<CanvasRenderingContext2d>()?;
        let dpr = window.device_pixel_ratio();
        let mut r = Self { canvas, ctx, dpr };
        r.resize(width, height);
        Ok(r)
    }

    pub fn resize(&mut self, width: f64, height: f64) {
        self.dpr = web_sys::window().map(|w| w.device_pixel_ratio()).unwrap_or(1.0);
        self.canvas.set_width((width * self.dpr) as u32);
        self.canvas.set_height((height * self.dpr) as u32);
        let canvas: &HtmlElement = self.canvas.as_ref();
        canvas.style().set_property("width", &format!("{width}px")).ok();
        canvas.style().set_property("height", &format!("{height}px")).ok();
        let _ = self.ctx.set_transform(self.dpr, 0.0, 0.0, self.dpr, 0.0, 0.0);
    }

    pub fn draw(
        &self,
        world: &World,
        viewport_w: f64,
        viewport_h: f64,
        status: GameStatus,
        progress: f64,
    ) {
        let (bg_r, bg_g, bg_b) = BG_COLOR;
        self.ctx.set_fill_style_str(&format!("rgb({bg_r},{bg_g},{bg_b})"));
        self.ctx.fill_rect(0.0, 0.0, viewport_w, viewport_h);

        let player = match world.player() {
            Some(p) => p,
            None => return,
        };

        let (cam_x, cam_y) = camera_offset(
            player.x,
            player.y,
            viewport_w,
            viewport_h,
            world.map_width,
            world.map_height,
        );

        self.draw_map_border(cam_x, cam_y, viewport_w, viewport_h, world);

        let mut draw_list: Vec<&Bubble> = world.bubbles.iter().collect();
        draw_list.sort_by(|a, b| {
            a.radius()
                .partial_cmp(&b.radius())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for bubble in draw_list {
            self.draw_bubble(bubble, cam_x, cam_y);
        }

        self.draw_hud(viewport_w, viewport_h, player, progress);

        if status != GameStatus::Playing {
            self.draw_overlay(viewport_w, viewport_h, status);
        }
    }

    fn draw_map_border(
        &self,
        cam_x: f64,
        cam_y: f64,
        viewport_w: f64,
        viewport_h: f64,
        world: &World,
    ) {
        let sx = -cam_x;
        let sy = -cam_y;
        self.ctx.set_stroke_style_str("rgba(255,255,255,0.08)");
        self.ctx.set_line_width(2.0);
        self.ctx.stroke_rect(sx, sy, world.map_width, world.map_height);

        let grid = 80.0;
        self.ctx.set_stroke_style_str("rgba(255,255,255,0.03)");
        self.ctx.set_line_width(1.0);
        let start_x = (cam_x / grid).floor() * grid;
        let start_y = (cam_y / grid).floor() * grid;
        let mut x = start_x;
        while x <= cam_x + viewport_w {
            self.ctx.begin_path();
            self.ctx.move_to(x - cam_x, 0.0);
            self.ctx.line_to(x - cam_x, viewport_h);
            self.ctx.stroke();
            x += grid;
        }
        let mut y = start_y;
        while y <= cam_y + viewport_h {
            self.ctx.begin_path();
            self.ctx.move_to(0.0, y - cam_y);
            self.ctx.line_to(viewport_w, y - cam_y);
            self.ctx.stroke();
            y += grid;
        }
    }

    fn draw_bubble(&self, bubble: &Bubble, cam_x: f64, cam_y: f64) {
        let sx = bubble.x - cam_x;
        let sy = bubble.y - cam_y;
        let r = bubble.radius();
        let (cr, cg, cb) = bubble.color;

        match bubble.kind {
            BubbleKind::Player | BubbleKind::Ai => {
                self.ctx
                    .set_fill_style_str(&format!("rgb({cr},{cg},{cb})"));
            }
            BubbleKind::Unit => {
                self.ctx.set_fill_style_str(&format!(
                    "rgba({cr},{cg},{cb},{STATIC_UNIT_ALPHA})"
                ));
            }
            BubbleKind::Remains => {
                self.ctx
                    .set_fill_style_str(&format!("rgba({cr},{cg},{cb},{REMAINS_ALPHA})"));
            }
        }

        self.ctx.begin_path();
        self.ctx.arc(sx, sy, r, 0.0, std::f64::consts::TAU).ok();
        self.ctx.fill();

        if bubble.kind == BubbleKind::Player || bubble.kind == BubbleKind::Ai {
            self.ctx.set_fill_style_str("rgba(255,255,255,0.2)");
            self.ctx.begin_path();
            self.ctx
                .arc(sx - r * 0.25, sy - r * 0.25, r * 0.35, 0.0, std::f64::consts::TAU)
                .ok();
            self.ctx.fill();
        } else if bubble.kind == BubbleKind::Remains {
            self.ctx.set_stroke_style_str(&format!(
                "rgba({cr},{cg},{cb},0.35)"
            ));
            self.ctx.set_line_width(1.0);
            self.ctx.begin_path();
            self.ctx.arc(sx, sy, r, 0.0, std::f64::consts::TAU).ok();
            self.ctx.stroke();
        }

        if bubble.kind == BubbleKind::Player {
            self.ctx.set_stroke_style_str("rgba(255,255,255,0.6)");
            self.ctx.set_line_width(2.0);
            self.ctx.begin_path();
            self.ctx.arc(sx, sy, r, 0.0, std::f64::consts::TAU).ok();
            self.ctx.stroke();
        }

        if bubble.expelling {
            self.ctx.set_stroke_style_str("rgba(255,255,255,0.25)");
            self.ctx.set_line_width(1.5);
            self.ctx.begin_path();
            self.ctx.arc(sx, sy, r + 3.0, 0.0, std::f64::consts::TAU).ok();
            self.ctx.stroke();
        }
    }

    fn draw_hud(&self, vw: f64, vh: f64, player: &Bubble, progress: f64) {
        let target = vw.min(vh) * WIN_DIAMETER_RATIO;
        self.ctx.set_fill_style_str("rgba(0,0,0,0.45)");
        self.ctx.fill_rect(10.0, 10.0, 220.0, 72.0);

        self.ctx.set_fill_style_str("#ffffff");
        let _ = self.ctx.set_font("14px sans-serif");
        let _ = self
            .ctx
            .fill_text(&format!("直径: {:.0}", player.diameter()), 20.0, 32.0);
        let _ = self.ctx.fill_text(
            &format!("目标: {:.0} ({:.0}%)", target, progress * 100.0),
            20.0,
            52.0,
        );
        let _ = self
            .ctx
            .fill_text(&format!("能量: {:.0}", player.energy), 20.0, 72.0);
    }

    fn draw_overlay(&self, vw: f64, vh: f64, status: GameStatus) {
        self.ctx.set_fill_style_str("rgba(0,0,0,0.55)");
        self.ctx.fill_rect(0.0, 0.0, vw, vh);

        self.ctx.set_fill_style_str("#ffffff");
        let _ = self.ctx.set_font("bold 36px sans-serif");
        let text = match status {
            GameStatus::Won => "胜利！",
            GameStatus::Lost => "被吃掉了！",
            GameStatus::Playing => "",
        };
        let tw = self.ctx.measure_text(text).unwrap().width();
        let _ = self.ctx.fill_text(text, (vw - tw) / 2.0, vh / 2.0 - 10.0);

        let _ = self.ctx.set_font("18px sans-serif");
        let hint = "点击或按 R 重新开始";
        let hw = self.ctx.measure_text(hint).unwrap().width();
        let _ = self.ctx.fill_text(hint, (vw - hw) / 2.0, vh / 2.0 + 30.0);
    }
}
