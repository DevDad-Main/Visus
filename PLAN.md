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
- Pages (wheel/arrow keys): Music (title/artist/progress/transport, lyrics toggle), Timer (Pomodoro),
  Stats (CPU, memory, network). Shelf and To-do are "Coming soon".
- Synced lyrics from LRCLIB shown in a split pane (current line highlighted).
- Quick launcher / command palette: a panel that drops down beneath the pill. Searches installed apps
  (Start Menu shortcuts / .desktop files), Visus commands, arithmetic (`12*7+3`), URLs, and falls back to a
  web search. Open it with the search button in the open pill, **Alt+Space** (Windows), or on Linux bind a
  key to `visus-app --toggle-palette` (Hyprland: `bind = SUPER, space, exec, visus-app --toggle-palette`).
- Album art from SMTC thumbnails (Windows) and MPRIS `artUrl` (Linux); gradient placeholder otherwise.
- Alert peeks: charger plugged/unplugged, Caps Lock, volume (Windows), timer done.
- Idle eye: almond eye with iris, blinks (irregular, sometimes double), glances around, follows the
  cursor (Windows via `GetCursorPos`, Hyprland via IPC), dozes off with "zz".
- Settings in `settings.json` (`%APPDATA%\visus` / `~/.config/visus`): `scale`, `closed_width`,
  `closed_height`, `minimal`, `mascot`, `idle_secs`, `hide_in_fullscreen`, `eye_iris`, `eye_sclera`
  (hex colours). Windows tray menu toggles the common ones and has size/eye-colour presets.
- Dev tooling: `visus-app --snapshot <music|lyrics|timer|stats|shelf|todo> out.png` renders the open pill
  headlessly using live OS state (handy for visual checks without hover automation).
- CI builds and tests on Windows and Linux; tags `v*` publish a Windows zip (see `release.yml`).

## Known gaps / caveats

- Windows code is verified to compile in CI but has had little hands-on testing: check click-through,
  transparency, tray, media controls, charger and volume HUDs on a real machine.
- Wi-Fi and Bluetooth peeks missing. No brightness/language HUDs.
- Lyrics: no click-to-seek, no disk cache. Position is extrapolated between player updates.
- No settings window yet (tray menu on Windows, hand-edit `settings.json` elsewhere). No tray on Linux.
- Linux layer-shell uses integer scaling only; the eye only tracks the cursor under Hyprland.
- Shelf (drag in/out) and To-do (needs keyboard input in a non-activating window) are not built.
- Hyprland cursor warping via `hyprctl` does not reliably trigger hover, so test open states with
  `--snapshot`.

## Roadmap

1. Windows hands-on fixes from real-machine testing.
2. Keyboard input is in (Windows drops `WS_EX_NOACTIVATE` while typing; Wayland grabs the keyboard
   exclusively while the launcher is open) -> To-do with reminders can reuse it; the quick
   launcher is done; To-do and File shelf (drag & drop) still to do. Launcher follow-ups: Store/UWP apps
   on Windows, app icons, clipboard history, configurable hotkey, plugin-provided results.
3. Settings window (sizes, colours, pages, eye style) replacing hand-editing, with an **auto-updater**:
   "Check for updates" queries the GitHub Releases API (`/repos/DevDad-Main/Visus/releases/latest`),
   compares semver with the running version, downloads `Visus-windows-x64.zip`, verifies the published
   SHA-256 (`Visus.exe.sha256`), swaps the exe (rename running exe, place new one, relaunch) and
   offers rollback. Prereleases only when the user opts in. Network use stays opt-in and manual.
4. Wi-Fi and Bluetooth peeks, brightness and keyboard-language HUDs.
5. Eye styles: "Sauron" eye with fiery glow and searchlight sweep; selectable in settings.
6. Visus extras: calendar, weather, notifications center, launcher, capture shortcuts.
7. Data-driven page registry, then a WASM plugin host.
8. macOS port; installer / Store packaging.

## Dev notes

- Build/run: `cargo run -p visus-app`; tests: `cargo test`. Force the plain-window host on Linux with
  `VISUS_BACKEND=winit`.
- Release profile is size/speed tuned (LTO, strip, panic=abort).
- No Windows target is installed on the Linux dev machine; use the CI Windows job
  (`gh run list`, `gh run view <id> --log-failed`) to verify Windows changes.
- Windows API signatures are easiest to confirm by grepping the `windows-0.62` crate source under
  `~/.cargo/registry/src` (e.g. async ops use `.join()`, GUIDs may live in a different feature module).
- Visual references are the user's Nusa screenshots; other pages were designed by intuition to match.
