//! `apply`: back up, write the profile files, apply `[machine.<name>]`
//! overrides by replacing element text in place. Never inserts an element.

use crate::binds;
use crate::bundle::{backup_stamp, join_rel, scan_profile};
use crate::facts::{self, Facts};
use crate::tiers::{self, OverrideTier, fmt_ssaa};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const DISPLAY: &str = "Graphics/DisplaySettings.xml";
const PROCESS: &str = "EliteDangerous64.exe";
const OVERRIDE_XML: &str = "Graphics/GraphicsConfigurationOverride.xml";
const T1_XML: &[u8] = include_bytes!("../tiers/t1.xml");
const T2_XML: &[u8] = include_bytes!("../tiers/t2.xml");

/// `LOADOUT_FAKE_GAME_RUNNING=1` reads as "the game is running", in debug
/// builds only: the refusal path and the app's red state without the game.
fn fake_running() -> bool {
    cfg!(debug_assertions) && std::env::var_os("LOADOUT_FAKE_GAME_RUNNING").is_some_and(|v| v == "1")
}

/// True when Elite Dangerous is running. Windows only; elsewhere prints a
/// notice and returns false.
#[cfg(windows)]
pub fn game_running() -> bool {
    use std::os::windows::process::CommandExt;
    // No console window: the desktop app polls this.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    if fake_running() {
        return true;
    }
    let filter = format!("IMAGENAME eq {PROCESS}");
    match std::process::Command::new("tasklist")
        .args(["/FI", &filter, "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .to_ascii_lowercase()
            .contains(&PROCESS.to_ascii_lowercase()),
        Err(e) => {
            note!("notice: could not run tasklist ({e}); game-running check skipped");
            false
        }
    }
}

#[cfg(not(windows))]
pub fn game_running() -> bool {
    if fake_running() {
        return true;
    }
    note!("notice: game-running check ({PROCESS}) is Windows-only; skipped on this platform");
    false
}

pub fn run(
    profile: &Path,
    options: &Path,
    machine: Option<&str>,
    facts_file: Option<&Path>,
    strict_binds: bool,
) -> Result<i32, String> {
    run_with(profile, options, machine, facts_file, strict_binds, game_running())
}

/// `running` is injected so the refusal path is testable off Windows.
pub fn run_with(
    profile: &Path,
    options: &Path,
    machine: Option<&str>,
    facts_file: Option<&Path>,
    strict_binds: bool,
    running: bool,
) -> Result<i32, String> {
    Ok(match apply_with(profile, options, machine, facts_file, strict_binds, running)? {
        Outcome::Refused(code) => code,
        Outcome::Written { .. } => 0,
    })
}

/// What an apply did.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    /// Nothing was written; the CLI exit code (2 game running, 1 strict binds).
    Refused(i32),
    /// The backup folder (always created, holds `manifest.toml`) and the
    /// relative paths written.
    Written { backup: PathBuf, files: Vec<String> },
}

pub fn apply_with(
    profile: &Path,
    options: &Path,
    machine: Option<&str>,
    facts_file: Option<&Path>,
    strict_binds: bool,
    running: bool,
) -> Result<Outcome, String> {
    if running {
        note!("refusing: {PROCESS} is running; quit the game and apply again");
        return Ok(Outcome::Refused(2));
    }

    let facts = facts::load(facts_file)?;
    if facts.is_none() {
        note!("notice: {}", facts::NO_FACTS);
        if strict_binds {
            return Err(format!("--strict-binds needs device facts; {}", facts::NO_FACTS));
        }
    }
    let mut plan = plan_files(profile, machine, facts.as_ref())?;
    if strict_binds && !plan.unconnected.is_empty() {
        note!("refusing: --strict-binds and {} bound device(s) not connected; nothing written", plan.unconnected.len());
        return Ok(Outcome::Refused(1));
    }
    plan_edits(&mut plan, options)?;
    let backup = write(&plan, options)?;
    Ok(Outcome::Written { backup, files: plan.files.into_keys().collect() })
}

