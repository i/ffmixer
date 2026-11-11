# Setup guide

> **Use at your own risk.** ffmixer sends commands straight to audio
> hardware and can change 48 V phantom power, levels and the clock. A
> mistake or a bug could damage microphones or other equipment,
> speakers, headphones or your hearing. It has been tested on one
> Fireface 800 on one Fedora machine. It comes without warranty and the
> authors accept no liability (see [LICENSE](../LICENSE)). Turn monitors
> and headphones down and disconnect delicate equipment before you go
> live for the first time.

ffmixer is a mixer for **RME Fireface** interfaces on Linux. Only the
Fireface 800 is supported so far (see the README). It talks to
the unit through FFADO's `ffado-dbus-server` (mixer and settings) and
reads the level meters directly from the FireWire device. This guide
takes you from a fresh machine to a running app.

Tested on Fedora 44 (kernel 7.1, Wayland, NVIDIA) with the kernel
`snd_fireface` driver and PipeWire loaded. Other distributions should
work; package names below for them are from memory, so check yours.

## 1. What you need

- A supported Fireface (currently the 800) connected over FireWire. The computer needs a
  FireWire (OHCI) port; the kernel module is `firewire_ohci`. A
  Thunderbolt or ExpressCard adapter works if it shows up as one.
- Check the unit is seen: `ls /dev/fw*` lists `fw0` (the controller) and
  `fw1` (the unit), and `cat /sys/bus/firewire/devices/fw1/guid` prints
  its GUID.

## 2. Install the dependencies

| Need | Fedora (tested) | Debian / Ubuntu, Arch (check names) |
|---|---|---|
| FFADO 2.4.x with the D-Bus server | `sudo dnf install ffado` | `ffado-dbus-server`; `libffado` |
| WebKitGTK 4.1 (development files) | `sudo dnf install webkit2gtk4.1-devel` | `libwebkit2gtk-4.1-dev`; `webkit2gtk-4.1` |
| Rust 1.85 or newer (edition 2024) | `rustup`, see <https://rustup.rs> | same |
| Node.js 22 or newer | `sudo dnf install nodejs` or `nvm` | same |

Tauri may need a few more development packages (a C toolchain,
`openssl-devel`, `librsvg2-devel` and similar). Its list for each
distribution is at <https://v2.tauri.app/start/prerequisites/>.

You do **not** need to install `cargo-tauri`: the Tauri CLI comes with
the app's npm dependencies.

## 3. Let your user talk to the FireWire device

FFADO and the meter reader both open `/dev/fw*` for reading and
writing, which is root-only by default. Without access,
`ffado-dbus-server` quits with "No FireWire adapters found" and the app
starts in demo mode.

```bash
sudo usermod -aG audio "$USER"
echo 'SUBSYSTEM=="firewire", GROUP="audio", MODE="0660"' \
  | sudo tee /etc/udev/rules.d/60-firewire-audio.rules
sudo udevadm control --reload
sudo udevadm trigger --subsystem-match=firewire
```

Log out and back in (the group change needs a new session), then check:

```bash
id -nG | tr ' ' '\n' | grep -x audio     # prints: audio
ls -l /dev/fw0 /dev/fw1                  # both: crw-rw---- ... audio
```

Never run the app or FFADO as root to get around this.

## 4. Check that FFADO sees the unit

The app starts `ffado-dbus-server` itself through D-Bus when it needs
it. To check it works before using the app:

```bash
busctl --user tree org.ffado.Control | grep -c DeviceManager
```

