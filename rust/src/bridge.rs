//! The desktop app's door into the core (cxx). Every function returns one
//! JSON object, `{ok, error, log, ...}`: `log` holds the lines the CLI would
//! have printed. No panic can cross: each body runs under `catch_unwind` and a
//! panic comes back as `{ok: false, error}`.

use crate::apply::{self, Outcome};
use crate::{capture as capture_mod, facts, log, tiers};
use serde_json::{Map, Value, json};
use std::fs;
use std::path::Path;

#[cxx::bridge(namespace = "loadout")]
mod ffi {
    extern "Rust" {
        /// This machine: `{facts: {hostname, screen_w, screen_h, refresh_hz,
        /// gpu_name, gpu_vram_mb, devices: []}, tier, derived: [{key, value,
        /// reason}]}`. `ok: false` when nothing could be detected.
        fn detect() -> String;
        /// The profiles under `dir`: `{profiles: [{name, path, source_machine,
        /// captured_at, files: [{rel, sha256}], machines: [], error}]}`. A
        /// missing `dir` is an empty list.
        fn list_profiles(dir: &str) -> String;
        /// Capture the Options dir into the new profile dir `out`: `{profile}`.
        fn capture(options: &str, out: &str, machine: &str) -> String;
        /// What an apply WOULD do, nothing written: `{rows: [{key, value,
        /// source, reason}], files: [{rel, exists, changed, edits}], edits:
        /// [{file, element, value}], unconnected: [{guid, bindings}], facts,
        /// backup_root}`. `machine` "" = the detected hostname.
        fn plan_apply(profile: &str, options: &str, machine: &str) -> String;
        /// Back up and write: `{backup, stamp, written: []}`; `code` is the
        /// CLI's exit code (2 = refused, the game is running).
        fn apply(profile: &str, options: &str, machine: &str) -> String;
        /// Undo an apply from its backup folder ("" = the last one):
        /// `{backup, stamp}`.
        fn restore(options: &str, backup: &str) -> String;
        /// The newest apply backup under the Options dir: `{path, stamp}`,
        /// both "" when there is none.
        fn last_backup(options: &str) -> String;
        /// True while EliteDangerous64.exe is up.
        fn game_running() -> bool;
    }
}

/// Run `body`, collecting its lines; fold the result into the JSON envelope.
fn guarded(body: impl FnOnce() -> Result<Map<String, Value>, String>) -> String {
    let run = std::panic::AssertUnwindSafe(|| log::collect(body));
    let (result, lines) = match std::panic::catch_unwind(run) {
        Ok(r) => r,
        Err(p) => {
            let what = p.downcast_ref::<&str>().map(|s| (*s).to_owned()).or_else(|| p.downcast_ref::<String>().cloned());
            (Err(format!("internal error: {}", what.unwrap_or_else(|| "panic".into()))), Vec::new())
        }
    };
    let mut out = Map::new();
    match result {
        Ok(fields) => {
            out.insert("ok".into(), true.into());
            out.insert("error".into(), "".into());
            out.extend(fields);
        }
        Err(e) => {
            out.insert("ok".into(), false.into());
            out.insert("error".into(), e.into());
        }
    }
    out.insert("log".into(), lines.into());
    Value::Object(out).to_string()
}

fn object(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        _ => Map::new(),
    }
}

fn opt(s: &str) -> Option<&str> {
    (!s.is_empty()).then_some(s)
}

