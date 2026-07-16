#![cfg(feature = "cli")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "usbeehive-capability-cli-{label}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_usbeehive"))
}

#[test]
fn capabilities_text_distinguishes_present_and_missing() {
    let root = TempRoot::new("text");
    fs::create_dir_all(root.path().join("bus/usb/devices/1-1")).unwrap();

    let output = cli()
        .arg("--capabilities")
        .arg("--sysfs-root")
        .arg(root.path())
        .output()
        .expect("binary runs");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("usbDevices: present=yes readable=yes entries=1"));
    assert!(stdout.contains("typeC: present=no readable=no entries=-"));
}

#[test]
fn capabilities_json_is_machine_readable() {
    let root = TempRoot::new("json");
    fs::create_dir_all(root.path().join("class/typec/port0")).unwrap();

    let output = cli()
        .arg("--capabilities")
        .arg("--json")
        .arg("--sysfs-root")
        .arg(root.path())
        .output()
        .expect("binary runs");

    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["areas"][1]["name"], "typeC");
    assert_eq!(parsed["areas"][1]["present"], true);
    assert_eq!(parsed["areas"][1]["readable"], true);
    assert_eq!(parsed["areas"][1]["entryCount"], 1);
}