A number above zero (about 58 for a Fireface 800) means FFADO found the
unit. Zero, or an error, means the server did not start. See
[Troubleshooting](#troubleshooting).

The kernel `snd_fireface` driver and PipeWire can stay loaded; they
coexist with FFADO here. FFADO is only used for the mixer and settings,
not for audio streaming.

## 5. Before the first run

> **FFADO writes the unit's saved (flash) mix to the live mixer every
> time its server starts.** Anything you changed since the last "store
> in flash" is replaced, for example mixes made in TotalMix on another
> computer or with stand-alone MIDI control.

- If you care about the current mix, save it to the unit first with RME's
  own software.
- Turn monitors and headphones down. The saved mix may be louder than
  you expect.

ffmixer itself starts **read-only** every time: it shows the unit's mix
and meters, and sends nothing until you press **Go live**.

## 6. Get the app

### Download a release

Release pages have a `ffmixer-<version>-linux-x86_64.tar.gz` and its
`.sha256`. Verify and unpack it:

```bash
sha256sum -c ffmixer-*-linux-x86_64.tar.gz.sha256
tar xzf ffmixer-*-linux-x86_64.tar.gz
./ffmixer-*/ffmixer
```

The binary is built on Ubuntu 22.04 and needs WebKitGTK 4.1 (the
runtime package, e.g. `libwebkit2gtk-4.1-0` or `webkit2gtk4.1`) and a
glibc at least as new as Ubuntu 22.04's. You still need FFADO and the
permissions from sections 2 to 5. Releases are pre-releases and tested
on one Fireface 800 only.

### Build from source

```bash
git clone https://github.com/i/ffmixer.git
cd ffmixer/apps/ffmixer
npm install
npm run tauri dev                 # development: hot reload
```

A release build (no installer package yet):

```bash
npm run tauri -- build --no-bundle
```

The binary is `target/release/ffmixer` in the repository, or under
`$CARGO_TARGET_DIR/release/` if you set that. It needs
`ffado-dbus-server` available on the session bus when it runs.

To check the code itself:

```bash
cargo test --workspace
cd apps/ffmixer && npm run check
```

## 7. First run

1. The Control Strip (right) shows a **Read-only** badge and "In sync
   with unit". If it says **Demo**, the app could not reach the unit or
   FFADO; the status line says why.
2. The meters move on their own, straight from the unit.
3. **Go live** re-reads the unit and from then on sends every change.
   **Stop sending** ends it. Edits made while read-only can be sent or
   discarded when you go live.
4. **Device Settings** (Control Strip) shows the unit's analog and
   digital options and clock. They can only be changed while live. Risky
   changes (48 V on, level changes, Drive, Limiter off, clock source)
   ask for confirmation first.

What is stored where:

| Data | Location |
|---|---|
| Channel names | `$XDG_CONFIG_HOME/ffmixer/names.json` (`~/.config/ffmixer/`) |
| "Don't ask before switching 48 V on" | the app's local storage |
| Mixer state | the unit only; **nothing is saved to its flash** |

The unit forgets what ffmixer sent when it is powered off, and
restarting `ffado-dbus-server` reloads the flash mix.

## Troubleshooting

| Symptom | Cause and fix |
|---|---|
| Status says "Demo mode: cannot reach ffado-dbus-server" or "reports no devices" | FFADO is not installed, did not start, or cannot see the unit. Run `ffado-dbus-server -v3` in a terminal and read the output; check section 3 first. |
| `journalctl --user -b -g ffado` shows "No FireWire adapters found" | No access to `/dev/fw*`. Redo section 3 and log in again. |
| `ls /dev/fw*` shows only `fw0` | The unit is off or not connected, or the cable or port is faulty. |
| Go live refused: "FFADO has N mute/invert flags set" | `ffado-mixer` set flags FFADO would apply to what we send. Clear them in `ffado-mixer`, or restart `ffado-dbus-server`. |
| "Stopped sending: ffado-dbus-server restarted…" | The server was restarted and reloaded the flash mix onto the unit. Use **Reload from unit**, then go live again. |
| "Stopped sending: …" with another message | A write failed or FFADO did not accept a setting. The unit may not be in the state shown: **Reload from unit**. |
| Device Settings says FFADO did not report the settings | The mix loaded but the settings read failed. Restart the app; if it persists, check `ffado-dbus-server -v3`. |
| Window crashes at start on Wayland ("Error 71 (Protocol error)") | WebKitGTK's DMA-BUF renderer. The app turns it off by default; if you set `WEBKIT_DISABLE_DMABUF_RENDERER=0` yourself, unset it. |
| Meters stay at the floor | The meter reader needs read/write access to `/dev/fw1`; see section 3. Meters show the unit's real levels, so also check something is playing or connected. |

## Using the unit with other FFADO tools

`ffado-mixer` talks to the same server, and changes made there can
overwrite what ffmixer sent (and the reverse). Use one at a time while
editing.
