//! The settings bundle: which files under the Elite options dir belong to it,
//! and sha256 hashing. Paths inside a bundle are relative and always use `/`.

use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// True when `rel` (e.g. `Graphics/Settings.xml`) is part of the bundle.
pub fn in_file_set(rel: &str) -> bool {
    let Some((dir, name)) = rel.split_once('/') else {
        return false;
    };
    if name.contains('/') {
        return false;
    }
    match dir {
        "Graphics" => {
            (name.starts_with("Custom.") && name.ends_with(".fxcfg"))
                || name == "Settings.xml"
                || name == "DisplaySettings.xml"
                || name == "StartPreset.start"
                || name == "GraphicsConfigurationOverride.xml"
        }
        "Bindings" => {
            name.ends_with(".binds")
                || (name.starts_with("StartPreset.") && name.ends_with(".start"))
        }
        _ => false,
    }
}

/// Every bundle file under `options`, as sorted relative paths.
pub fn scan_options(options: &Path) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for dir in ["Graphics", "Bindings"] {
        let d = options.join(dir);
        if !d.is_dir() {
            continue;
        }
        let entries = fs::read_dir(&d).map_err(|e| format!("{}: {e}", d.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("{}: {e}", d.display()))?;
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let rel = format!("{dir}/{name}");
            if in_file_set(&rel) {
                out.push(rel);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Join a `/`-separated relative path onto `base`.
pub fn join_rel(base: &Path, rel: &str) -> PathBuf {
    let mut p = base.to_path_buf();
    for part in rel.split('/') {
        p.push(part);
    }
    p
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::in_file_set;

    #[test]
    fn file_set_matches_the_plan() {
        for yes in [
            "Graphics/Custom.4.4.fxcfg",
            "Graphics/Settings.xml",
            "Graphics/DisplaySettings.xml",
            "Graphics/StartPreset.start",
            "Graphics/GraphicsConfigurationOverride.xml",
            "Bindings/Custom.4.2.binds",
            "Bindings/StartPreset.4.start",
        ] {
            assert!(in_file_set(yes), "{yes}");
        }
        for no in [
            "Bindings/StartPreset.4.start.broken",
            "Bindings/StartPreset.4.start.bak-20261121-120000-000",
            "Bindings/BindingLoadingErrors.log",
            "Graphics/Other.xml",
            "Settings.xml",
            "Graphics/sub/Settings.xml",
        ] {
            assert!(!in_file_set(no), "{no}");
        }
    }
}