/// One effective key: its value, where it comes from (`derived` or
/// `[machine.<name>]`) and why.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub key: &'static str,
    pub value: String,
    pub source: String,
    pub reason: String,
}

/// Everything an apply would do, computed without writing anything.
#[derive(Debug)]
pub struct Plan {
    /// Final bytes of every file to write, by relative path.
    pub files: BTreeMap<String, Vec<u8>>,
    /// The effective keys, in print order.
    pub rows: Vec<Row>,
    /// (file, element, new text) for every element edit.
    pub edits: Vec<(String, String, String)>,
    /// Bound devices that are not connected: (GUID, bindings). Empty without facts.
    pub unconnected: Vec<(String, usize)>,
    overrides: Overrides,
}

/// The whole plan for `profile` into `options`: the same steps and the same
/// lines as `apply`, up to the point where it would back up and write.
pub fn plan(profile: &Path, options: &Path, machine: Option<&str>, facts: Option<&Facts>) -> Result<Plan, String> {
    let mut plan = plan_files(profile, machine, facts)?;
    plan_edits(&mut plan, options)?;
    Ok(plan)
}

/// First half: profile.toml, the effective keys, the profile's files, the
/// binds check.
fn plan_files(profile: &Path, machine: Option<&str>, facts: Option<&Facts>) -> Result<Plan, String> {
    let toml_path = profile.join("profile.toml");
    let table: toml::Table = fs::read_to_string(&toml_path)
        .map_err(|e| format!("{}: {e}", toml_path.display()))?
        .parse()
        .map_err(|e| format!("{}: {e}", toml_path.display()))?;
    let (overrides, rows) = effective_overrides(&table, machine, facts)?;

    // Final contents of every file, overrides applied, before anything is
    // written: a missing element aborts with the options dir untouched.
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for rel in scan_profile(profile)? {
        let p = join_rel(&profile.join("files"), &rel);
        let bytes = fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        files.insert(rel, bytes);
    }
    let unconnected = match facts {
        Some(f) => binds_check(&files, &f.devices),
        None => {
            note!("binds check skipped (no device facts)");
            Vec::new()
        }
    };
    Ok(Plan { files, rows, edits: Vec::new(), unconnected, overrides })
}

/// Second half: the override XML choice and the element edits.
fn plan_edits(plan: &mut Plan, options: &Path) -> Result<(), String> {
    let files = &mut plan.files;
    match plan.overrides.override_tier.unwrap_or(OverrideTier::Captured) {
        OverrideTier::Captured => {}
        OverrideTier::None => {
            files.remove(OVERRIDE_XML);
            say!("override_tier none: {OVERRIDE_XML} in the Options dir is left alone");
        }
        t => {
            let bytes = if t == OverrideTier::T1 { T1_XML } else { T2_XML };
            files.insert(OVERRIDE_XML.to_owned(), bytes.to_vec());
            say!("override_tier {}: writing tiers/{}.xml as {OVERRIDE_XML}", t.as_str(), t.as_str());
        }
    }
    let edits = edits(files, &plan.overrides)?;
    for (rel, tag, value) in &edits {
        if tag == "SSAAMultiplier" || tag == "TerrainQuality" {
            refuse_newer_fxcfg(options, rel)?;
        }
        let bytes = files.get_mut(rel).ok_or_else(|| format!("{tag}: profile has no {rel}"))?;
        let text = std::str::from_utf8(bytes).map_err(|_| format!("{rel}: not UTF-8"))?;
        let new = replace_element(text, tag, value).map_err(|e| format!("{rel}: {e}"))?;
        say!("override {rel}: {tag} = {value}");
        *bytes = new.into_bytes();
    }
    plan.edits = edits;
    Ok(())
}

