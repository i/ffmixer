# totalnix

A TotalMix FX-style mixer for the RME Fireface 800 on Linux, built on
`ffado-dbus-server`. See `docs/totalmix-fx-reference.md` for the target
design and the hardware/FFADO findings.

## Layout

| Path | Purpose |
|---|---|
| `apps/totalnix` | Tauri v2 app: Svelte/TypeScript UI in `src/`, Rust shell in `src-tauri/`. |
| `crates/mixer-core` | The TotalMix submix model: sends, pan/balance law, mute, solo, stereo pairs. Renders hardware crosspoints and imports them back. No I/O. |
| `crates/ffado-dbus` | Read-only client for `ffado-dbus-server`. Only getter methods are allowed (`READ_METHODS`). |
| `crates/fw-meter` | Read-only Fireface 800 level meters via `/dev/fw*`. Sends block reads only. |
| `crates/ffado-probe` | Prints controls, non-silent mixer routes and a few seconds of meters. |

**Nothing in the workspace can write to the device yet.** The app adopts
the mix FFADO reports, shows live hardware meters, and edits an
in-memory copy; the Control Strip shows how many crosspoints differ from
the unit. Without a unit it starts in demo mode with simulated meters.

## What the app does so far

- Three rows (hardware inputs, software playback, hardware outputs) and
  an optional 2 Row view.
- Submix view: click an output name to edit what is sent to it.
- Per strip: pan/balance knob, mute, solo (current submix only), meter
  with peak hold and over indicator, fader, numeric readouts, and a
  settings panel with mono/stereo.
- Output strips: volume, balance, mute.
- Control Strip: mute/solo masters (clear, then restore), RMS readouts.
- Fader and knob gestures follow the TotalMix manual: drag, wheel, Shift
  for fine, double-click or Ctrl+click for 0 dB / −∞ (faders) or centre
  (knobs), Shift/Shift+Ctrl+click for hard left/right.

## Requirements

- `ffado-dbus-server` (FFADO 2.4.x), started on demand by D-Bus.
- Read/write access to `/dev/fw*`: be in the `audio` group, plus a udev
  rule such as
  `SUBSYSTEM=="firewire", GROUP="audio", MODE="0660"`.
- Node.js 22+, `cargo-tauri` 2.x, and WebKitGTK 4.1 development files.

The app sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` unless already set:
WebKitGTK's DMA-BUF renderer crashed on Wayland here (NVIDIA).

## Run

```bash
cargo test --workspace
cargo run -p ffado-probe -- 5    # 5 s of meters; 0 skips them

cd apps/totalnix
npm install
npm run check                     # svelte-check + tsc
cargo tauri dev                   # run the app
```
