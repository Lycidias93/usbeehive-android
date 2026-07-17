//! usbeehive CLI entry point.

use std::io::{self, Write};

use clap::Parser;

use usbeehive::{DeviceManager, Sysfs};

mod output;

use output::{print_json, print_text, print_tree};

#[derive(Parser, Debug)]
#[command(
    name = "usbeehive",
    version,
    about = "Tells you what each USB cable / device on Linux can actually do.",
    long_about = "usbeehive — shows what each USB cable / device can do.\n\
                  Port of WhatCable (macOS) by Darryl Morley."
)]
struct Cli {
    /// Structured JSON output.
    #[arg(long)]
    json: bool,

    /// Render a flat list with full per-device details (default is a topology tree).
    #[arg(long, conflicts_with = "tree")]
    list: bool,

    /// Render the bus topology as a tree (default).
    #[arg(long, conflicts_with = "list")]
    tree: bool,

    /// Stream updates as devices change (requires the `watch` feature).
    #[arg(long)]
    watch: bool,

    /// Include raw sysfs attributes in the output.
    #[arg(long)]
    raw: bool,

    /// Report whether the required sysfs areas exist and are readable.
    #[arg(long)]
    capabilities: bool,

    /// Override the sysfs root (default: /sys). Useful for fixture-based testing.
    #[arg(long, value_name = "PATH")]
    sysfs_root: Option<std::path::PathBuf>,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let sysfs = match cli.sysfs_root.as_ref() {
        Some(p) => Sysfs::with_root(p),
        None => Sysfs::linux(),
    };

    if cli.capabilities {
        return print_capabilities(&sysfs, cli.json);
    }

    let mut mgr = DeviceManager::with_sysfs(sysfs);
    let use_list = cli.list;
    if cli.watch {
        #[cfg(feature = "watch")]
        return run_watch(&mut mgr, cli.json, use_list, cli.raw);
        #[cfg(not(feature = "watch"))]
        {
            eprintln!("--watch is not available: built without the `watch` feature.");
            std::process::exit(2);
        }
    }
    mgr.refresh();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if cli.json {
        print_json(&mut out, &mgr, cli.raw)
    } else if use_list {
        print_text(&mut out, &mgr, cli.raw)
    } else {
        print_tree(&mut out, &mgr)
    }
}

fn print_capabilities(sysfs: &Sysfs, use_json: bool) -> io::Result<()> {
    let report = sysfs.capability_report();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    if use_json {
        serde_json::to_writer_pretty(&mut out, &report).map_err(io::Error::other)?;
        writeln!(out)?;
        return Ok(());
    }

    writeln!(
        out,
        "Sysfs capability report (root: {})",
        report.root.display()
    )?;
    for area in &report.areas {
        let present = match area.present {
            Some(true) => "yes",
            Some(false) => "no",
            None => "unknown",
        };
        let entries = area
            .entry_count
            .map(|count| count.to_string())
            .unwrap_or_else(|| "-".to_string());
        writeln!(
            out,
            "- {}: present={} readable={} entries={} path={}",
            area.name,
            present,
            if area.readable { "yes" } else { "no" },
            entries,
            area.path.display()
        )?;
        if let Some(error) = &area.error {
            writeln!(out, "  error={error}")?;
        }
    }
    Ok(())
}

#[cfg(feature = "watch")]
fn run_watch(
    mgr: &mut DeviceManager,
    use_json: bool,
    use_list: bool,
    show_raw: bool,
) -> io::Result<()> {
    use std::time::Duration;
    use usbeehive::watch::{run_loop, RefreshReason};

    run_loop(Duration::from_millis(500), |reason| {
        mgr.refresh();
        let stdout = io::stdout();
        let mut out = stdout.lock();
        if !use_json {
            // Clear screen + home cursor between renders so the latest snapshot
            // is always visible at the top of the terminal.
            if matches!(reason, RefreshReason::Hotplug | RefreshReason::Initial) {
                write!(out, "\x1b[2J\x1b[H")?;
            }
            if use_list {
                print_text(&mut out, mgr, show_raw)?;
            } else {
                print_tree(&mut out, mgr)?;
            }
        } else {
            print_json(&mut out, mgr, show_raw)?;
        }
        out.flush()
    })
}
