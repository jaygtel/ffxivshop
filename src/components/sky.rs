//! The backdrop: the coming-soon page's night sky, drawn on a canvas from
//! Rust. Stars twinkle, and motes of aether (mostly blue, a few gold) drift
//! up through them. The glows, aurora and vignette underneath are CSS (see
//! main.css). With reduced motion turned on it draws one still frame.

use leptos::prelude::*;
use std::cell::RefCell;
use std::f64::consts::TAU;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

/// The animation callback, which re-schedules itself every frame.
type FrameCallback = Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>;

/// The stars' colour.
const STARLIGHT: &str = "#e2ebff";
/// The motes' colours, as "r, g, b": aether blue, periwinkle and gold.
const AETHER: &str = "127, 216, 255";
const PERIWINKLE: &str = "170, 200, 255";
const GOLD: &str = "248, 222, 150";

#[component]
pub fn AetherSky() -> impl IntoView {
    let canvas = NodeRef::<leptos::html::Canvas>::new();
    canvas.on_load(start);
    view! { <canvas class="sky" node_ref=canvas aria-hidden="true"></canvas> }
}

fn random(low: f64, high: f64) -> f64 {
    low + js_sys::Math::random() * (high - low)
}

struct Star {
    /// Position as a fraction of the window, so resizing (a phone's address
    /// bar coming and going) keeps every star where it was.
    x: f64,
    y: f64,
    radius: f64,
    alpha: f64,
    /// How fast it twinkles, and where in the twinkle it starts.
    speed: f64,
    phase: f64,
}

impl Star {
    fn new() -> Star {
        Star {
            x: random(0.0, 1.0),
            y: random(0.0, 1.0),
            radius: random(0.25, 1.25),
            alpha: random(0.25, 0.95),
            speed: random(0.3, 1.4),
            phase: random(0.0, TAU),
        }
    }
}

struct Mote {
    /// Where it rises from, as a fraction of the window's width; it sways
    /// either side of this.
    x: f64,
    /// Height as a fraction of the window: 0 is the top.
    y: f64,
    radius: f64,
    /// Pixels per second.
    rise: f64,
    sway: f64,
    phase: f64,
    /// Sways per second.
    freq: f64,
    colour: &'static str,
}

impl Mote {
    /// A new mote, either anywhere on screen (when the sky is first drawn) or
    /// just below the bottom edge, ready to rise into view.
    fn new(anywhere: bool, height: f64) -> Mote {
        let colour = if js_sys::Math::random() < 0.18 {
            GOLD
        } else if js_sys::Math::random() < 0.5 {
            AETHER
        } else {
            PERIWINKLE
        };
        Mote {
            x: random(0.0, 1.0),
            y: if anywhere {
                random(0.0, 1.0)
            } else {
                1.0 + random(10.0, 80.0) / height.max(1.0)
            },
            radius: random(0.9, 2.6),
            rise: random(9.0, 24.0),
            sway: random(10.0, 30.0),
            phase: random(0.0, TAU),
            freq: random(0.25, 0.6),
            colour,
        }
    }
}

struct Sky {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    stars: Vec<Star>,
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

        // As many as the coming-soon page has for a window this size. Those
        // already in the sky stay put; only the difference comes or goes.
        let area = self.width * self.height;
        let stars = (area / 4800.0).round().min(320.0) as usize;
        let motes = (area / 19000.0).round().min(80.0) as usize;
        self.stars.truncate(stars);
        while self.stars.len() < stars {
            self.stars.push(Star::new());
        }
        self.motes.truncate(motes);
        while self.motes.len() < motes {
            self.motes.push(Mote::new(true, self.height));
        }
    }

    /// Draws one frame. `still` draws it without movement or twinkling.
    fn frame(&mut self, now_ms: f64, still: bool) {
        let dt = self
            .last
            .map_or(0.0, |last| ((now_ms - last) / 1000.0).clamp(0.0, 0.05));
        self.last = Some(now_ms);
        let t = now_ms / 1000.0;
        let (width, height) = (self.width, self.height);

        let ctx = &self.ctx;
        ctx.clear_rect(0.0, 0.0, width, height);

        ctx.set_fill_style_str(STARLIGHT);
        for star in &self.stars {
            let twinkle = if still {
                1.0
            } else {
                0.55 + 0.45 * (t * star.speed + star.phase).sin()
            };
            ctx.set_global_alpha(star.alpha * twinkle);
            ctx.begin_path();
            let _ = ctx.arc(star.x * width, star.y * height, star.radius, 0.0, TAU);
            ctx.fill();
        }
        ctx.set_global_alpha(1.0);

        for mote in &mut self.motes {
            if !still {
                mote.y -= mote.rise * dt / height.max(1.0);
                if mote.y * height < -30.0 {
                    *mote = Mote::new(false, height);
                }
            }
            let x = mote.x * width + (mote.phase + t * mote.freq).sin() * mote.sway;
            let y = mote.y * height;
            // Brightest low down, fading out as it nears the top.
            let alpha = (mote.y * 1.4).clamp(0.0, 1.0) * 0.9;
            if alpha <= 0.0 {
                continue;
            }
            let glow_radius = mote.radius * 6.0;
            let colour = mote.colour;
            if let Ok(glow) = ctx.create_radial_gradient(x, y, 0.0, x, y, glow_radius) {
                let _ = glow.add_color_stop(0.0, &format!("rgba({colour}, {alpha:.3})"));
                let _ = glow.add_color_stop(0.25, &format!("rgba({colour}, {:.3})", alpha * 0.45));
                let _ = glow.add_color_stop(1.0, &format!("rgba({colour}, 0)"));
                ctx.set_fill_style_canvas_gradient(&glow);
                ctx.begin_path();
                let _ = ctx.arc(x, y, glow_radius, 0.0, TAU);
                ctx.fill();
            }
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
    let still = crate::browser::prefers_reduced_motion();

    let sky = Rc::new(RefCell::new(Sky {
        canvas,
        ctx,
        stars: Vec::new(),
        motes: Vec::new(),
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
            // Resizing clears the canvas, so redraw the last frame at once
            // rather than leave a blank one (the animation, if it's running,
            // carries on from there).
            let now = sky.last.unwrap_or(0.0);
            sky.frame(now, still);
        })
    };
    let _ = window.add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref());
    on_resize.forget();

    if still {
        sky.borrow_mut().frame(0.0, true);
        return;
    }

    let tick: FrameCallback = Rc::new(RefCell::new(None));
    let next = tick.clone();
    *tick.borrow_mut() = Some(Closure::new(move |now: f64| {
        sky.borrow_mut().frame(now, false);
        if let (Some(window), Some(callback)) = (web_sys::window(), next.borrow().as_ref()) {
            let _ = window.request_animation_frame(callback.as_ref().unchecked_ref());
        }
    }));
    if let Some(callback) = tick.borrow().as_ref() {
        let _ = window.request_animation_frame(callback.as_ref().unchecked_ref());
    }
}
