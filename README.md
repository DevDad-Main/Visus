# Visus

A tiny always-on-top pill at the top of your screen that shows what matters at a glance and gets
out of the way. Native Rust, no web engine, near-zero CPU when idle.

## Features

- **Music**: any player (Windows media sessions / Linux MPRIS), album art, transport controls, synced lyrics.
- **Focus timer**: presets and steppers; the pill ring counts down. **To-do** with `@14:30` reminders.
- **Stats** (CPU, memory, network), **Calendar**, **Weather** (search your city), **File shelf** (drop files on the pill).
- **Launcher**: Alt+Space or Ctrl+Alt+Space (Windows) or `visus --toggle-palette` (Linux, bind it in your compositor). Searches apps
  with icons, runs Visus commands, does maths, opens URLs, searches the web, takes screenshots.
- **Alerts**: charger, Caps Lock, volume, keyboard language, Wi-Fi, reminders.
- **Game HUD**: Pills, Bars, Card, or Compact. CPU/GPU load, temperatures, and RAM where supported;
  Card and Compact show FPS/frame time when a source is available. Linux reads active MangoHud logs.
- **Idle eye** that blinks, looks around and follows your cursor. Classic, Sauron, Cat, or Robot styles.
- **Expanding settings** (gear in the open pill): the notch inflates into sidebar sections for appearance, clock, mascot, behavior,
  gaming, integrations, and updates. Live previews, independent closed/header clock sizes, sliders and
  typed values, explicit style choices, and per-section reset. Close or Escape collapses it back into
  the pill. Changes save automatically.

## Customization

| Section | Available controls |
| --- | --- |
| Appearance | Pill scale (0.60-1.25x), battery ring, music visualizer, minimal pill, network units |
| Clock | Classic/Flip/LED, closed clock size (12-32 px), header clock size (16-30 px), 12/24-hour time |
| Mascot | Enable, four styles, idle delay (5-300 seconds), expressions, iris and eye color swatches |
| Behavior | Fullscreen hiding, quiet alerts, clipboard history, focus duration, start at login |
| Gaming | HUD on/off and Pills/Bars/Card/Compact layout selection |
| Integrations | Weather city, calendar URL/file, file-manager command, launcher folders |
| Updates | Automatic checks, manual check, install an available update |

Clock sizes can be changed without moving the pill-scale slider. Larger clocks grow the pill's minimum
bounds to avoid overlap. Style selectors and switches apply immediately; numeric sliders also accept
typed values. Settings are saved locally, with per-section reset and live closed/header/mascot/game previews.

Not every visual dimension is independently adjustable: font family, page text sizes, spacing, button
dimensions, and individual visualizer geometry remain fixed. Hardware readings, startup integration,
and update installation depend on the platform and permissions.

![Settings expanding from and collapsing into the notch](docs/assets/gifs/settings_morph.gif)

## Install

Download from [Releases](https://github.com/DevDad-Main/Visus/releases):

- Windows: run `Visus-Setup.exe` (per-user install, no admin needed; optional start at login and desktop shortcut; uninstall from the tray menu or Settings > Apps). Or unzip `Visus-windows-x64.zip` and run `Visus.exe`. If SmartScreen shows a notice, choose More info > Run anyway.
- Linux (Wayland, wlr-layer-shell compositors such as Hyprland/Sway): extract `Visus-linux-x64.tar.gz`, run `visus/visus`.
  Hyprland launcher key: `bind = SUPER, space, exec, /path/to/visus --toggle-palette`.

Launcher tips: `note <text>` saves a note, `cal <ics link>` sets a calendar, `fm <command>` sets your file manager (e.g. `fm thunar {path}`, `fm "C:\\Tools\\Explorer++\\Explorer++.exe" {path}`, `fm foot -e yazi {path}`; `fm default` resets), and typing part of a window title switches to it.

Settings, to-dos and the shelf live in `%APPDATA%\visus` (Windows) or `~/.config/visus` (Linux).
Linux in-pill updates verify the release archive's SHA-256, replace the executable in a user-writable
installation, and restart Visus. System-owned installations must be updated with their package manager.
Versions through v1.11.0 only open the release page on Linux and require a manual upgrade before this
update path is available. The Linux updater requires x86_64 and `tar`.
Visus makes network requests for lyrics (LRCLIB), missing cover art, weather and city search (Open-Meteo),
calendar URLs you configure, and update checks/downloads. Automatic update checks can be disabled in
Settings. No accounts, no analytics.

## Build

```
cargo run -p visus-app            # run
cargo test                        # tests
visus-app --snapshot settings out.png   # render a page headlessly
visus-app --settings-preview clock.png Clock
visus-app --settings-preview header.png Clock:header
visus-app --settings-morph-preview /tmp/settings-frame
```

Settings text fields accept Enter to commit and Escape to cancel. Launcher folders are separated by
semicolons; an empty optional field restores its default behavior.

### Extending Settings

Register sections and fields in `crates/visus-app/src/settings_ui.rs` (`SECTIONS`). The registry owns
sidebar navigation and row layout; switches, numeric sliders with typed values, style choices, color
swatches, and text fields share rendering and input handling. Add persistent fields and defaults to
`visus-core::Settings`. Registry tests verify that controls bind to serialized settings keys.

`settings_surface.rs` connects the modular UI directly to the notch's input and action handling.
Settings expands from the pill's spring-animated shape and reverses on close, sharing the same renderer
and controls on Windows/X11 and Linux layer-shell. No extra window or companion process is opened.

Release builds are code-signed when `WINDOWS_CERT_PFX` (a base64-encoded PFX) and
`WINDOWS_CERT_PASSWORD` are set as repository secrets. Both `Visus.exe` and `Visus-Setup.exe`
(including its uninstaller) are signed; without the secrets the workflow still produces unsigned
artifacts.
