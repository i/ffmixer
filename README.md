# totalnix

A TotalMix FX-style mixer for the RME Fireface 800 on Linux, built on
`ffado-dbus-server`. See `docs/totalmix-fx-reference.md` for the target
design and the hardware/FFADO findings.

## Layout

| Path | Purpose |
|---|---|
| `apps/totalnix` | Tauri v2 app: Svelte/TypeScript UI in `src/`, Rust shell in `src-tauri/`. |
| `crates/mixer-core` | The TotalMix submix model: sends, pan/balance law, mute, solo, stereo pairs. Renders hardware crosspoints, imports them back, and plans safe (ramped) writes. No I/O. |
| `crates/ffado-dbus` | Client for `ffado-dbus-server`. `FfadoReader` calls getters only (`READ_METHODS`); `writer::FfadoWriter` can set fader-matrix gains and the typed device `settings::Setting`s, and nothing else. |
| `crates/fw-meter` | Read-only Fireface 800 level meters via `/dev/fw*`. Sends block reads only. |
| `crates/ffado-probe` | Read-only: prints controls, non-silent mixer routes and a few seconds of meters. |

**The app starts read-only every time.** It adopts the mix FFADO
reports, shows live hardware meters, and edits an in-memory copy; the
Control Strip shows how many crosspoints differ from the unit. **Go
live** re-reads the unit and from then on sends every edit, until
**Stop sending** or an error. Edits made while read-only can be sent or
discarded (**Reload from unit**). Without a unit it starts in demo mode
with simulated meters.

What going live can and can't touch:

- The three fader matrices (input, playback and output gains), and the
  device settings in `settings::Setting` (input source, levels, 48V,
  instrument options, digital formats, clock source). Flash, sample
  rate, bandwidth limit and FFADO's own flags are unreachable: targets
  are enums, not paths.
- Device settings are applied at once, only while live. 48V on, level
  changes, Drive on, Limiter off and clock source changes ask for
  confirmation first (the backend asks again). The 48V prompt alone can
  be switched off with a checkbox under Phantom Power (off by default,
  remembered across restarts). Each write is read back
  and checked; a mismatch stops sending.
- Every gain is checked: whole number, at most +6 dB, crosspoint in
  range. Increases ramp at 3 dB per 10 ms; decreases are immediate.
- Going live is refused if FFADO has mute/invert flags set, since FFADO
  would apply them to what we send.
- Writes go to the running server's unique bus name, so a crashed
  server is never silently restarted (a new server reloads the flash
  mix). The app checks once a second and stops sending if it changed.
- Nothing is stored to flash: the unit loses the mix on power-off, and
  restarting `ffado-dbus-server` reloads the flash mix.

## What the app does so far

- Three rows (hardware inputs, software playback, hardware outputs) and
  an optional 2 Row view.
- Submix view: click anywhere on an output strip to edit what is sent
  to it.
- Per strip: pan/balance knob, mute, solo (current submix only), meter
  with peak hold and over indicator, fader, numeric readouts, and a
  Settings panel: Stereo, Width, MS Proc, Phase L/R (Phase for mono).
- Output strips: volume, balance, mute, and a Settings panel with
  Phase L/R. Mono outputs, Talkback, No Trim and Loopback are shown but
  disabled until their features exist (Loopback: see the reference doc).
- Double-click a strip's name (or press F2 on it) to rename it. Names are
  display only and saved to `$XDG_CONFIG_HOME/totalnix/names.json`; an
  empty name restores the original.
- Control Strip: mute/solo masters (clear, then restore), RMS readouts.
- Device Settings (Control Strip): the Fireface Settings dialog's analog
  and digital options and clock source, plus live clock and input lock
  status. Store in Flash and Read Flash are shown but disabled; see the
  reference doc.
- Single clicks never change a level. Faders move by a relative drag
  (no jump to the pointer; Shift for fine), wheel or arrow keys.
  Double-click (or Home) sets 0 dB, except on the AN 1/2 output, which
  feeds the main monitors.
- Knob gestures follow the TotalMix manual: drag, wheel, Shift for
  fine, double-click or Ctrl+click to centre, Shift/Shift+Ctrl+click
  for hard left/right.

## Setup

See **[docs/setup.md](docs/setup.md)** for installation, the FireWire
permissions, a first-run checklist and troubleshooting. In short:

- A Fireface 800 on FireWire, and FFADO 2.4.x (`ffado-dbus-server`,
  started on demand by D-Bus). **The mixer and Device Settings need
  that server; without it the app starts in demo mode.** The meters
  read the unit directly and do not.
- Read/write access to `/dev/fw*` (the `audio` group plus a udev rule).
- Node.js 22+, Rust 1.85+, and WebKitGTK 4.1 development files. The
  Tauri CLI comes from `npm install`; no `cargo-tauri` needed.

The app sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` unless already set:
WebKitGTK's DMA-BUF renderer crashed on Wayland here (NVIDIA).

## Build and run

```bash
cd apps/totalnix
npm install
npm run check                     # svelte-check + tsc
npm run tauri dev                 # run the app
npm run tauri -- build --no-bundle   # release binary, no installer

cd ../..
cargo test --workspace
cargo test -p totalnix -- --ignored   # go-live checks against the running server (reads only)
cargo run -p ffado-probe -- 5     # 5 s of meters; 0 skips them
```
