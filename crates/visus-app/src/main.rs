mod text;
#[cfg(windows)]
mod tray;
mod ui;

use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use softbuffer::{Context, Surface};
use tiny_skia::Pixmap;
use visus_core::{Alert, Event, MediaCommand, Notch};
use visus_platform_windows as platform;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId, WindowLevel};

const WIN_W: f32 = 800.0;
const WIN_H: f32 = 280.0;
const FRAME: Duration = Duration::from_millis(16);

#[derive(Debug)]
pub enum UserEvent {
    Os(Event),
    ToggleAutostart,
    Quit,
}

struct App {
    proxy: EventLoopProxy<UserEvent>,
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    hwnd: isize,
    notch: Notch,
    text: text::Text,
    cursor: (f32, f32),
    shown: bool,
    last_tick: Instant,
    #[cfg(windows)]
    tray: Option<tray::Tray>,
}

impl App {
    fn scale(&self) -> f32 {
        self.window.as_ref().map_or(1.0, |w| w.scale_factor() as f32)
    }

    fn render(&mut self) {
        let scale = self.scale();
        let (Some(window), Some(surface)) = (&self.window, &mut self.surface) else { return };
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        surface.resize(w, h).unwrap();
        let Some(mut pixmap) = Pixmap::new(size.width, size.height) else { return };

        if !self.notch.is_hidden() {
            ui::draw(&mut pixmap, &self.text, &self.notch, scale);
        }

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

        if self.hwnd != 0 {
            let s = self.notch.shape();
            platform::set_window_region(
                self.hwnd,
                (s.rect.x * scale).floor() as i32,
                0,
                (s.rect.w * scale).ceil() as i32,
                (s.rect.h * scale).ceil() as i32,
                (s.bottom_radius * scale) as i32,
            );
        }
    }

    fn sync_visibility(&mut self) {
        let want = !self.notch.is_hidden();
        if want != self.shown {
            self.shown = want;
            if let Some(w) = &self.window {
                w.set_visible(want);
            }
        }
    }

    fn cursor_moved(&mut self, x: f32, y: f32) {
        let s = self.scale();
        self.cursor = (x / s, y / s);
        let inside = self.notch.shape().rect.contains(self.cursor.0, self.cursor.1);
        self.notch.hover(inside);
    }

    fn click(&mut self) {
        let (x, y) = self.cursor;
        for (b, r) in ui::buttons(&self.notch) {
            if r.contains(x, y) {
                match b {
                    ui::Button::Prev => platform::media_command(MediaCommand::Previous),
                    ui::Button::PlayPause => platform::media_command(MediaCommand::PlayPause),
                    ui::Button::Next => platform::media_command(MediaCommand::Next),
                    ui::Button::TimerToggle => self.notch.toggle_timer(),
                    ui::Button::TimerReset => self.notch.reset_timer(),
                }
                return;
            }
        }
    }
}

#[cfg(windows)]
fn native_handle(window: &Window) -> isize {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match window.window_handle().map(|h| h.as_raw()) {
        Ok(RawWindowHandle::Win32(h)) => h.hwnd.get(),
        _ => 0,
    }
}

#[cfg(not(windows))]
fn native_handle(_: &Window) -> isize {
    0
}

fn window_attributes() -> winit::window::WindowAttributes {
    let attrs = Window::default_attributes()
        .with_title("Visus")
        .with_inner_size(LogicalSize::new(WIN_W, WIN_H))
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false)
        .with_window_level(WindowLevel::AlwaysOnTop);
    #[cfg(windows)]
    let attrs = {
        use winit::platform::windows::WindowAttributesExtWindows;
        attrs.with_skip_taskbar(true).with_undecorated_shadow(false)
    };
    attrs
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = Rc::new(el.create_window(window_attributes()).expect("window"));

        if let Some(m) = el.primary_monitor().or_else(|| window.current_monitor()) {
            let win_w = window.outer_size().width as i32;
            let x = m.position().x + (m.size().width as i32 - win_w) / 2;
            window.set_outer_position(PhysicalPosition::new(x, m.position().y));
        }

        self.hwnd = native_handle(&window);
        if self.hwnd != 0 {
            platform::prepare_window(self.hwnd);
        }

        let ctx = Context::new(window.clone()).expect("softbuffer context");
        self.surface = Some(Surface::new(&ctx, window.clone()).expect("surface"));
        self.window = Some(window);
        self.last_tick = Instant::now();

        let proxy = self.proxy.clone();
        platform::start(Arc::new(move |ev| {
            let _ = proxy.send_event(UserEvent::Os(ev));
        }));

        #[cfg(windows)]
        {
            self.tray = tray::create(self.proxy.clone());
        }
    }

    fn user_event(&mut self, el: &ActiveEventLoop, ev: UserEvent) {
        match ev {
            UserEvent::Os(e) => {
                self.notch.handle(e);
                self.sync_visibility();
            }
            UserEvent::ToggleAutostart => {
                let _ = platform::set_autostart(!platform::autostart_enabled());
                #[cfg(windows)]
                if let Some(t) = &self.tray {
                    t.set_autostart_checked(platform::autostart_enabled());
                }
            }
            UserEvent::Quit => el.exit(),
        }
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, ev: WindowEvent) {
        match ev {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::RedrawRequested => self.render(),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_moved(position.x as f32, position.y as f32)
            }
            WindowEvent::CursorLeft { .. } => self.notch.hover(false),
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                self.click()
            }
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
                    // Dev shortcuts for peeking without real OS events.
                    Key::Character("c") => self.notch.alert(Alert::Charging { percent: 70 }),
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
            return;
        }

        // Idle: sleep until the clock minute changes or periodic content (progress) needs a refresh.
        let mut wake = self.notch.refresh_interval();
        if !self.notch.is_hidden() {
            let secs = chrono::Timelike::second(&chrono::Local::now()) as u64;
            let minute = Duration::from_secs(60 - secs) + Duration::from_millis(20);
            wake = Some(wake.map_or(minute, |w| w.min(minute)));
        }
        match wake {
            Some(d) => {
                el.set_control_flow(ControlFlow::WaitUntil(now + d));
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            None => el.set_control_flow(ControlFlow::Wait),
        }
    }
}

fn main() {
    let el = EventLoop::<UserEvent>::with_user_event().build().expect("event loop");
    let mut app = App {
        proxy: el.create_proxy(),
        window: None,
        surface: None,
        hwnd: 0,
        notch: Notch::new(WIN_W),
        text: text::Text::new(),
        cursor: (-1.0, -1.0),
        shown: true,
        last_tick: Instant::now(),
        #[cfg(windows)]
        tray: None,
    };
    el.run_app(&mut app).expect("run");
}