/// Back up every file that will be overwritten, record all of them in the
/// backup's `manifest.toml`, then write. Returns the backup folder, which is
/// created even when nothing pre-existed: the manifest is what lets `restore`
/// remove the files this apply added.
fn write(plan: &Plan, options: &Path) -> Result<PathBuf, String> {
    let entries: Vec<(String, bool)> =
        plan.files.keys().map(|rel| (rel.clone(), join_rel(options, rel).exists())).collect();
    let backup_root = new_backup(options, "apply", &entries)?;
    let backed_up = entries.iter().filter(|(_, existed)| *existed).count();
    say!("backup: {} ({backed_up} files)", backup_root.display());

    for (rel, bytes) in &plan.files {
        let target = join_rel(options, rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::write(&target, bytes).map_err(|e| format!("{}: {e}", target.display()))?;
        say!("wrote {rel}");
    }
    Ok(backup_root)
}

pub const BACKUP_DIR: &str = "_loadout-backup";
pub const MANIFEST: &str = "manifest.toml";

/// `manifest.toml` of one backup folder.
#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    /// `apply`, or `restore` for the copy a restore takes before it overwrites.
    pub kind: String,
    pub created_at: String,
    /// (relative path, existed before). `true`: its old bytes are in the
    /// backup folder. `false`: the operation added it.
    pub files: Vec<(String, bool)>,
}

impl Manifest {
    pub fn to_toml(&self) -> String {
        let q = |s: &str| toml::Value::String(s.to_owned()).to_string();
        let mut out = String::from(
            "# LOADOUT backup manifest. [files]: true = the file existed before and its old\n# bytes are in this folder; false = it did not exist and was added.\n",
        );
        out.push_str(&format!("kind = {}\ncreated_at = {}\n\n[files]\n", q(&self.kind), q(&self.created_at)));
        for (rel, existed) in &self.files {
            out.push_str(&format!("{} = {existed}\n", q(rel)));
        }
        out
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let t: toml::Table = text.parse().map_err(|e: toml::de::Error| e.to_string())?;
        let string = |k: &str| t.get(k).and_then(|v| v.as_str()).map(str::to_owned).ok_or(format!("{k}: expected a string"));
        let mut files = Vec::new();
        for (rel, v) in t.get("files").and_then(|v| v.as_table()).ok_or("[files]: missing")? {
            let safe = rel.split('/').all(|part| !part.is_empty() && part != "." && part != ".." && !part.contains(['\\', ':']));
            if !safe {
                return Err(format!("[files] {rel:?}: not a relative path inside the Options dir"));
            }
            files.push((rel.clone(), v.as_bool().ok_or(format!("[files] {rel}: expected true or false"))?));
        }
        Ok(Self { kind: string("kind")?, created_at: string("created_at")?, files })
    }

    pub fn read(backup: &Path) -> Result<Self, String> {
        let path = backup.join(MANIFEST);
        let text = fs::read_to_string(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                format!("{}: no {MANIFEST} (a backup made before v0.3); copy its files back by hand", backup.display())
            } else {
                format!("{}: {e}", path.display())
            }
        })?;
        Self::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// Create `<options>/_loadout-backup/<stamp>`, copy into it every entry that
/// exists, and write the manifest.
fn new_backup(options: &Path, kind: &str, entries: &[(String, bool)]) -> Result<PathBuf, String> {
    let parent = options.join(BACKUP_DIR);
    fs::create_dir_all(&parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    let root = parent.join(backup_stamp());
    // create_dir, not create_dir_all: two backups never share a folder.
    fs::create_dir(&root).map_err(|e| format!("{}: {e}", root.display()))?;
    for (rel, existed) in entries {
        if *existed {
            let bak = join_rel(&root, rel);
            if let Some(parent) = bak.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
            }
            fs::copy(join_rel(options, rel), &bak).map_err(|e| format!("{}: {e}", bak.display()))?;
        }
    }
    let manifest = Manifest {
        kind: kind.to_owned(),
        created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        files: entries.to_vec(),
    };
    let path = root.join(MANIFEST);
    fs::write(&path, manifest.to_toml()).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(root)
}

