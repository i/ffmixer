# Setup guide

> **Use at your own risk.** ffmixer sends commands to real audio
> hardware and can change 48 V phantom power, levels and the clock. A
> mistake or a bug could damage microphones or other equipment,
> speakers, headphones or your hearing. It has only been tested on one
> Fireface 800 on one Fedora machine. There is no warranty (see
> [LICENSE](../LICENSE)). Turn monitors and headphones down and
> disconnect delicate equipment before you start ffmixer for the first
> time.

ffmixer controls an RME Fireface through FFADO's `ffado-dbus-server` and
reads the level meters directly from the FireWire device. Only the
Fireface 800 is supported so far. This guide was tested on Fedora 44
with the kernel `snd_fireface` driver and PipeWire loaded. Package names
for other distributions are from memory, so check them.

## 1. Requirements

- A Fireface 800 connected over FireWire (OHCI, kernel module
  `firewire_ohci`). `ls /dev/fw*` should list `fw0` (the controller) and
  `fw1` (the unit).
- FFADO 2.4.x with its D-Bus server.
- To build from source: Rust 1.85+, Node.js 22+ and the WebKitGTK 4.1
  development files. The Tauri CLI comes with `npm install`.

| Package | Fedora (tested) | Debian / Ubuntu, Arch |
|---|---|---|
| FFADO | `sudo dnf install ffado` | `ffado-dbus-server`; `libffado` |
| WebKitGTK 4.1 | `sudo dnf install webkit2gtk4.1-devel` | `libwebkit2gtk-4.1-dev`; `webkit2gtk-4.1` |

Tauri may need more development packages (`openssl-devel`,
`librsvg2-devel` and similar). See
<https://v2.tauri.app/start/prerequisites/>.

## 2. FireWire permissions

`/dev/fw*` is root-only by default. Without access, FFADO fails with
"No FireWire adapters found" and the app starts in demo mode.

```bash
sudo usermod -aG audio "$USER"
echo 'SUBSYSTEM=="firewire", GROUP="audio", MODE="0660"' \
  | sudo tee /etc/udev/rules.d/60-firewire-audio.rules
sudo udevadm control --reload
sudo udevadm trigger --subsystem-match=firewire
```

Log out and back in, then check:

```bash
id -nG | tr ' ' '\n' | grep -x audio     # prints: audio
ls -l /dev/fw0 /dev/fw1                  # both: crw-rw---- ... audio
```

Don't run the app or FFADO as root.

## 3. Check that FFADO sees the unit

The app starts `ffado-dbus-server` through D-Bus when it needs it.
To check it first:

```bash
busctl --user tree org.ffado.Control | grep -c DeviceManager
```

A number above zero (about 58 for a Fireface 800) means FFADO found the
unit. If it's zero or an error, see [Troubleshooting](#troubleshooting).

The kernel `snd_fireface` driver and PipeWire can stay loaded. FFADO is
only used for the mixer and settings, not for audio.

## 4. Before the first run

> **Every time its server starts, FFADO loads the unit's saved (flash)
> mix onto the live mixer.** Changes made since the last "store in
> flash" are lost, for example a mix made in TotalMix on another
> computer.

If you care about the current mix, save it to the unit first with RME's
own software. Turn monitors and headphones down, because the saved mix
may be louder than you expect.

## 5. Get the app

**Download a release.** Each release has a tarball and a `.sha256` file
for 64-bit systems (`x86_64`) and for 32-bit x86 (`i686`). The 32-bit
build has not been run on real hardware. Use the one that matches your
system (`uname -m` prints `x86_64` or `i686`):

```bash
sha256sum -c ffmixer-*-linux-x86_64.tar.gz.sha256
tar xzf ffmixer-*-linux-x86_64.tar.gz
./ffmixer-*/ffmixer
```

The 32-bit build also needs the 32-bit WebKitGTK package. The builds
are made on Ubuntu 22.04 and needs the WebKitGTK 4.1 runtime package
(`libwebkit2gtk-4.1-0` or `webkit2gtk4.1`) and a glibc at least as new
as Ubuntu 22.04's. You still need FFADO and the permissions above.

**Build from source.**

```bash
git clone https://github.com/i/ffmixer.git
cd ffmixer/apps/ffmixer
npm install
npm run tauri dev                      # development, hot reload
npm run tauri -- build --no-bundle     # release binary, no installer
```

The binary is `target/release/ffmixer` (or under `$CARGO_TARGET_DIR`).

To run the checks:

```bash
cargo test --workspace
cargo test -p ffmixer -- --ignored    # needs the unit; reads only
cd apps/ffmixer && npm run check
```

## 6. First run

1. The Control Strip (right) shows **Live** and "In sync with unit".
   If it says **Demo**, the app could not reach FFADO or the unit. If it
   says **Offline**, it found the unit but could not start sending. The
   status line says why in both cases.
2. The meters move on their own.
3. Changes are sent to the unit as you make them. If the connection is
   lost, press **Reconnect**: it reads the unit again, and changes that
   were not sent are discarded.
4. **Device Settings** can only be changed while connected. Risky
   changes ask for confirmation first. Tick **Hide warnings** at the
   bottom of the sidebar to skip these questions once you trust your
   setup (off by default, remembered across restarts).

Channel names are saved in `~/.config/ffmixer/names.json`. Nothing is
saved to the unit's flash, so it forgets what ffmixer sent when it is
powered off.

## Troubleshooting

| Symptom | Cause and fix |
|---|---|
| "Demo mode: cannot reach ffado-dbus-server" or "reports no devices" | FFADO is missing, didn't start, or can't see the unit. Run `ffado-dbus-server -v3` and read the output. Check section 2 first. |
| `journalctl --user -b -g ffado` shows "No FireWire adapters found" | No access to `/dev/fw*`. Redo section 2 and log in again. |
| `ls /dev/fw*` shows only `fw0` | The unit is off or disconnected, or the cable or port is faulty. |
| "Not sending: FFADO has N mute/invert flags set" | `ffado-mixer` set flags that would apply to what ffmixer sends. Clear them there, or restart `ffado-dbus-server`. |
| "Stopped sending: ffado-dbus-server restarted" | The server restarted and reloaded the flash mix. Press **Reconnect**. |
| "Stopped sending" with another message | A write failed. The unit may not match what is shown. Press **Reconnect**. |
| Device Settings says FFADO did not report the settings | Restart the app. If it persists, check `ffado-dbus-server -v3`. |
| Crash at start on Wayland ("Error 71") | WebKitGTK's DMA-BUF renderer. The app turns it off by default. Unset `WEBKIT_DISABLE_DMABUF_RENDERER=0` if you set it. |
| Meters stay at the floor | The meter reader needs access to `/dev/fw1` (section 2), and something must be playing. |

`ffado-mixer` uses the same server. Changes made there can overwrite
what ffmixer sent, and the other way round, so use only one at a time.
