# Android / Pixel feasibility

## Goal

Determine whether a rooted Pixel exposes enough read-only USB-C and USB Power Delivery state to provide a useful Android equivalent of WhatCable.

This phase changes repository code only. It does not install software, escalate privileges, alter SELinux, or change any device setting.

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

## Outcome matrix

| Result | Meaning | Next action |
|---|---|---|
| USB readable, Type-C and PD absent | Basic USB device inspector only | Keep CLI scope; do not claim cable ratings |
| Type-C readable, cable identity absent | Roles and partner state available | Add Android summary mode; cable capability remains unknown |
| Type-C cable identity readable | E-marker VDOs available | Verify 3A/5A, speed and maximum-watt decoding |
| USB-PD readable | Charger PDO and active-contract data available | Enable charging-bottleneck diagnostics |
| Paths present but permission denied | Kernel support exists but caller is blocked | Repeat the same read-only probe through an existing root context; do not change SELinux |
| All four areas readable | Full prototype is justified | Build Termux CLI, then a small root bridge / Magisk WebUI |

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

### Phase 3: Android presentation layer

Only after the live report proves useful data exists:

- add polling without libudev;
- expose structured JSON through a minimal local bridge;
- build a Magisk WebUI or Android client;
- preserve a clear distinction between observed link state and advertised cable capability.

## Root-cause boundary

A missing result is not automatically an application bug. The likely classes are:

1. Pixel kernel does not register the Type-C or PD class;
2. the controller driver exposes only partial attributes;
3. Android SELinux or file permissions block the caller;
4. the connected cable has no e-marker;
5. no PD-capable partner is connected during the scan.

The capability report exists to separate classes 1 and 3 before any further implementation.

## Risk

- Repository and fixture work: low.
- Read-only Pixel scan: low.
- Existing-root read bridge: low to medium.
- Persistent Magisk service or WebUI: medium.
- Kernel or SELinux modifications: high and excluded from this phase.