/// The newest backup folder an apply made (restore's own copies are skipped),
/// or `None`.
pub fn last_backup(options: &Path) -> Option<PathBuf> {
    let mut stamps: Vec<PathBuf> = fs::read_dir(options.join(BACKUP_DIR))
        .ok()?
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.path())
        .collect();
    // Stamps are UTC `%Y%m%d-%H%M%S-%3f`: name order is time order.
    stamps.sort();
    stamps.into_iter().rev().find(|p| Manifest::read(p).is_ok_and(|m| m.kind == "apply"))
}

/// Undo an apply from its backup folder (`None`: the last one): files that
/// existed before are put back byte-for-byte, files the apply added are
/// removed. What is about to be replaced is copied to a new backup folder
/// (kind `restore`) first. Returns the backup that was restored.
pub fn restore(options: &Path, backup: Option<&Path>) -> Result<PathBuf, String> {
    restore_with(options, backup, game_running())
}

pub fn restore_with(options: &Path, backup: Option<&Path>, running: bool) -> Result<PathBuf, String> {
    if running {
        return Err(format!("refusing: {PROCESS} is running; quit the game and restore again"));
    }
    let backup = match backup {
        Some(p) => p.to_path_buf(),
        None => last_backup(options)
            .ok_or_else(|| format!("{}: no backup to restore", options.join(BACKUP_DIR).display()))?,
    };
    let manifest = Manifest::read(&backup)?;
    // Everything is checked before anything is touched.
    for (rel, existed) in &manifest.files {
        let bak = join_rel(&backup, rel);
        if *existed && !bak.is_file() {
            return Err(format!("{}: listed in {MANIFEST} but missing from the backup; nothing restored", bak.display()));
        }
    }
    let now: Vec<(String, bool)> =
        manifest.files.iter().map(|(rel, _)| (rel.clone(), join_rel(options, rel).exists())).collect();
    let safety = new_backup(options, "restore", &now)?;
    say!("backup: {} ({} files, the state before this restore)", safety.display(), now.iter().filter(|(_, e)| *e).count());

    for (rel, existed) in &manifest.files {
        let target = join_rel(options, rel);
        if *existed {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
            }
            fs::copy(join_rel(&backup, rel), &target).map_err(|e| format!("{}: {e}", target.display()))?;
            say!("restored {rel}");
        } else if target.exists() {
            fs::remove_file(&target).map_err(|e| format!("{}: {e}", target.display()))?;
            say!("removed {rel} (added by the apply)");
        }
    }
    say!("restore: {} put back", backup.display());
    Ok(backup)
}

/// Every device bound in the profile's highest `Bindings/Custom.4.<n>.binds`
/// (the file about to be written) that is not in `devices`, with a warning
/// line each: (GUID, bindings).
fn binds_check(files: &BTreeMap<String, Vec<u8>>, devices: &[String]) -> Vec<(String, usize)> {
    let Some(rel) = binds::highest_custom4(files.keys()) else {
        say!("binds check skipped (profile has no Bindings/Custom.4.<n>.binds)");
        return Vec::new();
    };
    say!("binds check: {rel} against {} connected device(s)", devices.len());
    let bound = binds::bound_devices(&String::from_utf8_lossy(&files[rel]));
    let mut out = Vec::new();
    for (guid, count) in bound {
        if !devices.iter().any(|d| d.eq_ignore_ascii_case(&guid)) {
            let plural = if count == 1 { "binding" } else { "bindings" };
            say!("warning: bound device {guid} ({count} {plural}) is not connected");
            out.push((guid, count));
        }
    }
    out
}

#[derive(Debug, Default, PartialEq)]
struct Overrides {
    ssaa: Option<f64>,
    screen_width: Option<i64>,
    screen_height: Option<i64>,
    refresh_hz: Option<i64>,
    max_fps: Option<i64>,
    terrain_quality: Option<i64>,
    override_tier: Option<OverrideTier>,
}

