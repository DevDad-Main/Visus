//! Linux host: a wlr-layer-shell surface anchored to the top edge. Falls back to winit if unavailable.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, Region},
    delegate_registry,
    output::{OutputHandler, OutputState},
    reexports::{
        calloop::{self, EventLoop},
        calloop_wayland_source::WaylandSource,
        client::{
            Connection, QueueHandle,
            globals::registry_queue_init,
            protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
        },
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};
use tiny_skia::Pixmap;
use visus_core::{Event, Notch, Outcome};

use crate::{actions, events, lyrics, weather, stats, store, text::Text, ui};

const WIN_W: u32 = 960;
const WIN_H: u32 = 500;
const FRAME: Duration = Duration::from_millis(16);
const BTN_LEFT: u32 = 0x110;
const SCROLL_STEP: f64 = 12.0;

struct State {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor: CompositorState,
    shm: Shm,
    pool: SlotPool,
    layer: LayerSurface,
    pointer: Option<wl_pointer::WlPointer>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    keyboard_grabbed: bool,

    notch: Notch,
    text: Text,
    sampler: Option<stats::Sampler>,
    fetcher: Option<lyrics::Fetcher>,
    emit: Option<visus_platform::Emit>,
    weather: weather::Poller,
    scale: i32,
    cursor: (f32, f32),
    scroll_accum: f64,
    configured: bool,
    needs_draw: bool,
    next_wake: Option<Instant>,
    last_tick: Instant,
    exit: bool,
    hypr: Option<PathBuf>,
    /// Focused monitor's top-centre in global layout coordinates, refreshed occasionally.
    monitor_top: Option<((f32, f32), Instant)>,
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::connect_to_env()?;
    let (globals, event_queue) = registry_queue_init::<State>(&conn)?;
    let qh = event_queue.handle();

    let compositor = CompositorState::bind(&globals, &qh)?;
    let layer_shell = LayerShell::bind(&globals, &qh)?;
    let shm = Shm::bind(&globals, &qh)?;

    let surface = compositor.create_surface(&qh);
    let layer = layer_shell.create_layer_surface(&qh, surface, Layer::Overlay, Some("visus"), None);
    layer.set_anchor(Anchor::TOP);
    layer.set_size(WIN_W, WIN_H);
    // -1: draw over panels instead of being pushed below them.
    layer.set_exclusive_zone(-1);
    layer.set_keyboard_interactivity(KeyboardInteractivity::None);
    layer.commit();

    let pool = SlotPool::new((WIN_W * WIN_H * 4 * 2) as usize, &shm)?;

    let mut notch = Notch::new(WIN_W as f32);
    store::load_into(&mut notch);

    let mut state = State {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        compositor,
        shm,
        pool,
        layer,
        pointer: None,
        keyboard: None,
        keyboard_grabbed: false,
        notch,
        text: Text::new(),
        sampler: stats::Sampler::new(),
        fetcher: None,
        emit: None,
        weather: weather::Poller::default(),
        scale: 1,
        cursor: (-1.0, -1.0),
        scroll_accum: 0.0,
        configured: false,
        needs_draw: true,
        next_wake: None,
        last_tick: Instant::now(),
        exit: false,
        hypr: hypr_socket(),
        monitor_top: None,
    };

    let mut event_loop: EventLoop<State> = EventLoop::try_new()?;
    WaylandSource::new(conn, event_queue).insert(event_loop.handle())?;

    // OS events (media, battery) and lyric results arrive on background threads.
    let (tx, rx) = calloop::channel::channel::<Event>();
    event_loop.handle().insert_source(rx, |ev, _, state| {
        if let calloop::channel::Event::Msg(e) = ev {
            events::apply(&mut state.notch, state.fetcher.as_ref(), e);
            state.needs_draw = true;
        }
    })?;
    let tx = std::sync::Mutex::new(tx);
    let emit: visus_platform::Emit = std::sync::Arc::new(move |ev| {
        if let Ok(tx) = tx.lock() {
            let _ = tx.send(ev);
        }
    });
    state.fetcher = Some(lyrics::Fetcher::new(emit.clone()));
    state.emit = Some(emit.clone());
    visus_platform::set_app_folders(state.notch.settings().app_folders.clone());
    visus_platform::start(emit);

    while !state.exit {
        let timeout = state.step();
        event_loop.dispatch(timeout, &mut state)?;
    }
    Ok(())
}

