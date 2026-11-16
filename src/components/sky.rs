//! The backdrop: motes of aether drifting up through a dusk garden, drawn on
//! a canvas from Rust. With reduced motion turned on it draws one still frame.

use leptos::prelude::*;
use std::cell::RefCell;
use std::f64::consts::TAU;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

/// The animation callback, which re-schedules itself every frame.
type FrameCallback = Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>;

#[component]
pub fn AetherSky() -> impl IntoView {
    let canvas = NodeRef::<leptos::html::Canvas>::new();
    canvas.on_load(start);
    view! { <canvas class="sky" node_ref=canvas aria-hidden="true"></canvas> }
}

struct Mote {
    /// Position as a fraction of the screen.
    x: f64,
    y: f64,
    radius: f64,
    /// Screen heights per second.
    rise: f64,
    sway: f64,
    phase: f64,
    warm: bool,
}

impl Mote {
    fn new(anywhere: bool) -> Mote {
        let r = js_sys::Math::random;
        Mote {
            x: r(),
            y: if anywhere { r() } else { 1.02 + r() * 0.1 },
            radius: 0.8 + r() * 1.8,
            rise: 0.006 + r() * 0.018,
            sway: 6.0 + r() * 18.0,
            phase: r() * TAU,
            warm: r() < 0.3,
        }
    }
}

struct Sky {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    motes: Vec<Mote>,
    width: f64,
    height: f64,
    last: Option<f64>,
}

impl Sky {
    fn resize(&mut self) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let dpr = window.device_pixel_ratio().clamp(1.0, 2.0);
        self.width = window
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(1280.0);
        self.height = window
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(800.0);
        self.canvas.set_width((self.width * dpr) as u32);
        self.canvas.set_height((self.height * dpr) as u32);
        let _ = self.ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
    }

    fn frame(&mut self, now_ms: f64) {
        let dt = self
            .last
            .map_or(0.0, |last| ((now_ms - last) / 1000.0).clamp(0.0, 0.05));
        self.last = Some(now_ms);
        let t = now_ms / 1000.0;

        let ctx = &self.ctx;
        ctx.clear_rect(0.0, 0.0, self.width, self.height);
        for mote in &mut self.motes {
            mote.y -= mote.rise * dt;
            if mote.y < -0.05 {
                *mote = Mote::new(false);
            }
            let x = mote.x * self.width + (t * 0.35 + mote.phase).sin() * mote.sway;
            let y = mote.y * self.height;
            let twinkle = 0.55 + 0.45 * (t * 1.3 + mote.phase * 2.0).sin();
            // Fade in near the bottom and out near the top.
            let fade = (mote.y.min(1.0 - mote.y) * 6.0).clamp(0.0, 1.0);
            let alpha = twinkle * fade;
            let colour = if mote.warm {
                "236, 200, 128"
            } else {
                "140, 220, 232"
            };

            let glow_radius = mote.radius * 6.0;
            if let Ok(glow) = ctx.create_radial_gradient(x, y, 0.0, x, y, glow_radius) {
                let _ = glow.add_color_stop(0.0, &format!("rgba({colour}, {:.3})", alpha * 0.32));
                let _ = glow.add_color_stop(1.0, &format!("rgba({colour}, 0)"));
                ctx.set_fill_style_canvas_gradient(&glow);
                ctx.begin_path();
                let _ = ctx.arc(x, y, glow_radius, 0.0, TAU);
                ctx.fill();
            }

            ctx.set_fill_style_str(&format!("rgba({colour}, {:.3})", alpha * 0.9));
            ctx.begin_path();
            let _ = ctx.arc(x, y, mote.radius, 0.0, TAU);
            ctx.fill();
        }
    }
}

fn start(canvas: HtmlCanvasElement) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(ctx) = canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())
    else {
        return;
    };
    let width = window
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(1280.0);
    let count = if width < 700.0 { 24 } else { 46 };
    let still = crate::browser::prefers_reduced_motion();

    let sky = Rc::new(RefCell::new(Sky {
        canvas,
        ctx,
        motes: (0..count).map(|_| Mote::new(true)).collect(),
        width: 0.0,
        height: 0.0,
        last: None,
    }));
    sky.borrow_mut().resize();

    // The sky lives as long as the page, so these closures are never freed.
    let on_resize = {
        let sky = sky.clone();
        Closure::<dyn FnMut()>::new(move || {
            let mut sky = sky.borrow_mut();
            sky.resize();
            if still {
                sky.frame(0.0);
            }
        })
    };
    let _ = window.add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref());
    on_resize.forget();

    if still {
        sky.borrow_mut().frame(0.0);
        return;
    }

    let tick: FrameCallback = Rc::new(RefCell::new(None));
    let next = tick.clone();
    *tick.borrow_mut() = Some(Closure::new(move |now: f64| {
        sky.borrow_mut().frame(now);
        if let (Some(window), Some(callback)) = (web_sys::window(), next.borrow().as_ref()) {
            let _ = window.request_animation_frame(callback.as_ref().unchecked_ref());
        }
    }));
    if let Some(callback) = tick.borrow().as_ref() {
        let _ = window.request_animation_frame(callback.as_ref().unchecked_ref());
    }
}
