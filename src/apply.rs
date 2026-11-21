//! `apply`: back up, write the profile files, apply `[machine.<name>]`
//! overrides by replacing element text in place. Never inserts an element.

use crate::bundle::{backup_stamp, join_rel, scan_profile};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const DISPLAY: &str = "Graphics/DisplaySettings.xml";
const PROCESS: &str = "EliteDangerous64.exe";

/// True when Elite Dangerous is running. Windows only; elsewhere prints a
/// notice and returns false.
#[cfg(windows)]
pub fn game_running() -> bool {
    let filter = format!("IMAGENAME eq {PROCESS}");
    match std::process::Command::new("tasklist")
        .args(["/FI", &filter, "/NH"])
        .output()
    {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .to_ascii_lowercase()
            .contains(&PROCESS.to_ascii_lowercase()),
        Err(e) => {
            eprintln!("notice: could not run tasklist ({e}); game-running check skipped");
            false
        }
    }
}

#[cfg(not(windows))]
pub fn game_running() -> bool {
    eprintln!("notice: game-running check ({PROCESS}) is Windows-only; skipped on this platform");
    false
}

pub fn run(profile: &Path, options: &Path, machine: &str) -> Result<i32, String> {
    run_with(profile, options, machine, game_running())
}

/// `running` is injected so the refusal path is testable off Windows.
pub fn run_with(profile: &Path, options: &Path, machine: &str, running: bool) -> Result<i32, String> {
    if running {
        eprintln!("refusing: {PROCESS} is running; quit the game and apply again");
        return Ok(2);
    }

    let toml_path = profile.join("profile.toml");
    let table: toml::Table = fs::read_to_string(&toml_path)
        .map_err(|e| format!("{}: {e}", toml_path.display()))?
        .parse()
        .map_err(|e| format!("{}: {e}", toml_path.display()))?;
    let overrides = machine_overrides(&table, machine)?;

    // Final contents of every file, overrides applied, before anything is
    // written: a missing element aborts with the options dir untouched.
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for rel in scan_profile(profile)? {
        let p = join_rel(&profile.join("files"), &rel);
        let bytes = fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        files.insert(rel, bytes);
    }
    for (rel, tag, value) in edits(&files, &overrides)? {
        if tag == "SSAAMultiplier" {
            refuse_newer_fxcfg(options, &rel)?;
        }
        let bytes = files.get_mut(&rel).expect("edit targets a profile file");
        let text = std::str::from_utf8(bytes).map_err(|_| format!("{rel}: not UTF-8"))?;
        let new = replace_element(text, &tag, &value).map_err(|e| format!("{rel}: {e}"))?;
        println!("override {rel}: {tag} = {value}");
        *bytes = new.into_bytes();
    }

    // Backup every file that will be overwritten.
    let backup_root = options.join("_loadout-backup").join(backup_stamp());
    let mut backed_up = 0;
    for rel in files.keys() {
        let target = join_rel(options, rel);
        if target.exists() {
            let bak = join_rel(&backup_root, rel);
            if let Some(parent) = bak.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
            }
            fs::copy(&target, &bak).map_err(|e| format!("{}: {e}", bak.display()))?;
            backed_up += 1;
        }
    }
    if backed_up > 0 {
        println!("backup: {} ({backed_up} files)", backup_root.display());
    }

    for (rel, bytes) in &files {
        let target = join_rel(options, rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::write(&target, bytes).map_err(|e| format!("{}: {e}", target.display()))?;
        println!("wrote {rel}");
    }
    Ok(0)
}

#[derive(Debug, Default, PartialEq)]
struct Overrides {
    ssaa: Option<f64>,
    screen_width: Option<i64>,
    screen_height: Option<i64>,
    refresh_hz: Option<i64>,
    max_fps: Option<i64>,
}

