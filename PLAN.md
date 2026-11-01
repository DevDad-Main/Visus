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
| `visus-core` | working | Platform-independent state: `Notch` (Closed/Peek/Open modes, hover, pages, alerts, timer, stats, lyrics state, idle eye logic), `Settings`, LRC parsing, `Spring` animation, `Event` type. No OS or rendering code. Unit tested. |
| `visus-app` | working | Two hosts sharing one renderer (`ui.rs`, tiny-skia + bundled Inter font): `main.rs` (winit; Windows/X11/macOS) and `wayland.rs` (wlr-layer-shell via smithay-client-toolkit; Linux default). Also: settings persistence (`store.rs`), LRCLIB lookups (`lyrics.rs`), CPU/RAM/net sampling (`stats.rs`), Windows tray (`tray.rs`). |
| `visus-platform` | Windows + Linux working | Windows: SMTC media, charger, volume/Caps Lock key HUDs, fullscreen detection, click-through region, autostart, cursor position. Linux: MPRIS media, UPower battery, XDG autostart. macOS: stubs. |
| `visus-pages` | not started | Would hold one module per page once the page API is data-driven. |
| `visus-plugins` | not started | WASM plugin host. |

Rule: `visus-core` must stay free of platform and rendering dependencies.

## Current state (what works)

- Closed pill (default 228x38, flared into the screen edge), hover to open, springs between states.
  Left ring = focus timer while one runs, otherwise battery level (bolt when charging).
- Pages (wheel/arrow keys): Music (art, progress, transport, synced lyrics via LRCLIB), Timer (presets,
  steppers, shrinking ring), Stats (CPU/RAM/network), Shelf (drop files onto the pill; click to open),
  To-do (`@14:30` suffix = reminder, saved to `todos.json`), Calendar (month grid), Weather (Open-Meteo,
  search a city). Settings is a popup under the pill (gear button): toggles, size, eye colour/style/delay,
  location, launcher drives/folders, update check. 
- Quick launcher / command palette beneath the pill: apps, Visus commands, arithmetic, URLs, web search,
  "Take screenshot", "Check for updates". Open via the magnifier in the open pill, **Alt+Space** or **Ctrl+Alt+Space** (Windows),
  or on Linux bind a key to `visus-app --toggle-palette`.
- Alert peeks: charger, Caps Lock, volume, keyboard language and Wi-Fi/online changes (Windows), timer done,
  task due.
- Idle eye: blinks, glances, follows the cursor (Windows; Hyprland), dozes off. Styles: Classic (colour
  configurable) and Sauron (burning lens, slit pupil).
- Auto-update: Settings -> "Check for updates" reads GitHub Releases; "Install" (Windows) downloads the zip,
  verifies SHA-256, swaps the exe and relaunches. Manual only.
- Settings file `settings.json` (`%APPDATA%\visus` / `~/.config/visus`), plus `todos.json`, `shelf.json`.
- Dev tooling: `visus-app --snapshot <music|lyrics|timer|stats|shelf|todo|calendar|weather|settings|palette> out.png`.
- CI builds and tests on Windows and Linux; tags `v*` publish a Windows zip (see `release.yml`).

## Known gaps / caveats

- Windows code compiles in CI but has had limited hands-on testing (click-through, tray, drag-and-drop, launcher
  focus, updater, network/language alerts, icon extraction).
- Post-v1 ideas: Bluetooth headphone peek, brightness HUD, notifications center, screen recording, calendar
  events, dragging files back out of the shelf, Sauron searchlight sweep, plugin system (WASM), macOS port,
  Store/UWP apps in the launcher, lyrics click-to-seek.
- Wayland host has no drag-and-drop (shelf drops work on Windows/X11). Wi-Fi/language alerts are Windows only.
- Single instance enforced (Windows mutex, Linux socket); panics write `crash.log` next to `settings.json`.

## Roadmap (after v1.0)

1. Fix whatever real-world Windows testing finds (settings popup, drop, launcher focus, updater, alerts still lightly tested).
   v1.0.1: Ctrl+Alt+Space hotkey, weather page has no location row (set city in settings), minimal notch hides content, equalizer glyph removed.
   v1.0.2: music visualizer (closed pill widens and animates only while music plays), more compact pill (size no longer read from settings.json), snappier launcher with a genie-style close, smarter lyrics matching (lyrics come from LRCLIB; tracks missing there, e.g. many YouTube uploads, have none).
   v1.0.3: launcher closes with a genie/black-hole warp (panel rendered offscreen, sliced per row, narrowed and pulled into the pill; `--snapshot genie <frames>` renders a frame); update errors show the full message.
   Ideas worth building next: clipboard history, quick-note capture, world clocks, volume/brightness HUD with sliders, per-app notification peeks, system tray quick toggles (Wi-Fi/Bluetooth/DND), pomodoro stats, calendar events (ICS/Google), file shelf drag-out.
   Still to do: real audio-driven visualizer (WASAPI loopback / PipeWire), lyrics fallback source, user-configurable hotkey, Linux global hotkey without compositor binding, Wayland drag-and-drop.
2. Bluetooth/brightness HUDs, notifications center, calendar events, shelf drag-out.
3. Plugin host behind a data-driven page registry; more eye styles; macOS port; installer/Store packaging.

## Dev notes

- Build/run: `cargo run -p visus-app`; tests: `cargo test`. Force the plain-window host on Linux with
  `VISUS_BACKEND=winit`.
- Release profile is size/speed tuned (LTO, strip, panic=abort).
- No Windows target is installed on the Linux dev machine; use the CI Windows job
  (`gh run list`, `gh run view <id> --log-failed`) to verify Windows changes.
- Windows API signatures are easiest to confirm by grepping the `windows-0.62` crate source under
  `~/.cargo/registry/src` (e.g. async ops use `.join()`, GUIDs may live in a different feature module).
- Visual references are the user's Nusa screenshots; other pages were designed by intuition to match.
