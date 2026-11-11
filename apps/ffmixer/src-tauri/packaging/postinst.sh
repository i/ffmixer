#!/bin/sh
# Apply the FireWire udev rule now, so no reboot is needed. Failures are
# ignored: this runs in chroots and containers without udev.
udevadm control --reload 2>/dev/null || true
udevadm trigger --subsystem-match=firewire 2>/dev/null || true
exit 0