fn machine_overrides(table: &toml::Table, machine: &str) -> Result<Overrides, String> {
    let Some(section) = table
        .get("machine")
        .and_then(|m| m.as_table())
        .and_then(|m| m.get(machine))
    else {
        println!("note: no [machine.{machine}] in profile.toml; applying the profile as captured");
        return Ok(Overrides::default());
    };
    let section = section
        .as_table()
        .ok_or_else(|| format!("[machine.{machine}] is not a table"))?;
    let mut o = Overrides::default();
    for (key, value) in section {
        let positive_int = || match value.as_integer() {
            Some(n) if n > 0 => Ok(n),
            _ => Err(format!("[machine.{machine}] {key}: expected a positive integer")),
        };
        match key.as_str() {
            "ssaa" => {
                let v = value
                    .as_float()
                    .or_else(|| value.as_integer().map(|n| n as f64))
                    .filter(|v| v.is_finite() && *v > 0.0)
                    .ok_or_else(|| format!("[machine.{machine}] ssaa: expected a positive number"))?;
                o.ssaa = Some(v);
            }
            "screen_width" => o.screen_width = Some(positive_int()?),
            "screen_height" => o.screen_height = Some(positive_int()?),
            "refresh_hz" => o.refresh_hz = Some(positive_int()?),
            "max_fps" => o.max_fps = Some(positive_int()?),
            _ => return Err(format!("[machine.{machine}] {key}: unknown override")),
        }
    }
    Ok(o)
}

/// (file, element, new text) for every override, per the plan's mapping.
fn edits(files: &BTreeMap<String, Vec<u8>>, o: &Overrides) -> Result<Vec<(String, String, String)>, String> {
    let mut out = Vec::new();
    if let Some(ssaa) = o.ssaa {
        // Elite reads the highest Custom.<Major>.<Minor>.fxcfg; old versions
        // left behind by updates are not touched.
        let mut best: Option<((u32, u32), &String)> = None;
        for rel in files.keys() {
            let Some(name) = rel.strip_prefix("Graphics/Custom.") else {
                continue;
            };
            if !name.contains("fxcfg") {
                continue;
            }
            let Some(version) = fxcfg_version(name) else {
                eprintln!("notice: ignoring {rel}: not Custom.<Major>.<Minor>.fxcfg");
                continue;
            };
            if best.is_none_or(|(v, _)| version > v) {
                best = Some((version, rel));
            }
        }
        let Some((_, rel)) = best else {
            return Err("ssaa: profile has no Graphics/Custom.<Major>.<Minor>.fxcfg".into());
        };
        println!("ssaa target: {rel}");
        out.push((rel.clone(), "SSAAMultiplier".into(), format!("{ssaa:.6}")));
    }
    let mut display = |tag: &str, value: String| out.push((DISPLAY.to_owned(), tag.to_owned(), value));
    if let Some(w) = o.screen_width {
        display("ScreenWidth", w.to_string());
    }
    if let Some(h) = o.screen_height {
        display("ScreenHeight", h.to_string());
    }
    if let Some(hz) = o.refresh_hz {
        display("DX11_RefreshRateNumerator", hz.to_string());
        display("DX11_RefreshRateDenominator", "1".into());
    }
    if let Some(fps) = o.max_fps {
        display("MaxFramesPerSecond", fps.to_string());
        display("LimitFrameRate", "true".into());
    }
    for (rel, tag, _) in &out {
        if !files.contains_key(rel) {
            return Err(format!("{tag}: profile has no {rel}"));
        }
    }
    Ok(out)
}

/// Error when the Options dir already holds a `Custom.<Major>.<Minor>.fxcfg`
/// newer than the profile's ssaa target: Elite would read that one, and the
/// override would land in a file the game ignores.
fn refuse_newer_fxcfg(options: &Path, target: &str) -> Result<(), String> {
    let Some(mine) = target.strip_prefix("Graphics/Custom.").and_then(fxcfg_version) else {
        return Ok(());
    };
    let Ok(entries) = fs::read_dir(options.join("Graphics")) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(v) = name.strip_prefix("Custom.").and_then(fxcfg_version)
            && v > mine
            && entry.file_type().is_ok_and(|t| t.is_file())
        {
            return Err(format!(
                "ssaa: {} has Graphics/{name}, newer than the profile's {target}; Elite reads the newer one. Capture from a machine on the same game version",
                options.display()
            ));
        }
    }
    Ok(())
}

