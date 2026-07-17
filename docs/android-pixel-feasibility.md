# Android / Pixel feasibility

## Goal

Determine whether a rooted Pixel exposes enough read-only USB-C and USB Power Delivery state to provide a useful Android equivalent of WhatCable.

This phase changes repository code only. It does not install software, alter SELinux, or change any device setting. Live validation uses an already-authorized read-only root context.

## Existing reusable components

The upstream Rust implementation already separates the relevant layers:

- pure USB-PD VDO and cable decoders;
- charging-bottleneck diagnostics;
- Linux sysfs enumeration;
- CLI text and JSON output;
- optional libudev watch support;
- optional D-Bus daemon.

Android can use the first four layers. The `watch` and `dbus` layers are intentionally disabled for the initial build.

## First decision gate

Run the Android CLI with `--capabilities --json` against the real `/sys` tree. The report covers:

1. `/sys/bus/usb/devices`
2. `/sys/class/typec`
3. `/sys/class/usb_power_delivery`
4. `/sys/class/power_supply`

Each area reports:

- `present=true`: metadata confirms the directory exists;
- `present=false`: the kernel does not expose the directory;
- `present=null`: metadata access itself was denied;
- `readable=true`: directory enumeration succeeded;
- `entryCount`: immediate entries visible to the caller;
- `error`: stable failure class such as `permission_denied`.

## Pixel 10 Pro XL live result

A read-only root scan on 2026-07-16 established the following state while a PD-capable charger was actively connected:

- `/sys/class/typec`, `/sys/class/usb_power_delivery`, and `/sys/class/power_supply` are present and readable;
- `port0` reports USB Power Delivery 3.0 and USB Type-C 1.2;
- a Type-C partner is registered;
- the partner identity directory is present, but all returned identity VDO values are zero;
- no `port0-cable` node is exported for the tested connection;
- PD-PPS is active;
- the live Type-C/PD power-supply path reports approximately 9.54 V and 2.65 A, or about 25.3 W;
- the same path exposes limits of 11 V and 3 A for the current charging state.

This proves that an Android prototype can report live charging mode, negotiated voltage/current, PD/PPS state, roles, orientation, and charging bottlenecks on this Pixel. It does not yet prove direct cable e-marker decoding or a reliable maximum cable wattage, because the kernel did not export a cable identity node and the partner VDOs were zero.

The standalone scanner treats optional sysfs attributes that return `ENODATA` or `EINVAL` as unavailable values. These expected driver states are omitted instead of being printed as shell read errors.

## Outcome matrix

| Result | Meaning | Next action |
|---|---|---|
| USB readable, Type-C and PD absent | Basic USB device inspector only | Keep CLI scope; do not claim cable ratings |
| Type-C readable, cable identity absent | Roles, partner state, and live charging may be available | Add Android charging summary; cable capability remains unknown |
| Type-C cable identity readable | E-marker VDOs available | Verify 3A/5A, speed and maximum-watt decoding |
| USB-PD readable | Charger and active-contract data may be available | Enable charging-bottleneck diagnostics |
| Paths present but permission denied | Kernel support exists but caller is blocked | Repeat the same read-only probe through an existing root context; do not change SELinux |
| All four areas readable | Full prototype is justified | Build Termux CLI, then a small root bridge or Magisk WebUI |

The Pixel live result reaches the full-prototype gate for charging diagnostics, but not the e-marker gate for direct cable-rating claims.

## Build strategy

### Phase 1: native Termux CLI

Use the no-libudev feature set:

```text
cargo build --release --no-default-features --features cli,sysfs
```

Expected binary behavior:

- `--capabilities` works without reading individual device attributes;
- normal text and JSON scans degrade gracefully when optional paths are absent;
- no background service and no hotplug watcher;
- no root escalation inside the binary.

### Phase 2: Pixel fixture capture

When direct access is insufficient, capture only the required sysfs subtree through an already-authorized read-only root context and replay it with `--sysfs-root`. Do not collect USB serial numbers unless explicitly needed and approved.

The first fixture should preserve the successful active-PPS state and should include Type-C, USB-PD, and selected power-supply attributes. Cable identity must remain optional because it was absent in the real connection.

### Phase 3: Android presentation layer

The live report proves useful data exists. The next presentation layer can therefore:

- add polling without libudev;
- expose structured JSON through a minimal local bridge;
- show active PD/PPS mode, voltage, current, approximate input power, charging status, roles, and orientation;
- distinguish observed charging state from advertised cable capability;
- show cable identity as unavailable rather than inferring an unsupported rating;
- later become a Magisk WebUI or Android client.

## Root-cause boundary

A missing result is not automatically an application bug. The likely classes are:

1. Pixel kernel does not register the Type-C or PD class;
2. the controller driver exposes only partial attributes;
3. Android SELinux or file permissions block the caller;
4. the connected cable has no exported e-marker identity;
5. no PD-capable partner is connected during the scan;
6. an optional sysfs attribute exists but currently returns `ENODATA` or `EINVAL`.

The capability report and standalone scanner separate these classes before further implementation.

## Risk

- Repository and fixture work: low.
- Read-only Pixel scan: low.
- Existing-root read bridge: low to medium.
- Persistent Magisk service or WebUI: medium.
- Kernel or SELinux modifications: high and excluded from this phase.