impl Overrides {
    /// (key, value) for every key that is set, in `[machine]` key names.
    fn entries(&self) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        let ints = [
            ("screen_width", self.screen_width),
            ("screen_height", self.screen_height),
            ("refresh_hz", self.refresh_hz),
            ("max_fps", self.max_fps),
        ];
        out.extend(ints.into_iter().filter_map(|(k, v)| v.map(|v| (k, v.to_string()))));
        out.extend(self.ssaa.map(|v| ("ssaa", fmt_ssaa(v))));
        out.extend(self.terrain_quality.map(|v| ("terrain_quality", v.to_string())));
        out.extend(self.override_tier.map(|v| ("override_tier", v.as_str().to_owned())));
        out
    }
}

/// Derived values from the facts (if any), then `[machine.<name>]` (name =
/// `--machine`, else the facts' hostname) key by key on top. Every effective
/// key is printed with its source.
fn effective_overrides(
    table: &toml::Table,
    machine: Option<&str>,
    facts: Option<&Facts>,
) -> Result<(Overrides, Vec<Row>), String> {
    let derived = facts.map(tiers::derive);
    if let Some(d) = &derived {
        d.print();
    }
    let name = machine.map(str::to_owned).or_else(|| facts.map(|f| f.hostname.clone()));
    let user = match &name {
        Some(n) => machine_overrides(table, n)?,
        None => None,
    };
    let as_captured = if derived.is_some() { "using the derived values" } else { "applying the profile as captured" };
    match &name {
        Some(n) if user.is_none() => say!("note: no [machine.{n}] in profile.toml; {as_captured}"),
        None => say!("note: no --machine and no device facts; {as_captured}"),
        _ => {}
    }
    let d = derived.as_ref().map(|d| Overrides {
        ssaa: Some(d.ssaa),
        screen_width: Some(d.screen_width),
        screen_height: Some(d.screen_height),
        refresh_hz: Some(d.refresh_hz),
        max_fps: Some(d.max_fps),
        terrain_quality: Some(d.terrain_quality),
        override_tier: Some(d.override_tier),
    });
    let (d, u) = (d.unwrap_or_default(), user.unwrap_or_default());
    let o = Overrides {
        ssaa: u.ssaa.or(d.ssaa),
        screen_width: u.screen_width.or(d.screen_width),
        screen_height: u.screen_height.or(d.screen_height),
        refresh_hz: u.refresh_hz.or(d.refresh_hz),
        max_fps: u.max_fps.or(d.max_fps),
        terrain_quality: u.terrain_quality.or(d.terrain_quality),
        override_tier: u.override_tier.or(d.override_tier),
    };
    let section = format!("[machine.{}]", name.unwrap_or_default());
    let mine: Vec<&str> = u.entries().into_iter().map(|(k, _)| k).collect();
    let derived_line = |key: &str| derived.as_ref().and_then(|d| d.lines.iter().find(|l| l.0 == key));
    let mut rows = Vec::new();
    for (key, value) in o.entries() {
        let source = if mine.contains(&key) { section.as_str() } else { "derived" };
        say!("effective {key} = {value}  ({source})");
        let reason = match (source, derived_line(key)) {
            ("derived", Some(l)) => l.2.clone(),
            (_, Some(l)) => format!("set in profile.toml; overrides the derived {}", l.1),
            _ => "set in profile.toml".to_owned(),
        };
        rows.push(Row { key, value, source: source.to_owned(), reason });
    }
    Ok((o, rows))
}

