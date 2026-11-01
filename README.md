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
- **Idle eye** that blinks, looks around and follows your cursor. Classic or Sauron style, any colour.
- **Settings popup** (gear in the open pill): size, eye colour/style/delay, location, drives and folders the launcher
  searches, start at login, and a manual **update check**.

## Install

Download from [Releases](https://github.com/DevDad-Main/Visus/releases):

- Windows: run `Visus-Setup.exe` (per-user install, no admin needed; optional start at login and desktop shortcut; uninstall from Settings > Apps). Or unzip `Visus-windows-x64.zip` and run `Visus.exe`. Unsigned: SmartScreen > More info > Run anyway.
- Linux (Wayland, wlr-layer-shell compositors such as Hyprland/Sway): extract `Visus-linux-x64.tar.gz`, run `visus/visus`.
  Hyprland launcher key: `bind = SUPER, space, exec, /path/to/visus --toggle-palette`.

Launcher tips: `note <text>` saves a note, `cal <ics link>` sets a calendar, `fm <command>` sets your file manager (e.g. `fm thunar {path}`, `fm "C:\\Tools\\Explorer++\\Explorer++.exe" {path}`, `fm foot -e yazi {path}`; `fm default` resets), and typing part of a window title switches to it.

Settings, to-dos and the shelf live in `%APPDATA%\visus` (Windows) or `~/.config/visus` (Linux).
Visus makes network requests only for lyrics (LRCLIB), weather and city search (Open-Meteo) when you use them,
and for update checks when you click them. No accounts, no analytics.

## Build

```
cargo run -p visus-app            # run
cargo test                        # tests
visus-app --snapshot settings out.png   # render a page headlessly
```
