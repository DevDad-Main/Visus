# Changelog

## 1.13.0 - 2026-11-02

- Add Windows DWM live thumbnails to open-window cards, preserving aspect ratio and title fallback. Previews are removed when the page is hidden. Protected/minimized windows may not provide a usable thumbnail; Linux retains title cards.
- Route Windows-tab activation through the launcher's focus outcome and strengthen Windows foreground activation.
- Add a persistent settings footer with running version and Visus's own CPU percentage and resident RAM in MB.
- Use GitHub's latest stable release endpoint and show explicit Checking/Up to date/Available/Installing/Failed update feedback.
- Reduce HUD bars, constrain wide HUD rows to their host window, and center paired Card GPU readings.
- Add optional Windows PresentMon CSV ingestion for FPS/frame-time graphs. Configure a live capture path in Gaming settings or VISUS_PRESENTMON_CSV; Visus does not bundle or launch capture software. Missing/stale feeds show an unavailable-source hint.
- Validate CSV parsing, stream selection, stale/incomplete files, window activation routing, HUD geometry, update fetching, and resource-footer behavior.

Windows thumbnails, focus behavior, and PresentMon capture inside cloud rigs still need hands-on testing.
GPU telemetry works independently of FPS. Provider restrictions may prevent presentation capture.

## 1.12.0 - 2026-11-02

### Added

- Attached, Floating, and Soft pill styles with matching previews and input regions.
- Independent closed-pill and open-page scale, battery ring diameter, automatic clock/ring height matching, ring-clock gap, and outer padding.
- Optional dim clock, accent modes (Media/Mascot/Fixed), animation intensity, reduced motion, hover-open/close delays, and per-category alert switches.
- Scrollbars, clickable More below cues, contextual setting hints, automatic preview switching, and a top-right close button with hover feedback.
- Mascot reaction guide, cat breathing/slow blinks, robot reply lights, and a fiery Sauron iris with molten trails and framing arches. Effects remain silent.
- Windows GPU telemetry fallbacks adapted from SalsaStats: NVAPI, NVIDIA-SMI, and WMI/CIM counters. SalsaStats itself is unchanged.
- Optional dedicated VRAM in Pills/Bars/Card HUDs. GPU load no longer depends on temperature availability, and collection failures are retried.
- Local rebuild-on-save tooling with an optimized incremental development profile.

### Fixed

- Large clocks no longer dim accidentally; hidden battery rings reclaim their slot and center the clock.
- Idle pill width follows measured clock content rather than leaving unused visualizer space.
- Settings now uses the black pill surface and its outline; its cached renderer avoids repeated expensive rasterization.
- Settings previews use live battery status and media-derived accents.

### Testing And Limitations

- Linux regression tests cover geometry, hit testing, persistence, mascot clipping, telemetry parsing, command deadlines, and GPU/VRAM without temperatures.
- Windows build/tests run in CI. GeForce NOW/SalsaNow provider-specific telemetry still needs a real session test.
- Visus must run inside the cloud rig to read its GPU. A local streaming client cannot expose remote GPU/VRAM counters to Visus.
- Driver/provider restrictions may leave individual readings unavailable; no synthetic values are substituted. Windows FPS remains unavailable without a frame-time source.
- Linux builds through v1.11.0 require a one-time manual upgrade before in-pill updating is available.

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