fn stamp_of(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

pub fn detect() -> String {
    guarded(|| {
        let f = facts::load(None)?.ok_or("this machine could not be detected")?;
        let d = tiers::derive(&f);
        let derived: Vec<Value> =
            d.lines.iter().map(|(key, value, reason)| json!({"key": key, "value": value, "reason": reason})).collect();
        Ok(object(json!({
            "facts": {
                "hostname": f.hostname,
                "screen_w": f.screen_w,
                "screen_h": f.screen_h,
                "refresh_hz": f.refresh_hz,
                "gpu_name": f.gpu_name,
                "gpu_vram_mb": f.gpu_vram_mb,
                "devices": f.devices,
            },
            "tier": format!("{:?}", d.tier),
            "derived": derived,
        })))
    })
}

/// One profile dir as JSON; a profile.toml that does not parse is listed
/// with its `error`.
fn profile_json(dir: &Path) -> Value {
    let name = stamp_of(dir);
    let path = dir.to_string_lossy().into_owned();
    let toml_path = dir.join("profile.toml");
    let table: Result<toml::Table, String> = fs::read_to_string(&toml_path)
        .map_err(|e| e.to_string())
        .and_then(|t| t.parse().map_err(|e: toml::de::Error| e.to_string()));
    let table = match table {
        Ok(t) => t,
        Err(e) => {
            return json!({"name": name, "path": path, "source_machine": "", "captured_at": "", "files": [], "machines": [], "error": e});
        }
    };
    let text = |k: &str| table.get(k).and_then(|v| v.as_str()).unwrap_or("").to_owned();
    let files: Vec<Value> = table
        .get("files")
        .and_then(|v| v.as_table())
        .map(|t| t.iter().map(|(rel, sha)| json!({"rel": rel, "sha256": sha.as_str().unwrap_or("")})).collect())
        .unwrap_or_default();
    let machines: Vec<String> =
        table.get("machine").and_then(|v| v.as_table()).map(|t| t.keys().cloned().collect()).unwrap_or_default();
    json!({
        "name": name,
        "path": path,
        "source_machine": text("source_machine"),
        "captured_at": text("captured_at"),
        "files": files,
        "machines": machines,
        "error": "",
    })
}

pub fn list_profiles(dir: &str) -> String {
    guarded(|| {
        let root = Path::new(dir);
        let mut dirs: Vec<std::path::PathBuf> = match fs::read_dir(root) {
            Ok(entries) => entries.flatten().map(|e| e.path()).filter(|p| p.join("profile.toml").is_file()).collect(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(format!("{}: {e}", root.display())),
        };
        dirs.sort();
        let profiles: Vec<Value> = dirs.iter().map(|d| profile_json(d)).collect();
        Ok(object(json!({ "profiles": profiles })))
    })
}

pub fn capture(options: &str, out: &str, machine: &str) -> String {
    guarded(|| {
        capture_mod::run(Path::new(options), Path::new(out), machine)?;
        Ok(object(json!({ "profile": out })))
    })
}

pub fn plan_apply(profile: &str, options: &str, machine: &str) -> String {
    guarded(|| {
        let options = Path::new(options);
        let facts = facts::load(None)?;
        let plan = apply::plan(Path::new(profile), options, opt(machine), facts.as_ref())?;
        let rows: Vec<Value> = plan
            .rows
            .iter()
            .map(|r| json!({"key": r.key, "value": r.value, "source": r.source, "reason": r.reason}))
            .collect();
        let files: Vec<Value> = plan
            .files
            .iter()
            .map(|(rel, bytes)| {
                let now = fs::read(crate::bundle::join_rel(options, rel)).ok();
                json!({
                    "rel": rel,
                    "exists": now.is_some(),
                    "changed": now.as_deref() != Some(bytes.as_slice()),
                    "edits": plan.edits.iter().filter(|e| &e.0 == rel).count(),
                })
            })
            .collect();
        let edits: Vec<Value> =
            plan.edits.iter().map(|(file, element, value)| json!({"file": file, "element": element, "value": value})).collect();
        let unconnected: Vec<Value> =
            plan.unconnected.iter().map(|(guid, n)| json!({"guid": guid, "bindings": n})).collect();
        Ok(object(json!({
            "rows": rows,
            "files": files,
            "edits": edits,
            "unconnected": unconnected,
            "facts": facts.is_some(),
            "backup_root": options.join(apply::BACKUP_DIR).to_string_lossy(),
        })))
    })
}

pub fn apply(profile: &str, options: &str, machine: &str) -> String {
    guarded(|| {
        let running = apply::game_running();
        match apply::apply_with(Path::new(profile), Path::new(options), opt(machine), None, false, running)? {
            Outcome::Written { backup, files } => Ok(object(json!({
                "code": 0,
                "backup": backup.to_string_lossy(),
                "stamp": stamp_of(&backup),
                "written": files,
            }))),
            Outcome::Refused(code) => Err(if code == 2 {
                "EliteDangerous64.exe is running; quit the game and apply again".to_owned()
            } else {
                format!("refused (code {code}); nothing written")
            }),
        }
    })
}

pub fn restore(options: &str, backup: &str) -> String {
    guarded(|| {
        let done = apply::restore(Path::new(options), opt(backup).map(Path::new))?;
        Ok(object(json!({ "backup": done.to_string_lossy(), "stamp": stamp_of(&done) })))
    })
}

pub fn last_backup(options: &str) -> String {
    guarded(|| {
        let last = apply::last_backup(Path::new(options));
        Ok(object(json!({
            "path": last.as_ref().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default(),
            "stamp": last.as_deref().map(stamp_of).unwrap_or_default(),
        })))
    })
}

pub fn game_running() -> bool {
    std::panic::catch_unwind(apply::game_running).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(s: String) -> Value {
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn a_panic_comes_back_as_json_not_across_the_bridge() {
        let v = parsed(guarded(|| panic!("boom {}", 7)));
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"], "internal error: boom 7");
        let v = parsed(guarded(|| Err("plain".into())));
        assert_eq!((v["ok"].clone(), v["error"].clone()), (json!(false), json!("plain")));
    }

    #[test]
    fn list_profiles_of_a_missing_dir_is_empty_and_of_a_capture_names_it() {
        let tmp = tempfile::tempdir().unwrap();
        let v = parsed(list_profiles(&tmp.path().join("nope").to_string_lossy()));
        assert_eq!(v["ok"], true);
        assert_eq!(v["profiles"], json!([]));

        let options = tmp.path().join("Options");
        fs::create_dir_all(options.join("Graphics")).unwrap();
        fs::write(options.join("Graphics/Settings.xml"), "<R>\n</R>\n").unwrap();
        let out = tmp.path().join("profiles").join("desk");
        let v = parsed(capture(&options.to_string_lossy(), &out.to_string_lossy(), "desk"));
        assert_eq!(v["ok"], true, "{v}");
        assert_eq!(v["log"][0], "captured Graphics/Settings.xml");
        fs::create_dir_all(tmp.path().join("profiles/not-a-profile")).unwrap();
        let v = parsed(list_profiles(&tmp.path().join("profiles").to_string_lossy()));
        assert_eq!(v["profiles"].as_array().unwrap().len(), 1);
        let p = &v["profiles"][0];
        assert_eq!((p["name"].clone(), p["source_machine"].clone()), (json!("desk"), json!("desk")));
        assert_eq!(p["files"][0]["rel"], "Graphics/Settings.xml");
        // A second capture into the same dir is refused, as JSON.
        let v = parsed(capture(&options.to_string_lossy(), &out.to_string_lossy(), "desk"));
        assert_eq!(v["ok"], false);
        assert!(v["error"].as_str().unwrap().contains("refusing to overwrite"), "{v}");
    }

    #[test]
    fn last_backup_of_an_untouched_options_dir_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let v = parsed(last_backup(&tmp.path().to_string_lossy()));
        assert_eq!((v["ok"].clone(), v["path"].clone(), v["stamp"].clone()), (json!(true), json!(""), json!("")));
    }
}
