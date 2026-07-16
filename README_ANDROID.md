# usbeehive for Android / Pixel

This fork evaluates how much of usbeehive can run on Android, with the initial target being a rooted Google Pixel and Termux.

## Current status

The core USB-PD decoders and the Linux sysfs backend are reusable without a rewrite. The first Android-specific addition is a read-only capability probe:

```text
usbeehive --capabilities
usbeehive --capabilities --json
```

The probe distinguishes these states for every required sysfs area:

- path is absent because the kernel does not export it;
- path exists and is readable;
- path exists but enumeration is blocked;
- metadata access is denied, so presence is unknown.

No USB, charging, kernel, SELinux, Magisk, DNS, route, or device setting is changed.

## Pre-build Pixel scan

`tools/android-pixel-capability-scan.sh` provides the same first decision gate before a Rust toolchain is installed. It is read-only and prints only selected USB, Type-C, PD and power-supply attributes; USB serial-number attributes are deliberately excluded.

The scanner has been checked with:

- LF line endings;
- `#!/usr/bin/env bash`;
- `bash -n`;
- no heredoc or here-string;
- a Linux smoke run ending in `RESULT: USBEEHIVE_ANDROID_CAPABILITY_SCAN_DONE rc=0`.

A Pixel run is still required to establish the real device capability state.

## Android build profile

Android does not provide libudev or a normal desktop D-Bus session. Build only the CLI and sysfs layers:

```text
cargo build --release --no-default-features --features cli,sysfs
```

This removes the `watch`, `udev`, `libc`, `dbus`, and `zbus` layers. A native Termux Rust toolchain is the preferred first build path. Cross-compilation with the Android NDK can follow after the native feasibility test.

## Required kernel interfaces

The useful result depends on what the Pixel kernel exports and what SELinux permits the caller to read:

| Area | Purpose |
|---|---|
| `/sys/bus/usb/devices` | USB devices, negotiated speed, topology, power declarations |
| `/sys/class/typec` | Type-C roles, partner, cable and e-marker identity |
| `/sys/class/usb_power_delivery` | PDOs, active contract and PPS capabilities |
| `/sys/class/power_supply` | Charging and power-supply attributes |

Basic USB information can still work when Type-C or USB-PD classes are absent. Full WhatCable-style cable ratings require the cable identity VDOs to be exported under `/sys/class/typec`.

## Deliberate limits

- The first phase is CLI-only and read-only.
- No automatic root escalation is implemented.
- No SELinux policy changes are planned.
- No claims about e-marker support are made until a real Pixel capability report is captured.
- `--watch` is excluded from the Android build; polling can be added later without libudev.

See [`docs/android-pixel-feasibility.md`](docs/android-pixel-feasibility.md) for the decision matrix and next steps.
