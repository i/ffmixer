# ffmixer

A mixer for the RME Fireface 800 on Linux, with a workflow inspired by
RME's TotalMix FX, built on FFADO's `ffado-dbus-server`. RME does not
offer TotalMix for Linux; this fills that gap. See
`docs/totalmix-fx-reference.md` for the target design and the
hardware/FFADO findings, and `docs/setup.md` to get started.

> **Warning: use at your own risk.** ffmixer sends commands straight to
> audio hardware. Misused, or through a bug, it could:
>
> - **damage microphones and other equipment** (48 V phantom power can
>   destroy ribbon microphones and some unbalanced or active sources);
> - **damage speakers, headphones or your hearing** (a level, gain or
>   routing change can produce a sudden, very loud signal);
> - leave the unit in an unexpected state, interrupt audio, or change
>   settings you did not mean to change.
>
> It has been tested on **one Fireface 800 on one Fedora machine**. Its
> safeguards (confirmations, gain limits and ramping, read-back checks)
> reduce the risk but do not remove it. Before going live: turn monitors
> and headphones down, and disconnect anything that must not receive 48 V
> or a loud signal.
>
> The software is provided **as is, without warranty of any kind**. The
> authors and contributors accept no liability for damage to hardware,
> equipment, hearing, data or anything else; see sections 15 and 16 of
> the [license](LICENSE).
>
> **Not affiliated with or endorsed by RME.** RME, Fireface and TotalMix
> are trademarks of RME Audio / Audio AG. They are used here only to say
> which hardware this works with and what it is modelled on.

## Screenshots

![ffmixer showing a Fireface 800: hardware inputs, software playback and hardware outputs, with the Control Strip on the right](docs/images/ffmixer.png)

The three rows are the hardware inputs, the software playback channels
and the hardware outputs. Click an output to edit what is sent to it
(its submix). The Control Strip on the right has the **Go live** switch,
mute and solo masters, and the view options.

<img src="docs/images/device-settings.png" alt="The Device Settings panel in the Control Strip" align="right" width="158">

**Device Settings** (Control Strip) is the Fireface Settings dialog:
input source, line levels, 48 V phantom power, instrument options,
digital formats, clock source, and live clock and input lock status.
Here it is read-only (shown as the unit holds them); the controls
become active when you go live, and risky changes ask for confirmation
first.

<br clear="right">

## Layout

| Path | Purpose |
|---|---|
| `apps/ffmixer` | Tauri v2 app: Svelte/TypeScript UI in `src/`, Rust shell in `src-tauri/`. |
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
  display only and saved to `$XDG_CONFIG_HOME/ffmixer/names.json`; an
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
cd apps/ffmixer
npm install
npm run check                     # svelte-check + tsc
npm run tauri dev                 # run the app
npm run tauri -- build --no-bundle   # release binary, no installer

cd ../..
cargo test --workspace
cargo test -p ffmixer -- --ignored   # go-live checks against the running server (reads only)
cargo run -p ffado-probe -- 5     # 5 s of meters; 0 skips them
```

## License

GPL-3.0-or-later; see [LICENSE](LICENSE).

## Credits

- [FFADO](https://ffado.org) (GPL): `ffado-dbus-server` does all the
  mixer and settings control. ffmixer talks to it over D-Bus and does not
  include or link its code. Its source was the reference for the value
  encodings recorded in the reference doc.
- [snd-firewire-ctl-services](https://github.com/alsa-project/snd-firewire-ctl-services)
  by Takashi Sakamoto (LGPL-3.0-or-later): documents the Fireface
  level-meter layout. `crates/fw-meter` is an independent implementation
  of that layout, not a copy of its code.
- RME's *Fireface 800 User's Guide*, which describes the behaviour the
  UI follows. The manual and screenshots are RME's and are not included
  in this repository.

