# Visus

"To see, to have vision." A tiny always-on-top notch/pill at the top of the screen that shows
what matters at a glance (music, lyrics, timers, files, alerts) and gets out of the way. Stable and
fast first, feature-rich second.

Inspired by [Nusa](https://rama2555.github.io/nusa/) (Windows-only Dynamic-Island-style notch,
Rust, ~30 MB RAM, 0% idle CPU). We take the idea and the pill feel, not the exact look.

## Decisions

- Platform order: **Windows first**, then macOS, then Linux. Development currently happens on Linux
  (Arch/Hyprland), so the UI shell must run on Linux for iteration while Windows code is written blind
  behind `cfg(windows)` and tested on a Windows machine/VM.
- Stack: **native Rust, no webview**. Currently `winit` + `softbuffer` + `tiny-skia` (CPU raster, tiny).
  A move to `wgpu` is allowed later if CPU raster becomes a bottleneck; keep drawing code behind the
  `draw()` function in `visus-app` so it is swappable.
- Performance contract (do not break):
  - Idle = zero CPU. Event loop uses `ControlFlow::Wait` when `Notch::is_animating()` is false.
  - Event-driven OS integration (subscribe, never poll in a loop).
  - Files on the shelf are stored as references, not copied into memory.
  - Each page/plugin must be isolated so one failure cannot crash the notch.
- Privacy: local-only data, no accounts, no analytics. Only network use: lyrics (LRCLIB), weather.
  Keyboard hook must only react to media/volume/brightness/Caps Lock/language keys, never log typing.

## Reference app (Nusa) feature inventory

Behaviour: hover to open, move away to close; pages switched by mouse wheel or left/right arrows;
page-dot bar turns into icons on hover; hides for fullscreen apps; minimal mode = thin black line;
idle mascot (eyes that doze off and wave on return); tray menu (autostart, mascot, minimal notch,
about, exit); settings window.

Pages/features: Music (any player, seek, volume), synced lyrics (split pill, click line to seek),
Pomodoro timer, water tracker (goal, chart, Drink button), file shelf (drag in/out, multi-select,
text cards), to-do with reminders + snooze, HUD overlays (volume, brightness, Caps Lock, keyboard
language), status peeks (charger, Wi-Fi loss, Bluetooth headphones with battery %).

Visual language (from screenshots): pure black pill attached to the top edge, flat top, large rounded
bottom corners (~22 px closed, ~44 px open), small inverse "flare" at the top corners; white text on
black; thin white progress bars; circular ring icons (green charging, blue Bluetooth); lyrics pane has a
dark-to-blue vertical gradient with the current line bold white and neighbours faded; closed state
shows ring + clock + equalizer glyph; open state shows album art, title/artist, elapsed/remaining,
transport controls, page dots at the bottom.

## Visus additions (user-selected)

Calendar / upcoming events, weather, system stats (CPU/RAM/net), notifications center, quick
launcher / command palette, screenshot & screen-recording shortcuts, plugin system (planned as WASM via
`wasmtime` with a capability-based API; each plugin declares pages + permissions).

## Architecture

Cargo workspace in `crates/`:

| Crate | Status | Purpose |
|---|---|---|
| `visus-core` | done (skeleton) | Platform-independent state: `Notch` (modes Closed/Peek/Open, hover, page switching, alert peeks), `Spring` animation. No OS or rendering code. Unit tested. |
| `visus-app` | done (skeleton) | winit window (borderless, transparent, always-on-top, top-center), tiny-skia drawing, softbuffer present, input handling. |
| `visus-platform-windows` | not started | SMTC media sessions, power/network/Bluetooth events, keyboard hook for HUD keys, click-through outside the pill (`WS_EX_TRANSPARENT`/hit-test), fullscreen detection, tray icon, drag & drop. |
| `visus-pages` | not started | One module per feature page. |
| `visus-plugins` | not started | WASM plugin host. |

Rule: `visus-core` must stay free of platform and rendering dependencies so macOS/Linux ports reuse it.

## Current state (what works)

- `cargo test -p visus-core` passes (spring settles, hover open/close, alert expiry, page wrap).
- `cargo run -p visus-app` opens an 800x280 transparent window at top-center showing a black pill that
  animates (springs) between closed / peek / open. Hover opens it, leaving closes after 250 ms.
- Dev keys: `C` = charging alert peek, `H` = headphones peek, Left/Right or wheel = change page,
  `Esc` = quit.
- Drawing is shapes only: pill, alert circle, page dots. **No text yet.**

## Known gaps / caveats

- No text rendering. Next step: `fontdue` or `ab_glyph` (already in the tree via softbuffer deps) with a
  bundled permissively licensed font (e.g. Inter, OFL). Needed for clock, titles, lyrics.
- Window is a fixed 800x280 transparent rectangle, so the empty area around the pill is not
  click-through yet. Needs per-platform input region/hit-testing.
- Positioning is top-center of primary monitor; on Wayland compositors, `set_outer_position` is
  ignored. Use a layer-shell surface for the Linux port (`wlr-layer-shell`).
- Alpha path: softbuffer expects premultiplied 0xAARRGGBB; verify transparency on Windows.
- Notch dimensions are hard-coded constants in `visus-core/src/notch.rs`; make them DPI/monitor aware.
- Top-corner flares from the reference design are not drawn.

## Roadmap

1. Text rendering + closed-state clock; open-state Music page layout (art, title, progress, controls).
2. Data-driven page system: `Page` trait/registry replacing the `Page` enum, per-page layout and
   events; settings store (JSON/SQLite, local).
3. Windows platform crate: SMTC media, battery/charger, Bluetooth, tray, click-through, fullscreen hide.
4. Pages: lyrics (LRCLIB, cached), Pomodoro, water, to-do + reminders, shelf (drag & drop).
5. HUD overlays (volume, brightness, Caps Lock, language) via keyboard hook.
6. Visus extras: calendar, weather, system stats, notifications center, launcher, capture shortcuts.
7. Plugin host (WASM) once the page API is stable.
8. Idle mascot, minimal mode, settings window, installer/MS Store packaging.
9. macOS and Linux (layer-shell) ports.

### Settings and idle eye (requested)

- **Settings**: tray menu entry opens a settings window (autostart, minimal notch, hide in fullscreen,
  notch size, mascot on/off, eye style, enabled pages/plugins). Every change applies instantly and is
  persisted to the local settings store. Core holds a `Settings` struct; UI is a separate window.
- **Idle eye**: after N seconds with no interaction the clock fades and a single eye appears in the closed
  pill. It follows the global cursor (needs cursor position from the platform layer, polled only while the
  eye is visible), dozes off after a longer idle period, and wakes with a blink when the user returns.
  Redraws only while the pupil moves, to keep idle CPU near zero.
- **Eye styles** (later, selectable in settings): default white eye; "Sauron" eye with a slit pupil, fiery
  glow, and a sweeping searchlight cone that scans the screen. Styles are a `EyeStyle` trait/enum in
  `visus-ui` so plugins can add more.

## Dev notes

- Build/run: `cargo run -p visus-app`; tests: `cargo test`.
- Release profile is size/speed tuned (LTO, strip, panic=abort).
- Cross-check for Windows: `cargo check --target x86_64-pc-windows-gnu` (needs `rustup target add`).
- Only the user's own screenshots of Nusa are available as visual reference; ask for more of the
  other pages (timer, water, shelf, to-do) before designing them.
