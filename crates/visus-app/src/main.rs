#![cfg_attr(windows, windows_subsystem = "windows")]

mod actions;
mod artfetch;
mod calendar;
mod downloads;
mod events;
mod lyrics;
mod game;
mod stats;
mod updater;
mod weather;
mod store;
mod text;
#[cfg(target_os = "linux")]
mod wayland;
#[cfg(windows)]
mod tray;
mod ui;

use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use softbuffer::{Context, Surface};
use tiny_skia::Pixmap;
use visus_core::{Alert, Event, Notch, Outcome};
use visus_platform as platform;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId, WindowLevel};

const WIN_W: f32 = 960.0;
const WIN_H: f32 = 520.0;
const FRAME: Duration = Duration::from_millis(16);

/// A user-facing setting changed from the tray (or later, a settings window).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(not(windows), allow(dead_code))]
pub enum Change {
    Scale(f32),
    EyeIris(visus_core::Rgb),
    ToggleMinimal,
    ToggleMascot,
    ToggleHideFullscreen,
}

impl Change {
    fn apply(self, s: &mut visus_core::Settings) {
        match self {
            Change::Scale(k) => s.scale = k,
            Change::EyeIris(c) => s.eye_iris = c,
            Change::ToggleMinimal => s.minimal = !s.minimal,
            Change::ToggleMascot => s.mascot = !s.mascot,
            Change::ToggleHideFullscreen => s.hide_in_fullscreen = !s.hide_in_fullscreen,
        }
    }
}

#[derive(Debug)]
#[cfg_attr(not(windows), allow(dead_code))]
pub enum UserEvent {
    Os(Event),
    Setting(Change),
    ToggleAutostart,
    #[cfg(windows)]
    Uninstall,
    Quit,
}

struct App {
    proxy: EventLoopProxy<UserEvent>,
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    hwnd: isize,
    notch: Notch,
    text: text::Text,
    sampler: Option<stats::Sampler>,
    fetcher: Option<lyrics::Fetcher>,
    emit: Option<platform::Emit>,
    weather: weather::Poller,
    cursor: (f32, f32),
    shown: bool,
    focus_requested: bool,
    last_region: Option<(i32, i32, i32, i32)>,
    last_cursor: Option<(i32, i32)>,
    last_tick: Instant,
    next_frame: Instant,
    frame: Duration,
    shift: bool,
    hand: bool,
    pending_click: Option<(f32, f32, Instant)>,
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
        // Rows below the pill and launcher are always transparent, so they are not rasterised.
        let rows = ((ui::content_bottom(&self.notch) * scale).ceil() as u32).clamp(1, size.height);
        let Some(mut pixmap) = Pixmap::new(size.width, rows) else { return };

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
        buf[(rows * size.width) as usize..].fill(0);
        window.pre_present_notify();
        buf.present().unwrap();

