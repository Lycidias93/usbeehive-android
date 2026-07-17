//! Read-only sysfs availability reporting.
//!
//! Android and locked-down Linux systems may expose a sysfs directory but deny
//! enumeration. Normal device scans intentionally degrade to empty collections;
//! this module keeps that behavior while providing an explicit capability probe
//! for diagnostics.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::reader::Sysfs;

/// Availability of one sysfs area used by usbeehive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SysfsAreaCapability {
    /// Stable machine-readable area name.
    pub name: String,
    /// Absolute path that was probed.
    pub path: PathBuf,
    /// Whether the path is present. `None` means metadata access was denied.
    pub present: Option<bool>,
    /// Whether the directory can be enumerated.
    pub readable: bool,
    /// Number of immediate entries when enumeration succeeded.
    pub entry_count: Option<usize>,
    /// Stable error-kind string when probing failed.
    pub error: Option<String>,
}

/// Capability report for every sysfs area used by usbeehive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SysfsCapabilityReport {
    /// Sysfs root supplied to [`Sysfs`].
    pub root: PathBuf,
    /// Per-area probe results.
    pub areas: Vec<SysfsAreaCapability>,
}

impl Sysfs {
    /// Probe the required sysfs areas without reading device attributes.
    pub fn capability_report(&self) -> SysfsCapabilityReport {
        SysfsCapabilityReport {
            root: self.root().to_path_buf(),
            areas: vec![
                probe_area("usbDevices", &self.usb_devices_dir()),
                probe_area("typeC", &self.typec_dir()),
                probe_area("usbPowerDelivery", &self.pd_dir()),
                probe_area("powerSupply", &self.power_supply_dir()),
            ],
        }
    }
}

fn probe_area(name: &str, path: &Path) -> SysfsAreaCapability {
    match fs::metadata(path) {
        Ok(metadata) if !metadata.is_dir() => SysfsAreaCapability {
            name: name.to_string(),
            path: path.to_path_buf(),
            present: Some(true),
            readable: false,
            entry_count: None,
            error: Some("not_a_directory".to_string()),
        },
        Ok(_) => match fs::read_dir(path) {
            Ok(entries) => {
                let mut count = 0usize;
                let mut first_error = None;
                for entry in entries {
                    match entry {
                        Ok(_) => count += 1,
                        Err(error) if first_error.is_none() => {
                            first_error = Some(error_kind(&error));
                        }
                        Err(_) => {}
                    }
                }
                SysfsAreaCapability {
                    name: name.to_string(),
                    path: path.to_path_buf(),
                    present: Some(true),
                    readable: first_error.is_none(),
                    entry_count: Some(count),
                    error: first_error,
                }
            }
            Err(error) => SysfsAreaCapability {
                name: name.to_string(),
                path: path.to_path_buf(),
                present: Some(true),
                readable: false,
                entry_count: None,
                error: Some(error_kind(&error)),
            },
        },
        Err(error) if error.kind() == ErrorKind::NotFound => SysfsAreaCapability {
            name: name.to_string(),
            path: path.to_path_buf(),
            present: Some(false),
            readable: false,
            entry_count: None,
            error: None,
        },
        Err(error) => SysfsAreaCapability {
            name: name.to_string(),
            path: path.to_path_buf(),
            present: None,
            readable: false,
            entry_count: None,
            error: Some(error_kind(&error)),
        },
    }
}

fn error_kind(error: &std::io::Error) -> String {
    match error.kind() {
        ErrorKind::NotFound => "not_found".to_string(),
        ErrorKind::PermissionDenied => "permission_denied".to_string(),
        kind => format!("{kind:?}").to_ascii_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_root(label: &str) -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "usbeehive-capability-{label}-{}-{sequence}",
            std::process::id()
        ))
    }

    #[test]
    fn report_distinguishes_present_and_missing_areas() {
        let root = temp_root("present-missing");
        let usb_devices = root.join("bus/usb/devices");
        fs::create_dir_all(usb_devices.join("1-1")).unwrap();

        let report = Sysfs::with_root(&root).capability_report();
        let usb = report
            .areas
            .iter()
            .find(|area| area.name == "usbDevices")
            .unwrap();
        assert_eq!(usb.present, Some(true));
        assert!(usb.readable);
        assert_eq!(usb.entry_count, Some(1));

        let typec = report
            .areas
            .iter()
            .find(|area| area.name == "typeC")
            .unwrap();
        assert_eq!(typec.present, Some(false));
        assert!(!typec.readable);
        assert!(typec.entry_count.is_none());

        fs::remove_dir_all(root).unwrap();
    }
}
