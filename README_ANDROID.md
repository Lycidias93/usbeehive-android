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

## Pixel live proof

A read-only root scan on a Pixel 10 Pro XL succeeded with an active PD-PPS charger connection.

Confirmed live data includes:

- readable Type-C, USB Power Delivery, and power-supply classes;
- USB Power Delivery 3.0 and USB Type-C 1.2;
- active Type-C partner registration;
- active PD-PPS charging;
- approximately 9.54 V and 2.65 A on the Type-C/PD path, or about 25.3 W;
- charging state, roles, orientation, voltage, current, and current-path limits.

The tested connection did not expose a `port0-cable` node. The partner identity directory existed, but all returned identity VDO values were zero. The prototype can therefore report live charging behavior and bottlenecks, but it must not infer a cable e-marker rating or maximum cable wattage from this connection.

## Pre-build Pixel scan

`tools/android-pixel-capability-scan.sh` provides the same first decision gate before a Rust toolchain is installed. It is read-only and prints only selected USB, Type-C, PD and power-supply attributes; USB serial-number attributes are deliberately excluded.

The scanner has been checked with:

- LF line endings;
- `#!/usr/bin/env bash`;
- `bash -n`;
- no heredoc or here-string;
- a Linux smoke run ending in `RESULT: USBEEHIVE_ANDROID_CAPABILITY_SCAN_DONE rc=0`;
- a real rooted Pixel run with active PD-PPS.

Optional sysfs attributes that return `ENODATA` or `EINVAL` are treated as unavailable and omitted without shell read noise.

For a rooted Pixel, `tools/android-pixel-root-capability-run.sh` is the guarded wrapper. It verifies both scripts, enters the existing root context with `su -c`, performs no writes or policy changes, and emits:

```text
RESULT: USBEEHIVE_ANDROID_ROOT_CHILD_SCAN_DONE rc=0
RESULT: USBEEHIVE_ANDROID_ROOT_CAPABILITY_SCAN_DONE rc=0
```

The root wrapper is intended to be copied together with the scanner into Termux `$PREFIX/tmp` and launched through the established `cg-run-file` workflow. It does not install a service, persist a binary, modify SELinux, or edit a Magisk module.

## Android build profile

Android does not provide libudev or a normal desktop D-Bus session. Build only the CLI and sysfs layers:

```text
cargo build --release --no-default-features --features cli,sysfs
```

This removes the `watch`, `udev`, `libc`, `dbus`, and `zbus` layers. A native Termux Rust toolchain is the preferred first build path. Cross-compilation with the Android NDK can follow after the native feasibility test.

The pull-request CI matrix includes this exact no-libudev feature profile in addition to the upstream default and decoder-only builds. Rustfmt, Clippy, MSRV, rustdoc and the shell syntax check remain required before merge.

## Required kernel interfaces

The useful result depends on what the Pixel kernel exports and what SELinux permits the caller to read:

| Area | Purpose |
|---|---|
| `/sys/bus/usb/devices` | USB devices, negotiated speed, topology, power declarations |
| `/sys/class/typec` | Type-C roles, partner, cable and e-marker identity |
| `/sys/class/usb_power_delivery` | PDOs, active contract and PPS capabilities |
| `/sys/class/power_supply` | Charging and power-supply attributes |

The Pixel live proof establishes enough data for an Android charging inspector and bottleneck analyzer. Full WhatCable-style cable ratings still require nonzero cable identity VDOs to be exported under `/sys/class/typec`.

## Deliberate limits

- The first phase is CLI-only and read-only.
- Root is used only through the explicit capability runner.
- No SELinux policy changes are planned.
- No persistent root service is installed.
- Live charging state must remain distinct from advertised cable capability.
- Missing cable identity is reported as unavailable, never guessed.
- `--watch` is excluded from the Android build; polling can be added later without libudev.

See [`docs/android-pixel-feasibility.md`](docs/android-pixel-feasibility.md) for the decision matrix and next steps.