        if self.hwnd != 0 {
            let animating = self.notch.is_animating();
            let r = if animating { ui::region_rect(&self.notch) } else { ui::hit_rect(&self.notch) };
            let radius = self.notch.shape().bottom_radius * self.notch.settings().scale;
            let mut region = (
                (r.x * scale).floor() as i32,
                (r.w * scale).ceil() as i32,
                (r.h * scale).ceil() as i32,
                (radius * scale) as i32,
            );
            // While animating the region only grows, so it changes once instead of every frame.
            if let (true, Some(prev)) = (animating, self.last_region) {
                let left = region.0.min(prev.0);
                let right = (region.0 + region.1).max(prev.0 + prev.1);
                region = (left, right - left, region.2.max(prev.2), region.3);
            }
            // SetWindowRgn repaints the window, so repeating it every frame makes it flicker.
            if self.last_region != Some(region) {
                self.last_region = Some(region);
                platform::set_window_region(self.hwnd, region.0, 0, region.1, region.2, region.3);
            }
        }
    }

    /// Points the idle eye at the global cursor where the OS exposes it.
    fn feed_eye(&mut self) {
        use visus_core::EyeState;
        let state = self.notch.eye_state();
        if state != EyeState::Awake && state != EyeState::Asleep {
            return;
        }
        let (Some(w), Some((cx, cy))) = (&self.window, platform::cursor_pos()) else { return };
        let moved = self.last_cursor.is_none_or(|(x, y)| (x - cx).abs() + (y - cy).abs() > 3);
        self.last_cursor = Some((cx, cy));
        if moved {
            self.notch.note_motion();
        }
        if self.notch.eye_state() != EyeState::Awake {
            self.notch.set_cursor_tracking(true);
            return;
        }
        let Ok(origin) = w.outer_position() else { return };
        self.notch.set_cursor_tracking(true);
        let k = w.scale_factor() as f32;
        let r = self.notch.shape().rect;
        let px = origin.x as f32 + (r.x + r.w / 2.0) * k;
        let py = origin.y as f32 + (r.h / 2.0) * k;
        self.notch.set_eye_target((cx as f32 - px) / (500.0 * k), (cy as f32 - py) / (300.0 * k));
    }

    fn sync_visibility(&mut self) {
        // A game can cover the pill; keep re-asserting it while the HUD is meant to be showing.
        static LAST_RAISE: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);
        if self.notch.game_active() && self.hwnd != 0 {
            let mut last = LAST_RAISE.lock().unwrap();
            if last.is_none_or(|t| t.elapsed() > Duration::from_millis(1500)) {
                *last = Some(Instant::now());
                platform::raise_window(self.hwnd);
            }
        }
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
        self.cursor = ui::to_logical(&self.notch, x / s, y / s);
        let inside = self.notch.shape().rect.contains(self.cursor.0, self.cursor.1);
        self.notch.hover(inside);
        self.notch.set_pointer(Some(self.cursor));
        ui::hover_palette(&mut self.notch);
        ui::drag(&mut self.notch, self.cursor.0);
        let want = ui::over_clickable(&self.notch);
        if want != self.hand {
            self.hand = want;
            if let Some(w) = &self.window {
                w.set_cursor(winit::window::Cursor::Icon(if want { winit::window::CursorIcon::Pointer } else { winit::window::CursorIcon::Default }));
            }
        }
    }

    fn click(&mut self, el: &ActiveEventLoop) {
        let (x, y) = self.cursor;
        // A click that lands while the pill is still growing open is replayed once its buttons exist.
        let growing = self.notch.is_open() && ui::buttons(&self.notch).is_empty() && !self.notch.palette_is_open() && !self.notch.settings_is_open();
        if growing {
            self.pending_click = Some((x, y, Instant::now()));
            return;
        }
        let outcome = ui::click(&mut self.notch, x, y);
        self.run_outcome(el, outcome);
    }

    fn run_outcome(&mut self, el: &ActiveEventLoop, outcome: Outcome) {
        let folders = self.notch.settings().app_folders.clone();
        if actions::run(outcome, self.emit.as_ref(), &folders) {
            el.exit();
        }
    }

    /// Takes keyboard focus while the launcher is open and gives it back afterwards.
    fn sync_focus(&mut self) {
        let want = self.notch.keyboard_active();
        if want == self.focus_requested {
            return;
        }
        self.focus_requested = want;
        if self.hwnd != 0 {
            platform::set_focusable(self.hwnd, want);
        }
        if let (true, Some(w)) = (want, &self.window) {
            w.focus_window();
        }
    }

    /// Returns true when the key was consumed by the launcher.
    fn palette_key(&mut self, el: &ActiveEventLoop, ev: &winit::event::KeyEvent) -> bool {
        if !self.notch.keyboard_active() {
            return false;
        }
        match ev.logical_key.as_ref() {
            Key::Named(NamedKey::Escape) => self.notch.key_escape(),
            Key::Named(NamedKey::Enter) => {
                let out = self.notch.key_enter();
                self.run_outcome(el, out);
            }
            Key::Named(NamedKey::Backspace) => self.notch.key_backspace(),
            Key::Named(NamedKey::ArrowDown | NamedKey::Tab) => self.notch.key_move(1),
            Key::Named(NamedKey::ArrowUp) => self.notch.key_move(-1),
            _ => {
                if let Some(t) = &ev.text {
                    self.notch.key_text(t);
                }
            }
        }
        true
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
        // Pace animation to the monitor's refresh rate (60 to 144 Hz).
        if let Some(mhz) = self.window.as_ref().and_then(|w| w.current_monitor()).and_then(|m| m.refresh_rate_millihertz()) {
            self.frame = Duration::from_micros((1_000_000_000.0 / mhz as f64).clamp(6900.0, 16700.0) as u64);
        }

        let proxy = self.proxy.clone();
        let emit: platform::Emit = Arc::new(move |ev| {
            let _ = proxy.send_event(UserEvent::Os(ev));
        });
        self.fetcher = Some(lyrics::Fetcher::new(emit.clone()));
        self.emit = Some(emit.clone());
        platform::set_app_folders(self.notch.settings().app_folders.clone());
        platform::start(emit);

        #[cfg(windows)]
        {
            self.tray = tray::create(self.proxy.clone(), self.notch.settings());
        }
    }

    fn user_event(&mut self, el: &ActiveEventLoop, ev: UserEvent) {
        match ev {
            UserEvent::Os(e) => {
                events::apply(&mut self.notch, self.fetcher.as_ref(), e);
                self.sync_visibility();
            }
            UserEvent::Setting(c) => {
                self.notch.update_settings(|s| c.apply(s));
                store::sync(&mut self.notch);
            }
            UserEvent::ToggleAutostart => {
                let _ = platform::set_autostart(!platform::autostart_enabled());
                #[cfg(windows)]
                if let Some(t) = &self.tray {
                    t.set_autostart_checked(platform::autostart_enabled());
                }
            }
            #[cfg(windows)]
            UserEvent::Uninstall => {
                // The installer's uninstaller shows its own confirmation and stops Visus.
                if !platform::uninstall() {
                    platform::open_path("ms-settings:appsfeatures");
                }
            }
            UserEvent::Quit => el.exit(),
        }
        self.sync_focus();
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
            WindowEvent::CursorLeft { .. } => {
                self.notch.hover(false);
                self.notch.set_pointer(None);
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                self.click(el)
            }
            WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Left, .. } => self.notch.end_drag(),
            WindowEvent::HoveredFileCancelled => {
                self.notch.set_drop_hover(false);
                self.notch.drag_cancelled();
            }
            WindowEvent::HoveredFile(_) => {
                self.notch.set_drop_hover(true);
                self.notch.show_page(visus_core::Page::Shelf);
            }
            WindowEvent::DroppedFile(path) => {
                self.notch.set_drop_hover(false);
                self.notch.shelf_add(path.to_string_lossy().into_owned());
                self.notch.show_page(visus_core::Page::Shelf);
            }
            WindowEvent::Focused(false) if self.notch.keyboard_active() => self.notch.key_escape(),
            WindowEvent::ModifiersChanged(mods) => self.shift = mods.state().shift_key(),
            WindowEvent::MouseWheel { delta, .. } if self.notch.is_open() => {
                let y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32,
                };
                if self.shift {
                    // Shift + wheel nudges the system volume.
                    let c = self.notch.controls();
                    let base = if c.has_volume { c.volume } else { platform::controls().volume };
                    let v = (base + if y > 0.0 { 0.05 } else { -0.05 }).clamp(0.0, 1.0);
                    self.notch.set_volume_local(v);
                    platform::set_volume(v);
                } else {
                    self.notch.next_page(if y < 0.0 { 1 } else { -1 });
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if self.palette_key(el, &event) {
                    // Consumed by the launcher.
                } else {
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
            }
            _ => {}
        }
        self.sync_focus();
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        let now = Instant::now();
        let dt = now - self.last_tick;
        self.last_tick = now;
        self.feed_eye();
        self.weather.tick(&self.notch, self.emit.as_ref());
        actions::pump(&mut self.notch, self.emit.as_ref());
        if self.notch.wants_stats() {
            if let Some(s) = &mut self.sampler {
                self.notch.set_stats(s.sample());
            }
        }
        if let Some((x, y, at)) = self.pending_click {
            if at.elapsed() > Duration::from_millis(600) {
                self.pending_click = None;
            } else if !ui::buttons(&self.notch).is_empty() {
                self.pending_click = None;
                let out = ui::click(&mut self.notch, x, y);
                self.run_outcome(el, out);
            }
        }
        self.notch.tick(dt.min(Duration::from_millis(50)));
        store::sync(&mut self.notch);

        if self.notch.is_animating() {
            // Fixed cadence: render time does not stretch the frame interval.
            self.next_frame = (self.next_frame + self.frame).max(now);
            el.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
            if let Some(w) = &self.window {
                w.request_redraw();
            }
            return;
        }

        let wake = ui::idle_wake(&self.notch);
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