impl State {
    /// Advances animation, draws if needed, and returns how long the loop may sleep.
    fn step(&mut self) -> Option<Duration> {
        if !self.configured {
            return None;
        }
        let now = Instant::now();
        let dt = now - self.last_tick;
        self.last_tick = now;
        self.sync_keyboard();
        self.feed_eye();
        self.weather.tick(&self.notch, self.emit.as_ref());
        actions::request_icons(&mut self.notch, self.emit.as_ref());
        if self.notch.wants_stats() {
            if let Some(s) = &mut self.sampler {
                self.notch.set_stats(s.sample());
            }
        }
        self.notch.tick(dt.min(Duration::from_millis(50)));
        store::sync(&mut self.notch);

        let animating = self.notch.is_animating();
        let wake_due = self.next_wake.is_some_and(|t| now >= t);
        if animating || self.needs_draw || wake_due {
            self.draw();
            self.needs_draw = false;
        }

        if animating {
            self.next_wake = None;
            return Some(FRAME);
        }
        let wake = ui::idle_wake(&self.notch);
        self.next_wake = wake.map(|d| now + d);
        wake
    }

    /// Wayland hides the global cursor, so ask Hyprland for it while the eye is awake.
    fn feed_eye(&mut self) {
        if self.notch.eye_state() != visus_core::EyeState::Awake {
            return;
        }
        let Some(sock) = self.hypr.clone() else { return };
        let stale = self.monitor_top.is_none_or(|(_, at)| at.elapsed() > Duration::from_secs(2));
        if stale {
            let mon = hypr_json(&sock, "j/monitors").and_then(|v| {
                let m = v.as_array()?.iter().find(|m| m["focused"].as_bool() == Some(true))?.clone();
                let scale = m["scale"].as_f64().unwrap_or(1.0) as f32;
                let x = m["x"].as_f64()? as f32 + m["width"].as_f64()? as f32 / scale / 2.0;
                Some((x, m["y"].as_f64()? as f32))
            });
            self.monitor_top = mon.map(|m| (m, Instant::now()));
        }
        let (Some(((mx, my), _)), Some(cur)) = (self.monitor_top, hypr_json(&sock, "j/cursorpos")) else { return };
        let (Some(cx), Some(cy)) = (cur["x"].as_f64(), cur["y"].as_f64()) else { return };
        self.notch.set_cursor_tracking(true);
        let cy_pill = my + self.notch.settings().closed_height / 2.0;
        self.notch.set_eye_target((cx as f32 - mx) / 500.0, (cy as f32 - cy_pill) / 300.0);
    }

    /// Grabs the keyboard while the launcher is open so typing reaches it, and lets go afterwards.
    fn sync_keyboard(&mut self) {
        let want = self.notch.keyboard_active();
        if want != self.keyboard_grabbed {
            self.keyboard_grabbed = want;
            self.layer.set_keyboard_interactivity(if want {
                KeyboardInteractivity::Exclusive
            } else {
                KeyboardInteractivity::None
            });
            self.layer.commit();
        }
    }

    fn run_outcome(&mut self, outcome: Outcome) {
        if actions::run(outcome, self.emit.as_ref()) {
            self.exit = true;
        }
    }

    fn palette_key(&mut self, event: &KeyEvent) {
        if !self.notch.keyboard_active() {
            return;
        }
        match event.keysym {
            Keysym::Escape => self.notch.key_escape(),
            Keysym::Return | Keysym::KP_Enter => {
                let out = self.notch.key_enter();
                self.run_outcome(out);
            }
            Keysym::BackSpace => self.notch.key_backspace(),
            Keysym::Down | Keysym::Tab => self.notch.key_move(1),
            Keysym::Up | Keysym::ISO_Left_Tab => self.notch.key_move(-1),
            _ => {
                if let Some(t) = &event.utf8 {
                    self.notch.key_text(t);
                }
            }
        }
        self.needs_draw = true;
    }

    fn draw(&mut self) {
        let scale = self.scale.max(1);
        let (w, h) = (WIN_W as i32 * scale, WIN_H as i32 * scale);
        let rows = ((ui::content_bottom(&self.notch) * scale as f32).ceil() as i32).clamp(1, h);
        let Some(mut pixmap) = Pixmap::new(w as u32, rows as u32) else { return };
        ui::draw(&mut pixmap, &self.text, &self.notch, scale as f32);

        let Ok((buffer, canvas)) =
            self.pool.create_buffer(w, h, w * 4, wl_shm::Format::Argb8888)
        else {
            return;
        };
        // Premultiplied RGBA -> little-endian ARGB8888 (bytes B, G, R, A).
        for (dst, src) in canvas.chunks_exact_mut(4).zip(pixmap.pixels()) {
            dst.copy_from_slice(&[src.blue(), src.green(), src.red(), src.alpha()]);
        }
        canvas[(rows * w * 4) as usize..].fill(0);

        let surface = self.layer.wl_surface();
        let _ = self.layer.set_buffer_scale(scale as u32);
        surface.damage_buffer(0, 0, w, h);

        // Only the pill takes input; everything around it is click-through.
        let r = ui::hit_rect(&self.notch);
        if let Ok(region) = Region::new(&self.compositor) {
            region.add(r.x.floor() as i32, 0, r.w.ceil() as i32, r.h.ceil() as i32);
            self.layer.set_input_region(Some(region.wl_region()));
        }

        if buffer.attach_to(surface).is_ok() {
            self.layer.commit();
        }
    }