/// `4.4.fxcfg` (the part after `Custom.`) -> `(4, 4)`; anything else -> None.
fn fxcfg_version(name: &str) -> Option<(u32, u32)> {
    let (major, minor) = name.strip_suffix(".fxcfg")?.split_once('.')?;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(major) || !digits(minor) {
        return None;
    }
    Some((major.parse().ok()?, minor.parse().ok()?))
}

/// Replace the text of the single `<tag>...</tag>` in `text`, leaving every
/// other byte unchanged. Absent or repeated element is an error.
fn replace_element(text: &str, tag: &str, value: &str) -> Result<String, String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut found = text.match_indices(&open);
    let Some((start, _)) = found.next() else {
        return Err(format!("<{tag}> not found; ed-loadout replaces elements, it never inserts"));
    };
    if found.next().is_some() {
        return Err(format!("<{tag}> occurs more than once"));
    }
    // The single literal match must be a real child of the root, not text
    // inside a comment or CDATA.
    let doc = roxmltree::Document::parse(text).map_err(|e| format!("not well-formed XML: {e}"))?;
    let real = doc
        .root_element()
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == tag)
        .count();
    if real != 1 {
        return Err(format!("<{tag}> not found as an element; ed-loadout replaces elements, it never inserts"));
    }
    let body = start + open.len();
    let end = text[body..]
        .find(&close)
        .map(|i| body + i)
        .ok_or_else(|| format!("<{tag}> has no {close}"))?;
    if text[body..end].contains('<') {
        return Err(format!("<{tag}> is not a text element"));
    }
    Ok(format!("{}{value}{}", &text[..body], &text[end..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusal_path_exits_2_and_writes_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let options = tmp.path().join("Options");
        fs::create_dir_all(&options).unwrap();
        // The profile does not even exist: refusal comes before any read.
        let code = run_with(&tmp.path().join("nope"), &options, "laptop", true).unwrap();
        assert_eq!(code, 2);
        assert_eq!(fs::read_dir(&options).unwrap().count(), 0);
    }

    #[test]
    fn replace_element_keeps_every_other_byte() {
        let s = "<R>\r\n\t<A>1.500000</A>\r\n\t<B>x</B>\r\n</R>\r\n";
        assert_eq!(
            replace_element(s, "A", "1.250000").unwrap(),
            "<R>\r\n\t<A>1.250000</A>\r\n\t<B>x</B>\r\n</R>\r\n"
        );
        assert!(replace_element(s, "C", "1").unwrap_err().contains("never inserts"));
        assert!(replace_element("<A/>", "A", "1").is_err());
        assert!(replace_element("<A>1</A><A>2</A>", "A", "3").is_err());
        let commented = "<R>\n\t<!-- <A>1</A> -->\n\t<B>x</B>\n</R>\n";
        assert!(replace_element(commented, "A", "2").unwrap_err().contains("never inserts"));
    }

    #[test]
    fn fxcfg_version_parses_major_minor_only() {
        assert_eq!(fxcfg_version("4.4.fxcfg"), Some((4, 4)));
        assert_eq!(fxcfg_version("4.10.fxcfg"), Some((4, 10)));
        assert!(fxcfg_version("4.10.fxcfg") > fxcfg_version("4.9.fxcfg"));
        for no in ["fxcfg.bak", "fxcfg", "4.fxcfg", "4.4.4.fxcfg", "a.4.fxcfg", "4.4.fxcfg.bak", "+4.4.fxcfg"] {
            assert_eq!(fxcfg_version(no), None, "{no}");
        }
    }

    #[test]
    fn overrides_parse_and_reject_unknown_keys() {
        let t: toml::Table = "[machine.laptop]\nssaa = 1\nmax_fps = 120\n".parse().unwrap();
        let o = machine_overrides(&t, "laptop").unwrap();
        assert_eq!(o.ssaa, Some(1.0));
        assert_eq!(o.max_fps, Some(120));
        let t: toml::Table = "[machine.laptop]\nsaa = 1.0\n".parse().unwrap();
        assert!(machine_overrides(&t, "laptop").is_err());
        let t: toml::Table = "[machine.laptop]\nmax_fps = -1\n".parse().unwrap();
        assert!(machine_overrides(&t, "laptop").is_err());
        assert_eq!(machine_overrides(&t, "desk").unwrap(), Overrides::default());
    }
}
