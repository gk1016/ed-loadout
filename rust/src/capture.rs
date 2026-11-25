//! `capture`: copy the bundle byte-for-byte into a profile and write profile.toml.

use crate::bundle::{join_rel, scan_options, sha256_hex};
use std::fs;
use std::path::Path;

pub fn run(options: &Path, out: &Path, machine: &str) -> Result<i32, String> {
    let files = scan_options(options)?;
    if files.is_empty() {
        return Err(format!(
            "{}: no bundle files found under Graphics/ or Bindings/",
            options.display()
        ));
    }
    if out.join("profile.toml").exists() || out.join("files").exists() {
        return Err(format!(
            "{}: already holds a profile; refusing to overwrite",
            out.display()
        ));
    }

    let mut toml = String::new();
    toml.push_str(&format!("source_machine = {}\n", toml_str(machine)));
    toml.push_str(&format!(
        "captured_at = {}\n",
        toml_str(&chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
    ));
    toml.push_str("\n[files]\n");

    for rel in &files {
        let src = join_rel(options, rel);
        let bytes = fs::read(&src).map_err(|e| format!("{}: {e}", src.display()))?;
        let dst = join_rel(&out.join("files"), rel);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::write(&dst, &bytes).map_err(|e| format!("{}: {e}", dst.display()))?;
        toml.push_str(&format!("{} = {}\n", toml_str(rel), toml_str(&sha256_hex(&bytes))));
        println!("captured {rel}");
    }

    let profile_toml = out.join("profile.toml");
    fs::write(&profile_toml, toml).map_err(|e| format!("{}: {e}", profile_toml.display()))?;
    println!("wrote {} ({} files)", profile_toml.display(), files.len());
    Ok(0)
}

/// A TOML basic string, escaped.
fn toml_str(s: &str) -> String {
    toml::Value::String(s.to_owned()).to_string()
}