/// A synthwave sunset used as demo cover art for captures (`VISUS_ART=1,2,3`).
fn synth_cover() -> Vec<u8> {
    let n = 128usize;
    let mut out = Vec::with_capacity(n * n * 4);
    for y in 0..n {
        for x in 0..n {
            let (fx, fy) = (x as f32 / n as f32, y as f32 / n as f32);
            let mix = |a: [f32; 3], b: [f32; 3], t: f32| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
            let mut c = mix([22.0, 8.0, 60.0], [255.0, 80.0, 120.0], (fy / 0.62).min(1.0).powf(1.4));
            let (dx, dy) = (fx - 0.5, fy - 0.46);
            if (dx * dx + dy * dy).sqrt() < 0.24 && !(fy > 0.5 && ((fy * 60.0) as i32 % 4 == 0)) {
                c = mix([255.0, 214.0, 90.0], [255.0, 60.0, 140.0], ((fy - 0.22) / 0.48).clamp(0.0, 1.0));
            }
            if fy > 0.62 {
                let g = ((fx - 0.5) / (fy - 0.6) * 0.5).fract().abs();
                let line = (fy * 40.0).fract() < 0.08 || g < 0.04 || g > 0.96;
                c = if line { [255.0, 70.0, 200.0] } else { [14.0, 6.0, 36.0] };
            }
            out.extend_from_slice(&[c[0] as u8, c[1] as u8, c[2] as u8, 255]);
        }
    }
    out
}

