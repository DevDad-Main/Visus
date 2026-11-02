# Changelog

## 1.11.1 - 2026-11-02

- Implement Linux in-pill updates for user-writable x86_64 installations, with archive checksum
	verification, atomic executable replacement, rollback if restart launch fails, and restart handoff.
- Replace the misleading Linux "Opened the release page" failure with real installation and clear
	messages for missing assets or system-owned installations. Versions through v1.11.0 need one manual upgrade.

## 1.11.0 - 2026-11-02

### Added

- Modular sidebar settings with Appearance, Clock, Mascot, Behavior, Gaming, Integrations, and Updates sections.
- A reversible balloon-style transition that expands settings from the notch and collapses back on close.
- Independent closed-clock and open-header sizes, live previews, sliders with typed values, explicit style selectors, color swatches, and per-section reset.
- Compact game HUD with FPS, a frame-time sparkline, and milliseconds directly inside the pill.
- Linux FPS/frame-time data from active MangoHud logs, with safe placeholders when unavailable.

### Improved

- Battery-percentage updates on Windows, plus a persistent unplugged battery glyph.
- Compact idle-pill spacing and spring-driven expansion when the music visualizer is active.
- Side-centered page-navigation arrows with separate click targets.
- Game HUD bar alignment with value text.
- Customization tests covering all registered value controls, persistence, layout bounds, animation reversal, clipping, and update actions.
- Refreshed website screenshots and settings animation; release builds now run tests before packaging.

### Compatibility

- Existing settings files retain their values and receive defaults for new clock controls.
- Per-element font families, spacing, button dimensions, and visualizer geometry are not independently configurable.
- FPS/frame-time availability depends on the platform and data source. Windows runtime behavior is checked by CI builds/tests but still benefits from hands-on testing.