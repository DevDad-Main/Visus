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
   v1.0.4: Control Center page (volume slider with drag, Wi-Fi/Bluetooth/Mute/Quiet tiles; platform::controls worker thread: Windows endpoint volume + Radios API, Linux wpctl/nmcli/bluetoothctl), audio-reactive visualizer (Windows IAudioMeterInformation peak; Linux falls back to a plain animation), battery ring hidden when the machine reports no/unknown battery (cloud rigs), updater falls back to curl, quicker genie ending just below the pill, smaller Sauron slit that follows the cursor. Brightness slider not built (needs WMI/DDC; most desktops and cloud rigs have none). Windows parts compile-checked by CI only.
   v1.0.5: 9-bar smoothed visualizer tinted from the album art (glow, centre-weighted), pill tightens around the idle eye and grows on hover, Control Center redesign (icon tiles, thin slider with knob, now-playing), lyrics fall back to NetEase Music (unofficial public endpoints; Google scraping rejected as unreliable and against its terms). Lyrics order: LRCLIB (exact, search, cleaned YouTube titles) then NetEase.
   v1.0.6: Open Iconic cog for the settings button, pill opens on the Shelf page while a file is dragged over or dropped, smoothness pass (release opt-level 3: draws 3-4x faster, cached glyph bitmaps, frame cadence follows the monitor refresh rate up to 144 Hz; `VISUS_BENCH=1 visus-app --snapshot <page> out.png` prints average draw time). Wayland host still uses its own 16 ms pacing.
   v1.0.7: Alt+Space handled in the low-level keyboard hook (works even when another app owns the hotkey; Ctrl+Alt+Space still registered), genie closes into a point with a small swell at the start and a quicker spring.
   v1.0.8: the shelf closes 0.5 s after a file drag leaves the window without dropping.
   v1.1.0: Windows installer (Inno Setup, installer/visus.iss, built in the release workflow, per-user in %LOCALAPPDATA%\\Programs\\Visus), Control Center now has volume + brightness sliders (DDC/CI for external monitors, WMI for laptop panels, brightnessctl on Linux; hidden when unsupported) and Wi-Fi/Bluetooth/Mute/Quiet tiles, settings gained Quiet mode, 24-hour clock, Visualizer and Battery ring toggles, lyrics.ovh as a third (plain, evenly spread) lyrics source, widgets mock removed, bench code behind the `dev` feature, more tests.
   v1.1.0 also: the idle eye no longer falls asleep (closed-lid slit) after a few minutes; it keeps blinking until the pill is used again.
   v1.2.0: eye sleeps again (after 4x the eye delay with no mouse movement) with three floating z's, drowsy droop before sleeping, expressions every ~16 s (side-to-side scan, surprise, happy squint, beating heart on Classic), pupil pulses with the music, Sauron gets lid flames, a breathing red halo, rising embers and a hot-rimmed slit. Dev previews: `visus-app --snapshot eye <idle seconds> out.png` and `--snapshot sauron`.
   v1.3.0: effects (album-art tint on the pill that breathes with the music, squash-and-stretch while resizing, rim ripple on alerts, thin progress line along the closed pill for the timer or track); Notes page (`note <text>` in the launcher, click to copy, stored in notes.json); Clipboard page (history kept in memory only, Settings > Clipboard to turn off; click to copy back); window switcher in the launcher (type part of a window title; Windows EnumWindows, Hyprland via hyprctl); calendar feed (`cal <ics link or file>` in the launcher; simple DAILY/WEEKLY repeats, no exceptions/timezones beyond local; agenda in the Calendar page, "Meeting in N min" peek that opens the call link on click); click a lyric line to seek; Shift+wheel on the open pill changes the volume (winit host); eye skins Cat and Robot, eye moods setting, sleepier at night and wider awake in the morning.
   Not built: notification peek (Windows only exposes other apps' notifications to packaged/MSIX apps), swipe-to-skip, eye click Easter egg (hovering opens the pill before a click can reach the eye), reactions to alerts (the eye is hidden while alerts show).
   v1.4.0: cover-art fallback (iTunes Search by cleaned title/artist when the player sends none; feeds the tint and glow too), network speed in bytes or bits (tap the network block on the Stats page; Gbps/Mbps/Kbps or GB/s/MB/s/KB/s), download indicator (Downloads folder watcher for .crdownload/.part/.download files: arrow plus live speed in the closed pill and a "Download complete" peek), mascot faces in alerts (happy, sad, dizzy, music, party, alert expressions in the current skin colours, bouncing as the alert lands; off with the Eye moods toggle), robot glitch mood. Dev previews: `--snapshot alert <name>:<style>` (charging, unplugged, headphones, offline, timer, todo) and `--snapshot download`.
   v1.5.0: launcher switches to an already-running app (window title contains the app name) instead of starting a second copy; Windows page (open windows, click to switch); hover highlights on every button plus hand cursor (winit host), launcher rows follow the pointer; clicks that land while the pill is still opening are replayed; Notes/Clipboard empty states with a "New note" / "Turn on" button, copy icon and "Copied to clipboard" toast; Shelf redesign (file tiles, size and folder line, show-in-folder button, dashed drop zone that lights up while dragging); animated album-art backdrop (two slow drifting blobs); download chip in the open pill opens the Downloads folder (also the "Download complete" peek); custom file manager (`fm <command with {path}>` in the launcher, `fm default` resets; stored in settings.json as file_manager); controller connect/disconnect peek with a face (XInput on Windows, /dev/input/js* on Linux).
   v1.5.1: app icon (indigo-to-blue tile, black notch with the eye; generated by tools/make_icon.py into assets/visus.ico, embedded with winresource in build.rs, also the installer and tray icon), snappier media (cover art loads on a side thread so title/artist update instantly, session manager cached for play/pause/skip, optimistic play/pause, 200 ms progress refresh, lyric sources queried in parallel with a 5 s timeout, online cover lookup waits 1.2 s for the player's own art).
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