    fn pointer_moved(&mut self, pos: (f64, f64)) {
        self.cursor = ui::to_logical(&self.notch, pos.0 as f32, pos.1 as f32);
        let inside = self.notch.shape().rect.contains(self.cursor.0, self.cursor.1);
        self.notch.hover(inside);
        self.needs_draw = true;
    }
}

impl CompositorHandler for State {
    fn scale_factor_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, factor: i32) {
        self.scale = factor.max(1);
        self.needs_draw = true;
    }

    fn transform_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: wl_output::Transform) {}

    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}

    fn surface_enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}

    fn surface_leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
}

impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl LayerShellHandler for State {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.exit = true;
    }

    fn configure(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface, _: LayerSurfaceConfigure, _: u32) {
        self.configured = true;
        self.needs_draw = true;
    }
}

impl SeatHandler for State {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(&mut self, _: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat, cap: Capability) {
        if cap == Capability::Pointer && self.pointer.is_none() {
            self.pointer = self.seat_state.get_pointer(qh, &seat).ok();
        }
        if cap == Capability::Keyboard && self.keyboard.is_none() {
            self.keyboard = self.seat_state.get_keyboard(qh, &seat, None).ok();
        }
    }

    fn remove_capability(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat, cap: Capability) {
        if cap == Capability::Pointer {
            if let Some(p) = self.pointer.take() {
                p.release();
            }
        }
        if cap == Capability::Keyboard {
            if let Some(k) = self.keyboard.take() {
                k.release();
            }
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl PointerHandler for State {
    fn pointer_frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_pointer::WlPointer, events: &[PointerEvent]) {
        for ev in events {
            if &ev.surface != self.layer.wl_surface() {
                continue;
            }
            match ev.kind {
                PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => self.pointer_moved(ev.position),
                PointerEventKind::Leave { .. } => {
                    self.notch.hover(false);
                    self.needs_draw = true;
                }
                PointerEventKind::Press { button, .. } if button == BTN_LEFT => {
                    self.pointer_moved(ev.position);
                    let out = ui::click(&mut self.notch, self.cursor.0, self.cursor.1);
                    self.run_outcome(out);
                    self.needs_draw = true;
                }
                PointerEventKind::Axis { vertical, .. } if self.notch.is_open() => {
                    self.scroll_accum += vertical.absolute;
                    if self.scroll_accum.abs() >= SCROLL_STEP {
                        self.notch.next_page(if self.scroll_accum > 0.0 { 1 } else { -1 });
                        self.scroll_accum = 0.0;
                        self.needs_draw = true;
                    }
                }
                _ => {}
            }
        }
    }
}

impl KeyboardHandler for State {
    fn enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _: u32, _: &[u32], _: &[Keysym]) {}

    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _: u32) {}

    fn press_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, event: KeyEvent) {
        self.palette_key(&event);
    }

    fn repeat_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, event: KeyEvent) {
        self.palette_key(&event);
    }

    fn release_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, _: KeyEvent) {}

    fn update_modifiers(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, _: Modifiers, _: RawModifiers, _: u32) {}
}

impl ShmHandler for State {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_registry!(State);

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

smithay_client_toolkit::delegate_dispatch2!(State);

fn hypr_socket() -> Option<PathBuf> {
    let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
    let rt = std::env::var("XDG_RUNTIME_DIR").ok()?;
    Some(PathBuf::from(rt).join("hypr").join(sig).join(".socket.sock"))
}

/// One request/response round trip on Hyprland's IPC socket.
fn hypr_json(sock: &Path, cmd: &str) -> Option<serde_json::Value> {
    let mut s = UnixStream::connect(sock).ok()?;
    s.set_read_timeout(Some(Duration::from_millis(100))).ok()?;
    s.write_all(cmd.as_bytes()).ok()?;
    // Hyprland may keep the socket open, so stop as soon as a full JSON document has arrived.
    let mut out = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = s.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        out.extend_from_slice(&chunk[..n]);
        if let Ok(v) = serde_json::from_slice(&out) {
            return Some(v);
        }
    }
    serde_json::from_slice(&out).ok()
}