/// `visus-app --snapshot <music|timer|stats|shelf|todo> <out.png>`: renders the open pill headlessly.
fn snapshot(page: &str, out: &str, query: Option<&str>) {
    let mut notch = Notch::new(WIN_W);
    notch.hover(true);
    for _ in 0..200 {
        notch.tick(Duration::from_millis(16));
    }
    // Let the OS event sources report once so the snapshot shows real media/battery state.
    let (tx, rx) = std::sync::mpsc::channel();
    let tx = std::sync::Mutex::new(tx);
    let emit: platform::Emit = Arc::new(move |ev| {
        if let Ok(tx) = tx.lock() {
            let _ = tx.send(ev);
        }
    });
    let fetcher = lyrics::Fetcher::new(emit.clone());
    platform::start(emit.clone());
    let wait = if std::env::var_os("VISUS_ART").is_some() || std::env::var_os("VISUS_FAST").is_some() { 400 } else { 6000 };
    let deadline = std::time::Instant::now() + Duration::from_millis(wait);
    let mut got_apps = false;
    while std::time::Instant::now() < deadline {
        if let Ok(ev) = rx.recv_timeout(Duration::from_millis(200)) {
            got_apps |= matches!(ev, Event::Apps(_));
            events::apply(&mut notch, Some(&fetcher), ev);
        }
        if (page == "palette" || page == "genie") && got_apps {
            break;
        }
        if matches!(notch.lyrics(), visus_core::Lyrics::Ready(_) | visus_core::Lyrics::NotFound) {
            break;
        }
    }
    if page == "lyrics" {
        notch.toggle_lyrics();
    }
    if page == "todo" {
        let task = |t: &str, done: bool, due: Option<u16>| visus_core::Todo { text: t.into(), done, due };
        notch.set_todos(vec![task("Review the release notes", false, Some(15 * 60)), task("Reply to Dina", false, None), task("Ship the alpha", true, None)]);
        notch.begin_todo_input();
        notch.key_text("Buy milk @18:00");
    }
    if let Some(c) = std::env::var("VISUS_ART").ok().and_then(|s| s.split(',').map(|v| v.trim().parse::<u8>().ok()).collect::<Option<Vec<_>>>()).filter(|v| v.len() == 3) {
        let rgba: Vec<u8> = (0..64 * 64).flat_map(|i| { let f = 0.6 + 0.4 * ((i / 64) as f32 / 63.0); [(c[0] as f32 * f) as u8, (c[1] as f32 * f) as u8, (c[2] as f32 * f) as u8, 255] }).collect();
        let rgba = if c == [1, 2, 3] { synth_cover() } else { rgba };
        notch.handle(Event::Media(visus_core::MediaInfo { has_session: true, title: "Midnight City".into(), artist: "M83".into(), playing: true, position: Duration::from_secs(80), duration: Duration::from_secs(244), art: Some(Arc::new(visus_core::Art { size: if c == [1, 2, 3] { 128 } else { 64 }, rgba, id: 77 })) }));
        for _ in 0..200 {
            notch.tick(Duration::from_millis(16));
        }
    }
    if page == "windows" {
        notch.handle(Event::Windows(Arc::new(vec![(1, "New Tab - Brave".into()), (2, "visus - Visual Studio Code".into()), (3, "Spotify Premium".into()), (4, "Downloads - File Explorer".into()), (5, "Inbox (3) - Mail".into()), (6, "Terminal".into()), (7, "Roadmap.pdf - Document Viewer".into()), (8, "Lo-fi beats - YouTube - Brave".into())])));
    }
    if page == "shelf" {
        notch.set_shelf(vec!["/home/devdad/Projects/visus/Cargo.toml".into(), "/usr/bin/ls".into(), "/tmp/missing-report.pdf".into()]);
    }
    if let Ok(v) = std::env::var("VISUS_CLOCK") {
        let style = visus_core::ClockStyle::ALL.into_iter().find(|c| c.name().eq_ignore_ascii_case(&v));
        if let Some(st) = style {
            notch.update_settings(|s| s.clock_style = st);
        }
    }
    if let Some(p) = std::env::var("VISUS_POINTER").ok().and_then(|s| s.split_once(',').and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?)))) {
        notch.set_pointer(Some(p));
    }
    if page == "notes" {
        notch.set_notes(vec!["Call the dentist about Thursday".into(), "Idea: genie close for the settings popup".into(), "https://github.com/DevDad-Main/Visus".into()]);
    }
    if page == "clipboard" {
        for t in ["cargo build --release", "meeting-notes-final-v2.pdf", "The quick brown fox jumps over the lazy dog"] {
            notch.handle(Event::Clipboard(t.into()));
        }
    }
    if page == "calendar" {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
        notch.handle(Event::Calendar(Arc::new(vec![
            visus_core::CalEvent { start: now + 3000, end: now + 6600, title: "Design review".into(), all_day: false, url: Some("https://meet.google.com/abc".into()) },
            visus_core::CalEvent { start: now + 90000, end: now + 93600, title: "Dentist".into(), all_day: false, url: None },
        ])));
    }
    if page == "settings" {
        notch.open_settings();
        for _ in 0..200 {
            notch.tick(Duration::from_millis(16));
        }
    }
    if page == "palette" || page == "genie" {
        notch.open_palette();
        notch.palette_type(if page == "genie" { "" } else { query.unwrap_or("") });
        // Icons load off-thread; give them a moment so the snapshot shows them.
        for _ in 0..8 {
            actions::request_icons(&mut notch, Some(&emit));
            if let Ok(ev) = rx.recv_timeout(Duration::from_millis(150)) {
                events::apply(&mut notch, Some(&fetcher), ev);
            }
        }
        for _ in 0..200 {
            notch.tick(Duration::from_millis(16));
        }
    }
    if page == "genie" {
        notch.close_palette();
        for _ in 0..query.and_then(|q| q.parse().ok()).unwrap_or(10) {
            notch.tick(Duration::from_millis(16));
        }
    }
    if page == "alert" || page == "download" {
        let q = query.unwrap_or("charging:robot");
        let (name, style) = q.split_once(':').unwrap_or((q, "classic"));
        notch.update_settings(|s| {
            s.eye_style = match style {
                "robot" => visus_core::EyeStyle::Robot,
                "cat" => visus_core::EyeStyle::Cat,
                "sauron" => visus_core::EyeStyle::Sauron,
                _ => visus_core::EyeStyle::Classic,
            }
        });
        notch.hover(false);
        for _ in 0..300 {
            notch.tick(Duration::from_millis(16));
        }
        if page == "download" {
            notch.handle(Event::Power { on_ac: true, percent: 76 });
            notch.handle(Event::Download(Some(12_500_000)));
        } else {
            let alert = match name {
                "unplugged" => visus_core::Alert::Unplugged { percent: 40 },
                "headphones" => visus_core::Alert::Headphones,
                "offline" => visus_core::Alert::Offline,
                "timer" => visus_core::Alert::TimerDone,
                "todo" => visus_core::Alert::TodoDue,
                "controller" => visus_core::Alert::Controller(true),
                "controller_off" => visus_core::Alert::Controller(false),
                "online" => visus_core::Alert::Online,
                "meeting" => visus_core::Alert::Meeting { mins: 5 },
                "update" => {
                    notch.handle(Event::Update(visus_core::UpdateStatus::Available { version: "v1.9.0".into() }));
                    visus_core::Alert::UpdateAvailable
                }
                "caps" => visus_core::Alert::CapsLock(true),
                "volume" => visus_core::Alert::Volume { level: 0.62, muted: false },
                "download-done" => {
                    notch.handle(Event::DownloadDone("Visus-Setup.exe".into()));
                    visus_core::Alert::DownloadDone
                }
                _ => visus_core::Alert::Charging { percent: 80 },
            };
            notch.alert(alert);
        }
        // `VISUS_FRAMES=n` records the reaction in real time as n numbered PNGs.
        let frames: usize = std::env::var("VISUS_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
        if frames > 0 {
            let step: u64 = std::env::var("VISUS_STEP_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(70);
            let text = text::Text::new();
            for i in 0..frames {
                let mut pm = Pixmap::new(WIN_W as u32 * 2, 240).expect("pixmap");
                ui::draw(&mut pm, &text, &notch, 2.0);
                pm.save_png(format!("{}_{i:03}.png", out.trim_end_matches(".png"))).expect("write png");
                std::thread::sleep(Duration::from_millis(step));
                for _ in 0..(step / 16).max(1) {
                    notch.tick(Duration::from_millis(16));
                }
            }
            return;
        }
        for _ in 0..60 {
            notch.tick(Duration::from_millis(16));
        }
        let mut pm = Pixmap::new(WIN_W as u32 * 2, 240).expect("pixmap");
        ui::draw(&mut pm, &text::Text::new(), &notch, 2.0);
        pm.save_png(out).expect("write png");
        return;
    }
    if page == "closed" {
        notch.hover(false);
        notch.handle(Event::Power { on_ac: true, percent: 76 });
        for i in 0..400 {
            notch.set_audio_level(Some(0.25 + 0.2 * (i as f32 * 0.35).sin().abs() + 0.15 * (i as f32 * 0.9).cos().abs()));
            notch.tick(Duration::from_millis(16));
        }
        let mut pm = Pixmap::new(WIN_W as u32 * 2, 240).expect("pixmap");
        ui::draw(&mut pm, &text::Text::new(), &notch, 2.0);
        pm.save_png(out).expect("write png");
        return;
    }
    if page == "game" {
        notch.hover(false);
        for _ in 0..300 {
            notch.tick(Duration::from_millis(16));
        }
        notch.handle(Event::Game(true));
        if let Some(st) = std::env::var("VISUS_HUD").ok().and_then(|v| [visus_core::GameStyle::Pills, visus_core::GameStyle::Bars, visus_core::GameStyle::Card].into_iter().find(|s| s.name().eq_ignore_ascii_case(&v))) {
            notch.update_settings(|s| s.game_style = st);
        }
        for i in 0..40 {
            let t = i as f32 / 39.0;
            notch.handle(Event::GameStats(visus_core::GameStats {
                cpu: 0.45 + 0.1 * (t * 9.0).sin(),
                cpu_temp: Some(71.0),
                gpu: Some(0.80 + 0.12 * (t * 6.0).sin() * (1.0 - t * 0.3)),
                gpu_temp: Some(76.0),
                ram: 0.38,
                ram_used_gb: 12.1,
            }));
        }
        for _ in 0..300 {
            notch.tick(Duration::from_millis(16));
        }
        let mut pm = Pixmap::new(WIN_W as u32 * 2, 520).expect("pixmap");
        ui::draw(&mut pm, &text::Text::new(), &notch, 2.0);
        pm.save_png(out).expect("write png");
        return;
    }
    if matches!(page, "eye" | "sauron" | "cat" | "robot") {
        let style = match page {
            "sauron" => Some(visus_core::EyeStyle::Sauron),
            "cat" => Some(visus_core::EyeStyle::Cat),
            "robot" => Some(visus_core::EyeStyle::Robot),
            _ => None,
        };
        if let Some(st) = style {
            notch.update_settings(|s| s.eye_style = st);
        }
        notch.hover(false);
        for _ in 0..300 {
            notch.tick(Duration::from_millis(16));
        }
        notch.touch();
        notch.rewind_activity(Duration::from_secs_f32(query.and_then(|q| q.parse().ok()).unwrap_or(60.0)));
        let frames: usize = std::env::var("VISUS_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
        let step: u64 = std::env::var("VISUS_STEP_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(70);
        let text = text::Text::new();
        for i in 0..frames {
            if i > 0 {
                notch.rewind_activity(Duration::from_millis(step));
                for _ in 0..(step / 16).max(1) {
                    notch.tick(Duration::from_millis(16));
                }
            }
            let mut pm = Pixmap::new(WIN_W as u32 * 2, 160).expect("pixmap");
            ui::draw(&mut pm, &text, &notch, 2.0);
            let path = if frames > 1 { format!("{}_{i:03}.png", out.trim_end_matches(".png")) } else { out.to_string() };
            pm.save_png(path).expect("write png");
        }
        return;
    }
    let idx = ["music", "control", "windows", "timer", "stats", "shelf", "todo", "notes", "clipboard", "calendar", "weather"].iter().position(|p| *p == page).unwrap_or(0);
    notch.next_page(idx as i32);
    if let Some(mut s) = stats::Sampler::new() {
        s.sample();
        std::thread::sleep(Duration::from_millis(400));
        notch.set_stats(s.sample());
    }
    for _ in 0..std::env::var("VISUS_TICKS").ok().and_then(|v| v.parse().ok()).unwrap_or(0) {
        notch.tick(Duration::from_millis(16));
    }
    let mut pm = Pixmap::new(WIN_W as u32 * 2, WIN_H as u32 * 2).expect("pixmap");
    ui::draw(&mut pm, &text::Text::new(), &notch, 2.0);
    #[cfg(feature = "dev")]
    {
        if std::env::var_os("VISUS_BENCH").is_some() {
        let text = text::Text::new();
        let t = std::time::Instant::now();
        for _ in 0..60 {
            let mut p = Pixmap::new(WIN_W as u32 * 2, WIN_H as u32 * 2).expect("pixmap");
            ui::draw(&mut p, &text, &notch, 2.0);
        }
        eprintln!("draw avg {:.2} ms", t.elapsed().as_secs_f32() * 1000.0 / 60.0);
    }
    }
    pm.save_png(out).expect("write png");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--toggle-palette") {
        std::process::exit(if platform::send_toggle() { 0 } else { 1 });
    }
    std::panic::set_hook(Box::new(|info| {
        if let Some(p) = store::settings_path().map(|p| p.with_file_name("crash.log")) {
            let _ = std::fs::write(p, format!("{info}\n"));
        }
    }));
    if platform::already_running() {
        return;
    }
    if let [_, flag, page, out, rest @ ..] = &args[..] {
        if flag == "--snapshot" {
            snapshot(page, out, rest.first().map(String::as_str));
            return;
        }
    }
    #[cfg(target_os = "linux")]
    if std::env::var_os("WAYLAND_DISPLAY").is_some()
        && std::env::var("VISUS_BACKEND").map_or(true, |v| v != "winit")
    {
        match wayland::run() {
            Ok(()) => return,
            Err(e) => eprintln!("layer-shell unavailable ({e}); using a plain window"),
        }
    }
    let el = EventLoop::<UserEvent>::with_user_event().build().expect("event loop");
    let mut notch = Notch::new(WIN_W);
    store::load_into(&mut notch);
    let mut app = App {
        proxy: el.create_proxy(),
        window: None,
        surface: None,
        hwnd: 0,
        notch,
        text: text::Text::new(),
        sampler: stats::Sampler::new(),
        fetcher: None,
        emit: None,
        weather: weather::Poller::default(),
        cursor: (-1.0, -1.0),
        shown: true,
        focus_requested: false,
        last_region: None,
        last_cursor: None,
        last_tick: Instant::now(),
        next_frame: Instant::now(),
        frame: FRAME,
        shift: false,
        hand: false,
        pending_click: None,
        #[cfg(windows)]
        tray: None,
    };
    el.run_app(&mut app).expect("run");
}