/// The `[machine.<machine>]` keys, or `None` when there is no such section.
fn machine_overrides(table: &toml::Table, machine: &str) -> Result<Option<Overrides>, String> {
    let Some(section) = table
        .get("machine")
        .and_then(|m| m.as_table())
        .and_then(|m| m.get(machine))
    else {
        return Ok(None);
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
            "terrain_quality" => match value.as_integer() {
                Some(n @ (3 | 4)) => o.terrain_quality = Some(n),
                _ => return Err(format!("[machine.{machine}] terrain_quality: expected 3 or 4")),
            },
            "override_tier" => {
                let t = value.as_str().and_then(OverrideTier::parse).ok_or_else(|| {
                    format!("[machine.{machine}] override_tier: expected \"t1\", \"t2\", \"captured\" or \"none\"")
                })?;
                o.override_tier = Some(t);
            }
            _ => return Err(format!("[machine.{machine}] {key}: unknown override")),
        }
    }
    Ok(Some(o))
}

/// (file, element, new text) for every override, per the plan's mapping.
fn edits(files: &BTreeMap<String, Vec<u8>>, o: &Overrides) -> Result<Vec<(String, String, String)>, String> {
    let mut out = Vec::new();
    if o.ssaa.is_some() || o.terrain_quality.is_some() {
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
                note!("notice: ignoring {rel}: not Custom.<Major>.<Minor>.fxcfg");
                continue;
            };
            if best.is_none_or(|(v, _)| version > v) {
                best = Some((version, rel));
            }
        }
        let key = if o.ssaa.is_some() { "ssaa" } else { "terrain_quality" };
        let Some((_, rel)) = best else {
            return Err(format!("{key}: profile has no Graphics/Custom.<Major>.<Minor>.fxcfg"));
        };
        say!("fxcfg target: {rel}");
        if let Some(ssaa) = o.ssaa {
            say!("ssaa target: {rel}");
            out.push((rel.clone(), "SSAAMultiplier".into(), format!("{ssaa:.6}")));
        }
        if let Some(tq) = o.terrain_quality {
            out.push((rel.clone(), "TerrainQuality".into(), tq.to_string()));
        }
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
/// newer than the profile's fxcfg target: Elite would read that one, and the
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
                "fxcfg: {} has Graphics/{name}, newer than the profile's {target}; Elite reads the newer one. Capture from a machine on the same game version",
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
        let code = run_with(&tmp.path().join("nope"), &options, Some("laptop"), None, false, true).unwrap();
        assert_eq!(code, 2);
        assert_eq!(fs::read_dir(&options).unwrap().count(), 0);
    }

    /// An Options dir holding two files and a profile that rewrites one of
    /// them and adds a third.
    fn options_and_profile(tmp: &Path) -> (PathBuf, PathBuf) {
        let options = tmp.join("Options");
        fs::create_dir_all(options.join("Graphics")).unwrap();
        fs::create_dir_all(options.join("Bindings")).unwrap();
        fs::write(options.join("Graphics/Settings.xml"), b"<R>\r\n\t<A>old</A>\r\n</R>\r\n").unwrap();
        fs::write(options.join("Bindings/Mine.binds"), b"untouched").unwrap();
        let profile = tmp.join("profile");
        fs::create_dir_all(profile.join("files/Graphics")).unwrap();
        fs::write(profile.join("profile.toml"), "source_machine = \"desk\"\n").unwrap();
        fs::write(profile.join("files/Graphics/Settings.xml"), b"<R>\n\t<A>new</A>\n</R>\n").unwrap();
        fs::write(profile.join("files/Graphics/Custom.4.2.fxcfg"), b"<F>\n</F>\n").unwrap();
        (options, profile)
    }

    /// Every file under `dir` except `_loadout-backup`, as (relative path, bytes).
    fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
        let mut out = BTreeMap::new();
        for e in walkdir::WalkDir::new(dir).into_iter().flatten().filter(|e| e.file_type().is_file()) {
            let rel = e.path().strip_prefix(dir).unwrap().to_string_lossy().replace('\\', "/");
            if !rel.starts_with(BACKUP_DIR) {
                out.insert(rel, fs::read(e.path()).unwrap());
            }
        }
        out
    }

    #[test]
    fn manifest_round_trip() {
        let m = Manifest {
            kind: "apply".into(),
            created_at: "2026-11-25T19:00:00Z".into(),
            files: vec![("Bindings/Custom.4.2.binds".into(), false), ("Graphics/Display \"x\".xml".into(), true)],
        };
        assert_eq!(Manifest::parse(&m.to_toml()).unwrap(), m);
        for bad in ["\"../outside.xml\" = true", "\"C:/x\" = true", "\"Graphics\\\\x\" = true", "\"/abs\" = true", "\"a\" = 1"] {
            let text = format!("kind = \"apply\"\ncreated_at = \"t\"\n\n[files]\n{bad}\n");
            assert!(Manifest::parse(&text).is_err(), "{bad}");
        }
        assert!(Manifest::parse("kind = \"apply\"\ncreated_at = \"t\"\n").unwrap_err().contains("[files]"));
    }

    #[test]
    fn apply_always_writes_a_backup_folder_with_a_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        let (options, profile) = options_and_profile(tmp.path());
        let Outcome::Written { backup, files } = apply_with(&profile, &options, None, None, false, false).unwrap() else {
            panic!("refused");
        };
        assert_eq!(files, ["Graphics/Custom.4.2.fxcfg", "Graphics/Settings.xml"]);
        assert_eq!(backup.parent().unwrap(), options.join(BACKUP_DIR));
        let m = Manifest::read(&backup).unwrap();
        assert_eq!(m.kind, "apply");
        assert_eq!(m.files, [("Graphics/Custom.4.2.fxcfg".to_owned(), false), ("Graphics/Settings.xml".to_owned(), true)]);
        assert_eq!(fs::read(backup.join("Graphics/Settings.xml")).unwrap(), b"<R>\r\n\t<A>old</A>\r\n</R>\r\n");
        assert!(!backup.join("Graphics/Custom.4.2.fxcfg").exists());
        assert_eq!(last_backup(&options), Some(backup));

        // Into an empty Options dir: still a backup folder, holding only the manifest.
        let empty = tmp.path().join("Empty");
        fs::create_dir_all(&empty).unwrap();
        let Outcome::Written { backup, .. } = apply_with(&profile, &empty, None, None, false, false).unwrap() else {
            panic!("refused");
        };
        let names: Vec<String> = fs::read_dir(&backup).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(names, [MANIFEST]);
        assert!(Manifest::read(&backup).unwrap().files.iter().all(|(_, existed)| !existed));
    }

    #[test]
    fn restore_puts_back_byte_for_byte_and_removes_what_the_apply_added() {
        let tmp = tempfile::tempdir().unwrap();
        let (options, profile) = options_and_profile(tmp.path());
        let before = tree(&options);
        apply_with(&profile, &options, None, None, false, false).unwrap();
        let applied = tree(&options);
        assert_ne!(applied, before);
        assert!(options.join("Graphics/Custom.4.2.fxcfg").exists());

        let restored = restore_with(&options, None, false).unwrap();
        assert_eq!(tree(&options), before, "same file set, same bytes");
        assert!(!options.join("Graphics/Custom.4.2.fxcfg").exists());
        // The state the restore replaced is kept too, as its own backup, and
        // the last APPLY backup is still the one just restored.
        let kinds: Vec<String> = fs::read_dir(options.join(BACKUP_DIR))
            .unwrap()
            .map(|e| Manifest::read(&e.unwrap().path()).unwrap().kind)
            .collect();
        assert_eq!(kinds.iter().filter(|k| *k == "restore").count(), 1, "{kinds:?}");
        assert_eq!(last_backup(&options), Some(restored.clone()));
        let safety = fs::read_dir(options.join(BACKUP_DIR)).unwrap().map(|e| e.unwrap().path()).find(|p| *p != restored).unwrap();
        assert_eq!(fs::read(safety.join("Graphics/Settings.xml")).unwrap(), applied["Graphics/Settings.xml"]);

        // Refusals touch nothing: game running, a backup with no manifest, a
        // manifest naming a file the backup does not hold.
        assert!(restore_with(&options, None, true).unwrap_err().contains("is running"));
        let old = options.join(BACKUP_DIR).join("20261121-000000-000");
        fs::create_dir_all(&old).unwrap();
        assert!(restore_with(&options, Some(&old), false).unwrap_err().contains("before v0.3"));
        fs::remove_file(restored.join("Graphics/Settings.xml")).unwrap();
        assert!(restore_with(&options, Some(&restored), false).unwrap_err().contains("nothing restored"));
        assert_eq!(tree(&options), before);
    }

    #[test]
    fn plan_writes_nothing_and_lists_what_apply_would_do() {
        let tmp = tempfile::tempdir().unwrap();
        let (options, profile) = options_and_profile(tmp.path());
        fs::write(profile.join("profile.toml"), "source_machine = \"desk\"\n\n[machine.lap]\nmax_fps = 90\n").unwrap();
        fs::write(profile.join("files/Graphics/DisplaySettings.xml"), "<D>\n\t<MaxFramesPerSecond>144</MaxFramesPerSecond>\n\t<LimitFrameRate>false</LimitFrameRate>\n</D>\n").unwrap();
        let before = tree(&options);
        let p = plan(&profile, &options, Some("lap"), None).unwrap();
        assert_eq!(tree(&options), before);
        assert!(!options.join(BACKUP_DIR).exists());
        assert_eq!(p.files.keys().collect::<Vec<_>>(), ["Graphics/Custom.4.2.fxcfg", "Graphics/DisplaySettings.xml", "Graphics/Settings.xml"]);
        assert_eq!(p.rows, [Row { key: "max_fps", value: "90".into(), source: "[machine.lap]".into(), reason: "set in profile.toml".into() }]);
        assert_eq!(p.edits.len(), 2);
        assert!(String::from_utf8_lossy(&p.files["Graphics/DisplaySettings.xml"]).contains("<MaxFramesPerSecond>90<"));
        assert!(p.unconnected.is_empty());
    }

    /// The fake is read in debug builds only (plan v0.3 criterion 5).
    #[test]
    fn fake_game_running_env_is_debug_only() {
        // SAFETY: no other test reads or writes this variable.
        unsafe { std::env::set_var("LOADOUT_FAKE_GAME_RUNNING", "1") };
        let faked = fake_running();
        unsafe { std::env::remove_var("LOADOUT_FAKE_GAME_RUNNING") };
        assert_eq!(faked, cfg!(debug_assertions));
        assert!(!fake_running());
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
        let o = machine_overrides(&t, "laptop").unwrap().unwrap();
        assert_eq!(o.ssaa, Some(1.0));
        assert_eq!(o.max_fps, Some(120));
        let t: toml::Table = "[machine.laptop]\nsaa = 1.0\n".parse().unwrap();
        assert!(machine_overrides(&t, "laptop").is_err());
        let t: toml::Table = "[machine.laptop]\nterrain_quality = 3\noverride_tier = \"none\"\n".parse().unwrap();
        let o = machine_overrides(&t, "laptop").unwrap().unwrap();
        assert_eq!((o.terrain_quality, o.override_tier), (Some(3), Some(OverrideTier::None)));
        for bad in ["terrain_quality = 2", "terrain_quality = \"4\"", "override_tier = \"t3\""] {
            let t: toml::Table = format!("[machine.laptop]\n{bad}\n").parse().unwrap();
            assert!(machine_overrides(&t, "laptop").is_err(), "{bad}");
        }
        let t: toml::Table = "[machine.laptop]\nmax_fps = -1\n".parse().unwrap();
        assert!(machine_overrides(&t, "laptop").is_err());
        assert_eq!(machine_overrides(&t, "desk").unwrap(), None);
    }
}
