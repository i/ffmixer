# TotalMix FX reference (Fireface 800)

What we are recreating, page by page, and how each piece maps onto the
Fireface 800 and `ffado-dbus-server`.

## Sources

| Source | Local copy |
|---|---|
| RME, *User's Guide Fireface 800*, ch. 7 (Settings dialog) and ch. 25–28 (TotalMix FX) — <https://www.archiv.rme-audio.de/download/fface800_e.pdf> | `research/fface800_e.pdf`, text in `research/fface800_e.txt` |
| Manual pages rendered as images | `research/pages/p<page>-<page>.png` |
| Product screenshots — <https://rme-audio.de/totalmix-fx.html> | `research/web/*.jpg` |

RME shipped TotalMix FX for the Fireface 800 in 2014 ("TotalMix FX 4
All"). On this unit everything DSP-related is absent: no EQ, dynamics,
reverb/echo, FX sends, Post-FX meters, Room EQ or DURec. The manual marks
each of these "Not available for the Fireface 800".

## Core model

The idea everything else rests on (manual §25.3):

- **There is no channel fader.** Every source (hardware input or software
  playback) has one send to *each* hardware output pair. On the FF800
  that is 14 stereo submixes, the equivalent of 28 aux sends.
- **Submix view (default):** clicking an output in row 3 selects it. Every
  fader in rows 1–2 then shows and edits that source's send *to the
  selected output*.
- **Free view:** each source strip picks its own routing target in the
  routing field under the fader, so several submixes can be edited at once.
- The matrix view and the mixer view are two views of the same gains, and
  always stay in sync.

The hardware only stores per-crosspoint gains (plus mutes and inverts).
Pan, stereo linking, width, M/S, trim, solo, cue, dim, talkback and the
other features are TotalMix logic that turns UI state into those gains.
That logic lives in our `mixer-core`.

| Concept | Rule (from the manual) |
|---|---|
| Pan | Stereo sources only; −3 dB at centre |
| Stereo/mono | Per input/playback channel; hardware outputs are always stereo |
| Width | 1.00 = stereo, 0.00 = mono, −1.00 = swapped; affects all routings of the channel |
| MS Proc | Encodes or decodes M/S; affects all routings |
| Phase L/R | Per side; affects all routings |
| Mute | Pre-fader; mutes *all* active routings of the channel |
| Solo | Solo-in-place, post-fader, *only within the current submix* |
| Trim (T) | One fader moves all of the channel's active sends together. The knob shows the highest send; orange triangles mark the others and a white triangle marks the current one. Can be global (View Options → Trim). |
| No Trim (output) | Excludes that output's routing from Trim |
| Cue (outputs) | Sends that output to Main (or a chosen Phones output) for listening |
| Meters | Inputs/playbacks are pre-fader, outputs post-fader. Peak shown as a yellow line, RMS as a green bar, over in red. Numeric readout updates twice a second. |

## Window layout

```
┌──────────────────────────────────────────────────────────┬───────────────┐
│ Row 1  HARDWARE INPUTS     (strips, horizontal scroll)   │ Control strip │
├──────────────────────────────────────────────────────────┤  device ▾     │
│ Row 2  SOFTWARE PLAYBACK                                 │  undo / redo  │
├────────────────────────────────────────────┬─────────────┤  M  S  F      │
│ Row 3  HARDWARE OUTPUTS                    │ CONTROL ROOM│  View options │
│        (click = select submix)             │ Main/Phones │  Snapshots    │
│                                            │ Dim Recall… │  Groups       │
└────────────────────────────────────────────┴─────────────┴  Layout pres. ┘
```

- **2 Row** option: inputs and playbacks side by side in one row.
- Rows scroll independently. Each scrollbar has location markers:
  ⏮, 1, 2, ⏭. Right-click 1 or 2 to store the current position.
- Screenshots: `research/web/TotalMix-Combi.jpg` (full window),
  `research/web/control-room.jpg`, manual p54 (row diagram).

## Surfaces

### 1. Channel strip (manual p55–57)

Top to bottom:

1. Name field: click selects, double-click renames.
2. Pan knob with a numeric field.
3. M and S buttons (outputs have M and CUE).
4. Numeric level readout (peak or RMS).
5. Meter, fader and T button.
6. Wrench button, which opens the settings panel to the right of the strip.
7. Gain readout.
8. Narrow/wide toggle (◀).
9. Routing field: shows the current target; click for the target list.
   An arrow marks active routings and **bold** marks the current one.

Settings panel:

- **Inputs and playbacks:** Stereo, Width, MS Proc, Phase L, Phase R.
- **Outputs:** Stereo, Phase L/R, Talkback, No Trim, Loopback.
  Loopback sends that output to the recording software in place of that
  channel's hardware input.

How totalnix implements these on the FF800 (all in `mixer-core`):

- **Phase L/R (sources):** negative crosspoint values; the hardware
  inverts per crosspoint.
- **Width and MS Proc:** a 2×2 mix of the pair's left and right channels
  into each output side. M/S comes first and is scaled by 1/√2, so it
  is its own inverse (encoder and decoder); width then blends the
  sides (1 keeps, 0 sums to mono, −1 swaps).
- **Phase L/R (outputs):** no Linux driver exposes an output inverter
  (FFADO has `InputInverts`/`PlaybackInverts` only), and whether the
  output volume register accepts negative values is untested. totalnix
  inverts every crosspoint feeding that output channel instead, which
  sounds identical.
- **Not yet:** mono outputs, Talkback and No Trim (need Control Room and
  Trim). **Loopback** looks like a hardware switch: FFADO's
  `RME_FF_OUTPUT_REC_MASK` (`0x801c0080`, write-only, one flag per
  output) is commented "record outputs", but FFADO only sets all flags
  together and does not expose it on D-Bus. Needs hardware writes and a
  test.
- Import does not reconstruct width, M/S or output phase; they come
  back as mono strips and per-source phase (same sound).

The FF800 has **no software gain or 48V on the strip**: mic gain is an
analog pot on the unit's front panel. 48V is handled under Device
settings (surface 9). We may put 48V/Inst toggles on strips 7–10 and 1
as a deliberate improvement on RME's design; that is our call.

Gestures:

| Gesture | Effect |
|---|---|
| Drag / mouse wheel | Move the fader |
| Shift + drag / wheel | Fine adjustment (on knobs, Shift speeds up instead) |
| Double-click, or Ctrl+click on fader | Jump to 0 dB, then −∞ |
| Ctrl+click / double-click on knob | Centre it |
| Shift+click pan | Hard left; Shift+Ctrl+click = hard right |
| Shift+click fader | Add to the temporary fader group (shown yellow; F clears it) |
| Double-click a value | Enter it with the keyboard |
| Drag on a value field | Up raises the value, down lowers it |
| Shift+click narrow or settings button | Apply to every channel to the right |
| Right-click any channel | Open Channel Layout with that channel preselected |

**totalnix departs from this on purpose: no click may change a level.**
A single misclick that jumps a route from −∞ to 0 dB can damage hearing
or equipment. Faders move only by a relative drag (after a few pixels
of travel), the wheel, or the keyboard; clicking the track or thumb,
Ctrl+click and double-click do nothing. Any future control that sets a
gain (matrix cells, value fields) follows the same rule. Clicking
anywhere on an output strip selects its submix, which is safe because
selection changes only what the UI edits.

### 2. Hardware output context menu (manual p69)

Right-click an output: **Clear Submix**, **Copy Submix**, **Paste Submix**.

### 3. Control Room (manual p58)

- **Assign** menu: Main Out, Main Out B, Phones 1–4, Cue/PFL target.
  Assigned outputs move out of row 3 into this section and are renamed.
- Buttons: **Dim**, **Recall**, **Speaker B** (with Link to gang the
  Main/B faders), **Mono**, **Talkback**, **Ext. Input**.
- Talkback dims the Phones and routes the talkback mic to them.

### 4. Control strip (manual p59–63)

- Device selector, Undo/Redo (unlimited; covers mixer state only, not
  layout).
- Global **M / S / F** masters: light up when any mute/solo/temporary
  group is active; click to toggle all of them or clear.
- **View Options:** Routing mode Submix | Free · Level meters RMS ·
  Show Trim | 2 Row | Names (original names). FX and Post FX are not
  applicable to the FF800.
- **Snapshots:** 8 named slots. A slot flashes once the mix changes
  after it was loaded. To store: click Store, then the slot. Alt+1…8
  loads a slot. Screenshot: `research/web/Snapshots.jpg`.
- **Groups:** 4 fader, 4 mute and 4 solo groups. Set them up with Edit:
  click a slot, select members, click Edit again. Mute and solo groups act
  only within the current submix.
- **Layout Presets:** 8 slots of hidden/visible channels; **All** shows
  everything. **Sub** shows only the channels that feed the selected submix.

### 5. Matrix view (manual p71, key X)

- Columns are the hardware outputs. Rows are the hardware inputs, then
  the playbacks, then an **Outputs** row for output volume.
- Cell colours: green `0.0` = unity, dark grey with a number = that gain
  in dB, blue = muted, red = phase inverted, plain dark grey = no route.
- Hovering highlights the row and column labels in orange. Ctrl+click
  toggles the route at 0 dB. Dragging a cell changes its gain. Labels stay
  visible while scrolling.
- **Mono Mode** switches whether edits affect a channel pair or a single
  channel.
- Screenshot: `research/web/matrix-view.jpg`.

### 6. Preferences (F3; manual p64)

- Meters: number of consecutive full-scale samples that count as an over
  (1–10), peak hold time (0.1–9.9 s), RMS +3 dB.
- Centre pan when switching between mono and stereo; disable double-click
  fader actions.
- Snapshots: an option not to load Main volume.
- Lock User Interface.

### 7. Settings dialog (manual p65)

- **Mixer page:** Talkback input and dim; Listenback input and dim;
  Main Out recall level and dim; External Input source and gain; PFL
  mode ("Live Mode, PFL replaces Solo").
- **MIDI page:** 4 remotes; Mackie Control options.
- **OSC page:** 4 remotes; ports; peak level sending.
- These are later-phase features.

### 8. Options menu / Reset Mix (manual p70)

- Straight playback with all routed to Main; straight playback; clear all
  submixes; reset output volumes (0 dB, with Main and Speaker B at
  −10 dB); reset names; total reset.
- Key commands.
- Workspaces (W opens quick select with 30 slots).

### 9. Device settings (the separate "Fireface Settings" app; manual p13–16)

On Windows/macOS this is a separate program. We fold it into our app as
its own page.

- **General:** buffer size and bandwidth (driver-side; not ours),
  Word clock single speed, ADAT2 as ADAT or SPDIF, SPDIF
  consumer/professional with emphasis, SPDIF input coax or ADAT2, TMS,
  sample rate, clock source (Internal, Word, ADAT1/2, SPDIF, TCO),
  input status (Lock / No Lock / Sync, per input).
- **Analog:** input select for channels 1, 7 and 8 (Front / Rear /
  Front+Rear), Line In and Line Out reference levels, **48V on Mic 7–10**,
  instrument Drive / Limiter / Speaker Emulation.
- **Store in Flash / Read Flash.** Saves the current state to the unit so
  it powers up in the same state and can run without a computer.
- Screenshots: manual p13 and p16.

## FF800 ↔ FFADO mapping

Known `ffado-dbus-server` paths (from the RME support in FFADO 2.4.9):

| Need | Control path |
|---|---|
| Input → output sends | `/Mixer/InputFaders` (matrix), `/Mixer/InputMutes`, `/Mixer/InputInverts` |
| Playback → output sends | `/Mixer/PlaybackFaders`, `/Mixer/PlaybackMutes`, `/Mixer/PlaybackInverts` |
| Output volume | `/Mixer/OutputFaders`, `/Mixer/OutputMutes` |
| 48V | `/Control/Phantom`. Bitmask: value bit *n*, plus enable bit *16+n* |
| Input select 1/7/8 | `/Control/Chan1_source`, `/Control/Chan7_source`, `/Control/Chan8_source` |
| Line In / Out / Phones reference level | `/Control/Input_level`, `/Control/Output_level`, `/Control/Phones_level` |
| Clock | `/Control/Clock_mode`, `/Control/sysclock_mode`, `/Control/sysclock_freq`, `/Generic/ClockSelect` |
| Store/read flash | `/Control/Flash_control` |

All paths sit under `/org/ffado/Control/DeviceManager/<GUID>/`. The
FF800 on this machine has GUID `000a35004aa780e6`. The level, clock and
flash paths were confirmed in the D-Bus tree; their value encodings are
not yet checked.

### Meters (not in FFADO)

FFADO 2.4.9's RME driver (`src/rme/`) has no metering code, so
`ffado-dbus-server` publishes no meter nodes. The unit exposes levels as
a block read at FireWire address `0x000080100000`. The layout was
reverse-engineered in
[snd-firewire-ctl-services](https://github.com/alsa-project/snd-firewire-ctl-services)
(`protocols/fireface/src/former.rs`):

- 8-byte entries per physical input, then 8-byte entries per physical
  output (pre-fader), then per physical output again (post-fader).
  Their meaning is not fully known.
- Then 4-byte little-endian entries for each physical input, stream
  (playback) input and physical output, in that order. Values are linear,
  from `0` to `0x7fffff00`; mask with `0x7fffff00`. Overload bits are not
  decoded.
- Total read length: 8 × (inputs + 2 × outputs) + 4 × (inputs + streams
  + outputs) bytes.

Implemented in `crates/fw-meter` (block reads only) and confirmed working
on this unit with `ffado-probe`: 28 inputs, 28 playbacks and 28 outputs
all report sensible levels.

- The first read after opening returns much higher peaks than the reads
  after it. The meters look like "peak since last read", but this hasn't
  been confirmed.
- It's not yet known whether these values are peak or RMS. Peak hold, RMS
  ballistics and over detection will be calculated in the app.

### Gain scale and readback

Gains are linear, and negative values mean phase inverted. There are
two scales:

| Layer | −∞ | 0 dB | +6 dB (max) |
|---|---|---|---|
| `ffado-dbus-server` (what we use) | `0` | `0x4000` | `0x8000` |
| Hardware register | `0` | `0x8000` | `0x10000` |

FFADO doubles D-Bus values before writing them
(`RmeSettingsMatrixCtrl::setValue` in `fireface_settings_ctrls.cpp`).
This was **verified on this unit** with a −20 dBFS, 1 kHz tone on
playback 1 (`AUX0` of the PipeWire Pro Audio sink), which metered at
−40.8 dB after the sink volume:

| Path | D-Bus gains | Expected | Measured |
|---|---|---|---|
| pb1 → Line out 1 | route 16384, output 1060 | −64.6 | −64.6 |
| pb2 → Line out 2 | route 16384, output 1011 | −65.0 | −65.0 |
| pb1 → Line out 7 / Mon 9 / SPDIF 1 | 16384, 16384 | −40.8 | −40.8 |

This also confirms that the 4-byte output meters are post-fader, and
that playback meters show the level arriving from the computer.

**The live mixer can't be read back; flash is the only copy on the unit.**
Neither FFADO nor snd-firewire-ctl-services reads the mixer registers;
both only write them. When `ffado-dbus-server` discovers the unit, it
reads the mix saved in flash and **writes every crosspoint and output
gain to the live mixer** (`init_hardware()` in `fireface_hw.cpp`). So
right after the server starts, `getValue` should match the hardware,
unless something else changes the mix later.

Flash stores a volume and a pan value per stereo pair. FFADO splits them
linearly (`fireface_flash.cpp`, `flash2faders`).

So the earlier "mismatch" was our mistake: we had applied the hardware
scale to D-Bus values.

Discrete values seen: `Input_level` 2 (+4 dBu), `Output_level` 1
(+4 dBu), `Phones_level` 0 (high gain), `Phantom` 0, `sysclock_freq`
48000.

**Caution:** starting `ffado-dbus-server` overwrites the live mix with
the one in flash. Anything changed since the last flash store (by
TotalMix on another machine, or by stand-alone MIDI control) is lost.

### Writing

`setValue(row, col, value)` on a fader matrix: `row` is the output
channel (always 0 for `OutputFaders`), `col` the source (or output)
channel, value on the D-Bus scale. FFADO doubles it, then applies its
own per-crosspoint flags (`Mixer/*Mutes` force 0, `Mixer/*Inverts` flip
the sign), then writes the mixer register. It returns −1 only for
argument errors; FireWire write failures are logged, not reported. Its
channel bounds check is `>` instead of `>=`, so callers must validate.

FFADO's `setMixerGain` comments warn that the hardware briefly plays
"much higher" when a gain goes from 0 to −1 or changes phase, and
avoids 0 on inverted routes. **Not reproduced on this FF800:** with a
−20 dBFS tone on playback 1 and Line out 1 sampled every 5 ms for 600
ms after each write, none of +g→−g, −g→+g, 0→−1, −1→0, 0→−g or −g→0
peaked above the steady level. Halving Line out 1's volume dropped its
meter by exactly 6.0 dB. totalnix therefore writes sign changes
directly.

## Where state lives

| State | Owner |
|---|---|
| Crosspoint gains, mutes, inverts, output volume, 48V, input select, levels, clock | Hardware, through FFADO |
| Pan, stereo pairs, width, M/S, trim, solo, cue, groups, Control Room assignments, talkback/dim/recall | `mixer-core`, which writes the resulting gains to the hardware |
| Names, snapshots, layout presets, undo history, workspaces, preferences | App files under `$XDG_CONFIG_HOME/totalnix/` |

## Build phases

1. **Core mixer:** three rows, submix view, faders, pan, mute/solo,
   meters, output selection, stereo/mono, phase, Device settings page.
2. **Control Room:** Assign, Main/Phones, Dim/Recall/Mono/Cue, Talkback.
3. **Productivity:** matrix view, undo/redo, snapshots, copy/paste/clear
   submix, Trim, temporary fader group, names, keyboard shortcuts.
4. **Later:** Free routing mode, groups, layout presets and Sub,
   workspaces, loopback, Reset Mix, OSC/MIDI remote.
