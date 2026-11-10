# totalnix

A TotalMix FX-style mixer for the RME Fireface 800 on Linux, built on
`ffado-dbus-server`. See `docs/totalmix-fx-reference.md` for the target
design and the hardware/FFADO findings.

## Layout

| Crate | Purpose |
|---|---|
| `crates/ffado-dbus` | Read-only client for `ffado-dbus-server`. Only getter methods are allowed (`READ_METHODS`). |
| `crates/fw-meter` | Read-only Fireface 800 level meters via `/dev/fw*`. Sends block reads only. |
| `crates/ffado-probe` | Prints controls, non-silent mixer routes and a few seconds of meters. |

Nothing in the workspace can write to the device yet.

## Requirements

- `ffado-dbus-server` (FFADO 2.4.x), started on demand by D-Bus.
- Read/write access to `/dev/fw*`: be in the `audio` group, plus a udev
  rule such as
  `SUBSYSTEM=="firewire", GROUP="audio", MODE="0660"`.

## Run

```bash
cargo test
cargo run -p ffado-probe -- 5    # 5 s of meters; 0 skips them
```
