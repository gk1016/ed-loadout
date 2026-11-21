//! `check-binds`: binds-file resolution, bound-device check, preset check.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const LOG_NAME: &str = "BindingLoadingErrors.log";
pub const PRESET_NAME: &str = "StartPreset.4.start";
const NOT_DEVICES: [&str; 3] = ["Keyboard", "Mouse", "{NoDevice}"];

/// Resolve the binds file. `StartPreset.4.start` is never consulted: Elite
/// overwrites it when a bound device is missing. Order: (a) `--binds`,
/// (b) the file named on line 1 of BindingLoadingErrors.log if it exists,
/// (c) the newest `Custom.4.*.binds` by modification time.
pub fn resolve_binds(dir: &Path, explicit: Option<&Path>) -> Result<(PathBuf, &'static str), String> {
    if let Some(p) = explicit {
        let p = if p.exists() || p.is_absolute() { p.to_path_buf() } else { dir.join(p) };
        if !p.is_file() {
            return Err(format!("{}: --binds file not found", p.display()));
        }
        return Ok((p, "--binds"));
    }
    if let Ok(log) = fs::read_to_string(dir.join(LOG_NAME))
        && let Some(name) = log.lines().next().and_then(log_preset_name)
    {
        let p = dir.join(name);
        if p.is_file() {
            return Ok((p, LOG_NAME));
        }
    }
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !(name.starts_with("Custom.4.") && name.ends_with(".binds")) {
            continue;
        }
        let Ok(mtime) = entry.metadata().and_then(|m| m.modified()) else {
            continue;
        };
        if best.as_ref().is_none_or(|(t, _)| mtime > *t) {
            best = Some((mtime, entry.path()));
        }
    }
    best.map(|(_, p)| (p, "newest Custom.4.*.binds"))
        .ok_or_else(|| format!("{}: no binds file found", dir.display()))
}

/// `There where errors when loading preset file: Custom.4.2.binds` -> the file name.
fn log_preset_name(line: &str) -> Option<&str> {
    let (_, name) = line.split_once("preset file:")?;
    let name = name.trim();
    (!name.is_empty() && !name.contains(['/', '\\'])).then_some(name)
}

/// Every `Device="..."` attribute value with its count, minus Keyboard/Mouse/{NoDevice}.
pub fn bound_devices(binds: &str) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    let mut rest = binds;
    while let Some(i) = rest.find("Device=\"") {
        let preceded_ok = rest[..i]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace);
        let after = &rest[i + "Device=\"".len()..];
        let Some(end) = after.find('"') else { break };
        let value = &after[..end];
        if preceded_ok && !NOT_DEVICES.contains(&value) {
            *out.entry(value.to_owned()).or_insert(0) += 1;
        }
        rest = &after[end..];
    }
    out
}

/// Unique GUIDs from `Failed to find GUID for device: <GUID>` lines, in order.
pub fn missing_from_log(log: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in log.lines() {
        if let Some((_, guid)) = line.split_once("Failed to find GUID for device:") {
            let guid = guid.trim().to_owned();
            if !guid.is_empty() && !out.contains(&guid) {
                out.push(guid);
            }
        }
    }
    out
}

/// One warning per line of StartPreset.4.start (lines 1-4) that is not `Custom`.
pub fn preset_warnings(dir: &Path) -> Vec<String> {
    let path = dir.join(PRESET_NAME);
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => return vec![format!("{}: {e}", path.display())],
    };
    let lines: Vec<&str> = text.lines().map(|l| l.trim_end_matches('\r')).collect();
    (0..4)
        .filter_map(|i| match lines.get(i) {
            Some(&"Custom") => None,
            Some(l) => Some(format!("{PRESET_NAME} line {}: \"{l}\", not Custom", i + 1)),
            None => Some(format!("{PRESET_NAME} line {}: missing, not Custom", i + 1)),
        })
        .collect()
}

pub fn run(dir: &Path, devices: Option<&str>, from_log: bool, binds: Option<&Path>) -> Result<i32, String> {
    let (binds_path, how) = resolve_binds(dir, binds)?;
    println!("binds: {} (from {how})", binds_path.display());
    let text = fs::read(&binds_path).map_err(|e| format!("{}: {e}", binds_path.display()))?;
    let bound = bound_devices(&String::from_utf8_lossy(&text));

    let missing: Vec<String> = if from_log {
        let log_path = dir.join(LOG_NAME);
        let log = fs::read_to_string(&log_path).map_err(|e| format!("{}: {e}", log_path.display()))?;
        missing_from_log(&log)
    } else {
        let present: Vec<String> = devices
            .unwrap_or("")
            .split(',')
            .map(|d| d.trim().to_ascii_uppercase())
            .filter(|d| !d.is_empty())
            .collect();
        bound
            .keys()
            .filter(|g| !present.contains(&g.to_ascii_uppercase()))
            .cloned()
            .collect()
    };

    for guid in &missing {
        let n = bound.get(guid).copied().unwrap_or(0);
        let plural = if n == 1 { "binding" } else { "bindings" };
        println!("missing device: {guid} ({n} {plural})");
    }
    let warnings = preset_warnings(dir);
    for w in &warnings {
        println!("warning: {w}");
    }
    if missing.is_empty() && warnings.is_empty() {
        println!("ok: every bound device present, {PRESET_NAME} is Custom x4");
        Ok(0)
    } else {
        Ok(1)
    }
}

/// `restore-preset`: back up StartPreset.4.start, then write `<preset>\r\n` x4.
pub fn restore_preset(dir: &Path, preset: &str) -> Result<i32, String> {
    if preset.is_empty() || preset.contains(['\r', '\n']) {
        return Err(format!("invalid preset name {preset:?}"));
    }
    let path = dir.join(PRESET_NAME);
    if path.exists() {
        let bak = dir.join(format!("{PRESET_NAME}.bak-{}", crate::bundle::backup_stamp()));
        if bak.exists() {
            return Err(format!("{}: backup already exists", bak.display()));
        }
        fs::copy(&path, &bak).map_err(|e| format!("{}: {e}", bak.display()))?;
        println!("backup: {}", bak.display());
    } else if !dir.is_dir() {
        return Err(format!("{}: not a directory", dir.display()));
    } else {
        println!("note: {} did not exist; nothing to back up", path.display());
    }
    let body = format!("{preset}\r\n").repeat(4);
    fs::write(&path, body).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("wrote {} ({preset} x4, CRLF)", path.display());
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bound_devices_skips_non_devices_and_device_index() {
        let s = r#"<Primary Device="231D0126" Key="Joy_1" /><Secondary Device="{NoDevice}" Key="" />
<Binding Device="Keyboard" Key="Key_A" /><X Device="Mouse" /><Y DeviceIndex="0" Device="045E0B22" />"#;
        let m = bound_devices(s);
        assert_eq!(m.len(), 2);
        assert_eq!(m["231D0126"], 1);
        assert_eq!(m["045E0B22"], 1);
    }

    #[test]
    fn log_parsing() {
        assert_eq!(
            log_preset_name("There where errors when loading preset file: Custom.4.2.binds"),
            Some("Custom.4.2.binds")
        );
        assert_eq!(log_preset_name("nothing here"), None);
        let log = "Failed to find GUID for device: 045E0B22\r\nFailed to find GUID for device: 045E0B22\nFailed to find GUID for device: ABCD0001\n";
        assert_eq!(missing_from_log(log), vec!["045E0B22", "ABCD0001"]);
    }
}
