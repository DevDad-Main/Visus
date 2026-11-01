use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};

use softbuffer::{Context, Surface};
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Transform};
use visus_core::{Alert, Notch, Page, Shape};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

const WIN_W: f32 = 800.0;
const WIN_H: f32 = 280.0;
const FRAME: Duration = Duration::from_millis(16);

struct App {
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    notch: Notch,
    last_tick: Instant,
}

impl App {
    fn render(&mut self) {
        let (Some(window), Some(surface)) = (&self.window, &mut self.surface) else { return };
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        surface.resize(w, h).unwrap();
        let Some(mut pixmap) = Pixmap::new(size.width, size.height) else { return };

        let scale = window.scale_factor() as f32;
        draw(&mut pixmap, &self.notch, scale);

        let mut buf = surface.buffer_mut().unwrap();
        // tiny-skia is premultiplied RGBA; softbuffer wants premultiplied 0xAARRGGBB.
        for (dst, src) in buf.iter_mut().zip(pixmap.pixels()) {
            *dst = (src.alpha() as u32) << 24
                | (src.red() as u32) << 16
                | (src.green() as u32) << 8
                | src.blue() as u32;
        }
        window.pre_present_notify();
        buf.present().unwrap();
    }

    fn set_cursor(&mut self, x: f32, y: f32) {
        let scale = self.window.as_ref().map_or(1.0, |w| w.scale_factor() as f32);
        let inside = self.notch.shape().rect.contains(x / scale, y / scale);
        self.notch.hover(inside);
    }
}

fn pill_path(s: &Shape) -> Option<tiny_skia::Path> {
    const K: f32 = 0.5523;
    let r = s.bottom_radius.min(s.rect.w / 2.0).min(s.rect.h);
    let (x, y, w, h) = (s.rect.x, s.rect.y, s.rect.w, s.rect.h);
    let mut p = PathBuilder::new();
    p.move_to(x, y);
    p.line_to(x + w, y);
    p.line_to(x + w, y + h - r);
    p.cubic_to(x + w, y + h - r + r * K, x + w - r + r * K, y + h, x + w - r, y + h);
    p.line_to(x + r, y + h);
    p.cubic_to(x + r - r * K, y + h, x, y + h - r + r * K, x, y + h - r);
    p.close();
    p.finish()
}

fn draw(pixmap: &mut Pixmap, notch: &Notch, scale: f32) {
    let t = Transform::from_scale(scale, scale);
    let shape = notch.shape();

    let mut black = Paint::default();
    black.set_color(Color::BLACK);
    black.anti_alias = true;
    if let Some(path) = pill_path(&shape) {
        pixmap.fill_path(&path, &black, FillRule::Winding, t, None);
    }

    let cx = shape.rect.x + shape.rect.w / 2.0;
    let mut paint = Paint::default();
    paint.anti_alias = true;

    if let Some(alert) = notch.alert_kind() {
        paint.set_color(match alert {
            Alert::Charging => Color::from_rgba8(48, 209, 88, 255),
            Alert::Headphones => Color::from_rgba8(90, 200, 250, 255),
        });
        let r = 20.0;
        let (ax, ay) = (shape.rect.x + 52.0, shape.rect.h / 2.0);
        if let Some(c) = PathBuilder::from_circle(ax, ay, r) {
            pixmap.fill_path(&c, &paint, FillRule::Winding, t, None);
        }
    }

    if notch.is_open() {
        let n = Page::ALL.len() as f32;
        let y = shape.rect.h - 12.0;
        for (i, p) in Page::ALL.iter().enumerate() {
            let active = *p == notch.page;
            paint.set_color(Color::from_rgba8(255, 255, 255, if active { 255 } else { 110 }));
            let dx = (i as f32 - (n - 1.0) / 2.0) * 14.0;
            let rad = if active { 4.0 } else { 3.0 };
            if let Some(c) = PathBuilder::from_circle(cx + dx, y, rad) {
                pixmap.fill_path(&c, &paint, FillRule::Winding, t, None);
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs: WindowAttributes = Window::default_attributes()
            .with_title("Visus")
            .with_inner_size(LogicalSize::new(WIN_W, WIN_H))
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_window_level(WindowLevel::AlwaysOnTop);
        let window = Rc::new(el.create_window(attrs).expect("window"));

        if let Some(m) = el.primary_monitor().or_else(|| window.current_monitor()) {
            let win_w = window.outer_size().width as i32;
            let x = m.position().x + (m.size().width as i32 - win_w) / 2;
            window.set_outer_position(PhysicalPosition::new(x, m.position().y));
        }

        let ctx = Context::new(window.clone()).expect("softbuffer context");
        self.surface = Some(Surface::new(&ctx, window.clone()).expect("surface"));
        self.window = Some(window);
        self.last_tick = Instant::now();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, ev: WindowEvent) {
        match ev {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::RedrawRequested => self.render(),
            WindowEvent::CursorMoved { position, .. } => {
                self.set_cursor(position.x as f32, position.y as f32)
            }
            WindowEvent::CursorLeft { .. } => self.notch.hover(false),
            WindowEvent::MouseWheel { delta, .. } if self.notch.is_open() => {
                let y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32,
                };
                self.notch.next_page(if y < 0.0 { 1 } else { -1 });
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.logical_key.as_ref() {
                    Key::Named(NamedKey::ArrowRight) => self.notch.next_page(1),
                    Key::Named(NamedKey::ArrowLeft) => self.notch.next_page(-1),
                    Key::Named(NamedKey::Escape) => el.exit(),
                    // Dev shortcuts until real platform event sources exist.
                    Key::Character("c") => self.notch.alert(Alert::Charging),
                    Key::Character("h") => self.notch.alert(Alert::Headphones),
                    _ => {}
                }
            }
            _ => {}
        }
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        let now = Instant::now();
        let dt = now - self.last_tick;
        self.last_tick = now;
        self.notch.tick(dt.min(Duration::from_millis(50)));

        if self.notch.is_animating() {
            el.set_control_flow(ControlFlow::WaitUntil(now + FRAME));
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        } else {
            // Idle: sleep until the next OS event, zero CPU.
            el.set_control_flow(ControlFlow::Wait);
        }
    }
}

fn main() {
    let el = EventLoop::new().expect("event loop");
    let mut app = App {
        window: None,
        surface: None,
        notch: Notch::new(WIN_W),
        last_tick: Instant::now(),
    };
    el.run_app(&mut app).expect("run");
}
