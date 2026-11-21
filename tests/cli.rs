//! End-to-end tests against fixtures/devmachine02, always via a temp copy.
//! Plan: claude/plans/2026-11-21-loadout-v0.md.

use assert_cmd::Command;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/devmachine02")
}

fn copy_tree(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let to = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &to);
        } else {
            fs::copy(entry.path(), &to).unwrap();
        }
    }
}

/// A temp copy of the fixture, so no test can touch the real corpus.
fn fixture_copy() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let opts = tmp.path().join("Options");
    copy_tree(&fixture(), &opts);
    (tmp, opts)
}

fn bin() -> Command {
    Command::cargo_bin("ed-loadout").unwrap()
}

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn capture(options: &Path, out: &Path, machine: &str) {
    bin()
        .args(["capture", "--options"])
        .arg(options)
        .arg("--out")
        .arg(out)
        .args(["--machine", machine])
        .assert()
        .success();
}

// ---- Criterion 1: capture -------------------------------------------------

#[test]
fn c1_capture_copies_byte_for_byte_and_records_sha256() {
    let (tmp, opts) = fixture_copy();
    // A CRLF file must survive with identical bytes.
    let crlf = b"<?xml version=\"1.0\" ?>\r\n<Root>\r\n\t<A>1</A>\r\n</Root>\r\n";
    fs::write(opts.join("Bindings/Crlf.4.0.binds"), crlf).unwrap();
    let profile = tmp.path().join("profile");
    capture(&opts, &profile, "devmachine02");

    let toml: toml::Table = fs::read_to_string(profile.join("profile.toml"))
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(toml["source_machine"].as_str(), Some("devmachine02"));
    let at = toml["captured_at"].as_str().unwrap();
    assert!(chrono::DateTime::parse_from_rfc3339(at).is_ok(), "{at}");
    let files = toml["files"].as_table().unwrap();

    let expected = [
        "Bindings/Crlf.4.0.binds",
        "Bindings/Custom.4.2.binds",
        "Bindings/StartPreset.4.start",
        "Graphics/Custom.4.4.fxcfg",
        "Graphics/DisplaySettings.xml",
        "Graphics/GraphicsConfigurationOverride.xml",
        "Graphics/Settings.xml",
        "Graphics/StartPreset.start",
    ];
    let mut got: Vec<&str> = files.keys().map(String::as_str).collect();
    got.sort();
    assert_eq!(got, expected);

    for rel in expected {
        let src = fs::read(opts.join(rel)).unwrap();
        let dst = fs::read(profile.join("files").join(rel)).unwrap();
        assert_eq!(src, dst, "{rel} bytes differ");
        assert_eq!(files[rel].as_str().unwrap(), sha(&src), "{rel} sha256");
    }
    assert_eq!(fs::read(profile.join("files/Bindings/Crlf.4.0.binds")).unwrap(), crlf);
    // Not in the file set.
    assert!(!profile.join("files/Bindings/StartPreset.4.start.broken").exists());
    assert!(!profile.join("files/Bindings/BindingLoadingErrors.log").exists());
}

#[test]
fn c1_capture_refuses_to_overwrite_a_profile() {
    let (tmp, opts) = fixture_copy();
    let profile = tmp.path().join("profile");
    capture(&opts, &profile, "a");
    bin()
        .args(["capture", "--options"])
        .arg(&opts)
        .arg("--out")
        .arg(&profile)
        .args(["--machine", "b"])
        .assert()
        .code(3);
}

#[test]
fn c1_capture_refuses_a_leftover_files_dir() {
    let (tmp, opts) = fixture_copy();
    let profile = tmp.path().join("profile");
    fs::create_dir_all(profile.join("files/Graphics")).unwrap();
    fs::write(profile.join("files/Graphics/Stale.xml"), "x").unwrap();
    bin()
        .args(["capture", "--options"])
        .arg(&opts)
        .arg("--out")
        .arg(&profile)
        .args(["--machine", "a"])
        .assert()
        .code(3);
    assert!(!profile.join("profile.toml").exists());
}

// ---- Criterion 2: diff ----------------------------------------------------

fn two_profiles() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let (tmp, opts) = fixture_copy();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    capture(&opts, &a, "devmachine02");
    capture(&opts, &b, "devmachine02");
    (tmp, a, b)
}

#[test]
fn c2_diff_identical_exits_0() {
    let (_tmp, a, b) = two_profiles();
    bin().arg("diff").arg(&a).arg(&b).assert().code(0).stdout("");
}

#[test]
fn c2_diff_ssaa_change_prints_one_element_line() {
    let (_tmp, a, b) = two_profiles();
    let fx = b.join("files/Graphics/Custom.4.4.fxcfg");
    let text = fs::read_to_string(&fx).unwrap();
    let old = "<SSAAMultiplier>1.500000</SSAAMultiplier>";
    assert!(text.contains(old));
    fs::write(&fx, text.replace(old, "<SSAAMultiplier>1.250000</SSAAMultiplier>")).unwrap();
    bin()
        .arg("diff")
        .arg(&a)
        .arg(&b)
        .assert()
        .code(1)
        .stdout("Graphics/Custom.4.4.fxcfg: SSAAMultiplier 1.500000 -> 1.250000\n");
}

#[test]
fn c2_diff_line_diff_and_one_side_only() {
    let (_tmp, a, b) = two_profiles();
    fs::copy(
        fixture().join("Bindings/StartPreset.4.start.broken"),
        b.join("files/Bindings/StartPreset.4.start"),
    )
    .unwrap();
    fs::remove_file(a.join("files/Graphics/GraphicsConfigurationOverride.xml")).unwrap();
    let out = bin().arg("diff").arg(&a).arg(&b).assert().code(1).get_output().stdout.clone();
    let out = String::from_utf8(out).unwrap();
    assert!(out.contains("only in b: Graphics/GraphicsConfigurationOverride.xml"), "{out}");
    assert!(out.contains("--- a/Bindings/StartPreset.4.start"), "{out}");
    assert!(out.contains("+++ b/Bindings/StartPreset.4.start"), "{out}");
    assert!(out.contains("-Custom\n"), "{out}");
    assert!(out.contains("+GenericJoystick\n"), "{out}");
}

// ---- Criterion 3: check-binds ---------------------------------------------

fn stdout_of(cmd: &mut Command, code: i32) -> String {
    let out = cmd.assert().code(code).get_output().stdout.clone();
    String::from_utf8(out).unwrap()
}

#[test]
fn c3a_devices_missing_xbox_pad_exits_1() {
    let (_tmp, opts) = fixture_copy();
    let out = stdout_of(
        bin()
            .arg("check-binds")
            .arg(opts.join("Bindings"))
            .args(["--devices", "231D0126,231D0127,231D2204"]),
        1,
    );
    assert!(out.contains("045E0B22 (3 bindings)"), "{out}");
    assert!(!out.contains("231D0126 ("), "{out}");
    assert!(!out.contains("warning:"), "{out}");
    assert!(out.contains("Custom.4.2.binds (from BindingLoadingErrors.log)"), "{out}");
}

#[test]
fn c3b_all_devices_present_exits_0() {
    let (_tmp, opts) = fixture_copy();
    let out = stdout_of(
        bin()
            .arg("check-binds")
            .arg(opts.join("Bindings"))
            .args(["--devices", "231D0126,231D0127,231D2204,045e0b22"]),
        0,
    );
    assert!(!out.contains("missing device"), "{out}");
}

#[test]
fn c3c_broken_preset_from_log_reports_pad_and_four_warnings() {
    let (_tmp, opts) = fixture_copy();
    let b = opts.join("Bindings");
    fs::rename(b.join("StartPreset.4.start.broken"), b.join("StartPreset.4.start")).unwrap();
    let out = stdout_of(bin().arg("check-binds").arg(&b).arg("--from-log"), 1);
    assert!(out.contains("Custom.4.2.binds (from BindingLoadingErrors.log)"), "{out}");
    assert!(out.contains("missing device: 045E0B22 (3 bindings)"), "{out}");
    assert_eq!(out.matches("missing device").count(), 1, "{out}");
    for (n, v) in [(1, "KeyboardMouseOnly"), (2, "GenericJoystick"), (3, "KeyboardMouseOnly"), (4, "KeyboardMouseOnly")] {
        assert!(out.contains(&format!("StartPreset.4.start line {n}: \"{v}\", not Custom")), "{out}");
    }
}

#[test]
fn c3_resolution_falls_back_to_newest_custom_binds() {
    let (_tmp, opts) = fixture_copy();
    let b = opts.join("Bindings");
    fs::remove_file(b.join("BindingLoadingErrors.log")).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(b.join("Custom.4.9.binds"), "<Root><Primary Device=\"ABCD0001\" Key=\"x\" /></Root>").unwrap();
    let out = stdout_of(bin().arg("check-binds").arg(&b).args(["--devices", ""]), 1);
    assert!(out.contains("Custom.4.9.binds (from newest Custom.4.*.binds)"), "{out}");
    // A newer directory with a binds-like name is not a candidate.
    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::create_dir(b.join("Custom.4.99.binds")).unwrap();
    let out = stdout_of(bin().arg("check-binds").arg(&b).args(["--devices", ""]), 1);
    assert!(out.contains("Custom.4.9.binds (from newest Custom.4.*.binds)"), "{out}");
    assert!(out.contains("missing device: ABCD0001 (1 binding)"), "{out}");
    // --binds wins over everything.
    let out = stdout_of(
        bin().arg("check-binds").arg(&b).args(["--devices", "231D0126,231D0127,045E0B22", "--binds", "Custom.4.2.binds"]),
        0,
    );
    assert!(out.contains("Custom.4.2.binds (from --binds)"), "{out}");
}

#[test]
fn c3_requires_devices_or_from_log() {
    let (_tmp, opts) = fixture_copy();
    bin().arg("check-binds").arg(opts.join("Bindings")).assert().code(2);
}

// ---- Criterion 4: restore-preset ------------------------------------------

#[test]
fn c4_restore_preset_writes_custom_crlf_x4_with_backup() {
    let tmp = tempfile::tempdir().unwrap();
    let b = tmp.path().join("Bindings");
    fs::create_dir_all(&b).unwrap();
    let broken = fs::read(fixture().join("Bindings/StartPreset.4.start.broken")).unwrap();
    fs::write(b.join("StartPreset.4.start"), &broken).unwrap();

    bin().arg("restore-preset").arg(&b).assert().success();

    assert_eq!(fs::read(b.join("StartPreset.4.start")).unwrap(), b"Custom\r\n".repeat(4));
    let baks: Vec<PathBuf> = fs::read_dir(&b)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.file_name().unwrap().to_str().unwrap().starts_with("StartPreset.4.start.bak-"))
        .collect();
    assert_eq!(baks.len(), 1, "{baks:?}");
    assert_eq!(fs::read(&baks[0]).unwrap(), broken);

    // The restored file now passes the preset check.
    let out = stdout_of(bin().arg("check-binds").arg(&b).args(["--devices", "", "--binds"]).arg(fixture().join("Bindings/Custom.4.2.binds")), 1);
    assert!(!out.contains("warning:"), "{out}");
}

// ---- Criterion 5: apply ---------------------------------------------------

/// Lines of `a` and `b` that differ, as (line number, a, b). Same line count required.
fn changed_lines(a: &[u8], b: &[u8]) -> Vec<(usize, String, String)> {
    let a = String::from_utf8(a.to_vec()).unwrap();
    let b = String::from_utf8(b.to_vec()).unwrap();
    let (la, lb): (Vec<&str>, Vec<&str>) = (a.split('\n').collect(), b.split('\n').collect());
    assert_eq!(la.len(), lb.len(), "line count changed");
    la.iter()
        .zip(&lb)
        .enumerate()
        .filter(|(_, (x, y))| x != y)
        .map(|(i, (x, y))| (i + 1, x.to_string(), y.to_string()))
        .collect()
}

fn laptop_profile(tmp: &Path, opts: &Path, section: &str) -> PathBuf {
    let profile = tmp.join("profile");
    capture(opts, &profile, "devmachine02");
    let mut t = fs::read_to_string(profile.join("profile.toml")).unwrap();
    t.push_str(section);
    fs::write(profile.join("profile.toml"), t).unwrap();
    profile
}

#[test]
fn c5_apply_overrides_change_only_the_target_lines_and_back_up() {
    let (tmp, src) = fixture_copy();
    let profile = laptop_profile(tmp.path(), &src, "\n[machine.laptop]\nssaa = 1.25\nmax_fps = 120\n");
    let target = tmp.path().join("Target");
    copy_tree(&fixture(), &target);

    bin()
        .arg("apply")
        .arg(&profile)
        .arg("--options")
        .arg(&target)
        .args(["--machine", "laptop"])
        .assert()
        .success();

    let fx = "Graphics/Custom.4.4.fxcfg";
    let ds = "Graphics/DisplaySettings.xml";
    let fixture_fx = fs::read(fixture().join(fx)).unwrap();
    let fixture_ds = fs::read(fixture().join(ds)).unwrap();
    assert_eq!(
        changed_lines(&fixture_fx, &fs::read(target.join(fx)).unwrap()),
        vec![(31, "\t<SSAAMultiplier>1.500000</SSAAMultiplier>".into(), "\t<SSAAMultiplier>1.250000</SSAAMultiplier>".into())]
    );
    assert_eq!(
        changed_lines(&fixture_ds, &fs::read(target.join(ds)).unwrap()),
        vec![(13, "\t<MaxFramesPerSecond>144</MaxFramesPerSecond>".into(), "\t<MaxFramesPerSecond>120</MaxFramesPerSecond>".into())]
    );
    // Every other bundle file is byte-identical to the fixture.
    for rel in [
        "Graphics/Settings.xml",
        "Graphics/StartPreset.start",
        "Graphics/GraphicsConfigurationOverride.xml",
        "Bindings/Custom.4.2.binds",
        "Bindings/StartPreset.4.start",
    ] {
        assert_eq!(fs::read(target.join(rel)).unwrap(), fs::read(fixture().join(rel)).unwrap(), "{rel}");
    }
    // The profile itself is not modified by apply.
    assert_eq!(fs::read(profile.join("files").join(fx)).unwrap(), fixture_fx);

    let stamps: Vec<PathBuf> = fs::read_dir(target.join("_loadout-backup")).unwrap().map(|e| e.unwrap().path()).collect();
    assert_eq!(stamps.len(), 1, "{stamps:?}");
    assert_eq!(fs::read(stamps[0].join(fx)).unwrap(), fixture_fx);
    assert_eq!(fs::read(stamps[0].join(ds)).unwrap(), fixture_ds);
}

#[test]
fn c5_apply_crlf_target_keeps_crlf() {
    let (tmp, src) = fixture_copy();
    let ds = src.join("Graphics/DisplaySettings.xml");
    let crlf = fs::read_to_string(&ds).unwrap().replace('\n', "\r\n");
    fs::write(&ds, &crlf).unwrap();
    let profile = laptop_profile(tmp.path(), &src, "\n[machine.laptop]\nrefresh_hz = 60\nscreen_width = 2560\n");
    let target = tmp.path().join("Target");
    fs::create_dir_all(&target).unwrap();
    bin().arg("apply").arg(&profile).arg("--options").arg(&target).args(["--machine", "laptop"]).assert().success();
    let expected = crlf
        .replace("<ScreenWidth>5120<", "<ScreenWidth>2560<")
        .replace("<DX11_RefreshRateNumerator>120<", "<DX11_RefreshRateNumerator>60<");
    assert_eq!(fs::read_to_string(target.join("Graphics/DisplaySettings.xml")).unwrap(), expected);
    // Nothing pre-existed, so nothing was backed up.
    assert!(!target.join("_loadout-backup").exists());
}

#[test]
fn c5_apply_absent_element_is_an_error_and_writes_nothing() {
    let (tmp, src) = fixture_copy();
    let ds = src.join("Graphics/DisplaySettings.xml");
    let text = fs::read_to_string(&ds).unwrap().replace("\t<MaxFramesPerSecond>144</MaxFramesPerSecond>\n", "");
    fs::write(&ds, text).unwrap();
    let profile = laptop_profile(tmp.path(), &src, "\n[machine.laptop]\nmax_fps = 120\n");
    let target = tmp.path().join("Target");
    copy_tree(&fixture(), &target);
    let err = bin()
        .arg("apply")
        .arg(&profile)
        .arg("--options")
        .arg(&target)
        .args(["--machine", "laptop"])
        .assert()
        .code(3)
        .get_output()
        .stderr
        .clone();
    assert!(String::from_utf8(err).unwrap().contains("never inserts"));
    assert!(!target.join("_loadout-backup").exists());
    assert_eq!(
        fs::read(target.join("Graphics/DisplaySettings.xml")).unwrap(),
        fs::read(fixture().join("Graphics/DisplaySettings.xml")).unwrap()
    );
}

// ---- v0.1 (claude/plans/2026-11-21-loadout-v0.1.md) -----------------------

#[test]
fn v01_c2_apply_ssaa_targets_highest_version_fxcfg() {
    let (tmp, src) = fixture_copy();
    let profile = laptop_profile(tmp.path(), &src, "\n[machine.laptop]\nssaa = 1.25\n");
    let fx44 = fs::read(fixture().join("Graphics/Custom.4.4.fxcfg")).unwrap();
    let pg = profile.join("files/Graphics");
    fs::write(pg.join("Custom.4.2.fxcfg"), &fx44).unwrap();
    fs::write(pg.join("Custom.fxcfg.bak"), &fx44).unwrap();
    let target = tmp.path().join("Target");
    copy_tree(&fixture(), &target);

    let out = bin()
        .arg("apply")
        .arg(&profile)
        .arg("--options")
        .arg(&target)
        .args(["--machine", "laptop"])
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stdout.contains("ssaa target: Graphics/Custom.4.4.fxcfg\n"), "{stdout}");
    assert!(stderr.contains("notice: ignoring Graphics/Custom.fxcfg.bak"), "{stderr}");
    assert!(!stderr.contains("Custom.4.2"), "{stderr}");

    assert_eq!(
        changed_lines(&fx44, &fs::read(target.join("Graphics/Custom.4.4.fxcfg")).unwrap()),
        vec![(31, "\t<SSAAMultiplier>1.500000</SSAAMultiplier>".into(), "\t<SSAAMultiplier>1.250000</SSAAMultiplier>".into())]
    );
    assert_eq!(fs::read(target.join("Graphics/Custom.4.2.fxcfg")).unwrap(), fx44);
    assert_eq!(fs::read(target.join("Graphics/Custom.fxcfg.bak")).unwrap(), fx44);
}

#[test]
fn v01_c2_apply_ssaa_with_no_parsable_fxcfg_is_an_error() {
    let (tmp, src) = fixture_copy();
    let profile = laptop_profile(tmp.path(), &src, "\n[machine.laptop]\nssaa = 1.25\n");
    let pg = profile.join("files/Graphics");
    fs::rename(pg.join("Custom.4.4.fxcfg"), pg.join("Custom.fxcfg.bak")).unwrap();
    let target = tmp.path().join("Target");
    copy_tree(&fixture(), &target);
    bin()
        .arg("apply")
        .arg(&profile)
        .arg("--options")
        .arg(&target)
        .args(["--machine", "laptop"])
        .assert()
        .code(3);
    assert!(!target.join("_loadout-backup").exists());
}

#[test]
fn v01_c2_apply_ssaa_refuses_when_target_has_a_newer_fxcfg() {
    let (tmp, src) = fixture_copy();
    let profile = laptop_profile(tmp.path(), &src, "\n[machine.laptop]\nssaa = 1.25\n");
    let target = tmp.path().join("Target");
    copy_tree(&fixture(), &target);
    let fx44 = fs::read(fixture().join("Graphics/Custom.4.4.fxcfg")).unwrap();
    fs::write(target.join("Graphics/Custom.4.5.fxcfg"), &fx44).unwrap();
    let err = bin()
        .arg("apply")
        .arg(&profile)
        .arg("--options")
        .arg(&target)
        .args(["--machine", "laptop"])
        .assert()
        .code(3)
        .get_output()
        .stderr
        .clone();
    let err = String::from_utf8(err).unwrap();
    assert!(err.contains("Graphics/Custom.4.5.fxcfg, newer than the profile's Graphics/Custom.4.4.fxcfg"), "{err}");
    assert!(!target.join("_loadout-backup").exists());
    assert_eq!(fs::read(target.join("Graphics/Custom.4.4.fxcfg")).unwrap(), fx44);
}

#[test]
fn v01_c4_version_is_0_1_1() {
    bin().arg("--version").assert().success().stdout("ed-loadout 0.1.1\n");
}

/// A LOCALAPPDATA root holding a fixture copy at the default Options path.
fn local_appdata() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let opts = tmp.path().join("LocalAppData/Frontier Developments/Elite Dangerous/Options");
    copy_tree(&fixture(), &opts);
    (tmp, opts)
}

#[test]
fn v01_c3_capture_defaults_to_localappdata_options() {
    let (tmp, opts) = local_appdata();
    let explicit = tmp.path().join("explicit");
    capture(&opts, &explicit, "devmachine02");
    let defaulted = tmp.path().join("defaulted");
    let out = bin()
        .env("LOCALAPPDATA", tmp.path().join("LocalAppData"))
        .args(["capture", "--out"])
        .arg(&defaulted)
        .args(["--machine", "devmachine02"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(out).unwrap();
    assert!(out.starts_with(&format!("options dir: {}\n", opts.display())), "{out}");

    // Same result as v0 criterion 1: same file set, same bytes, same sha256.
    let files = |p: &Path| {
        let t: toml::Table = fs::read_to_string(p.join("profile.toml")).unwrap().parse().unwrap();
        t["files"].as_table().unwrap().clone()
    };
    assert_eq!(files(&defaulted), files(&explicit));
    assert_eq!(files(&defaulted).len(), 7);
    for rel in files(&defaulted).keys() {
        assert_eq!(
            fs::read(defaulted.join("files").join(rel)).unwrap(),
            fs::read(fixture().join(rel)).unwrap(),
            "{rel}"
        );
    }
}

#[test]
fn v01_c3_unset_localappdata_exits_3_naming_the_path() {
    let tmp = tempfile::tempdir().unwrap();
    let err = bin()
        .env_remove("LOCALAPPDATA")
        .args(["capture", "--out"])
        .arg(tmp.path().join("p"))
        .args(["--machine", "a"])
        .assert()
        .code(3)
        .get_output()
        .stderr
        .clone();
    let err = String::from_utf8(err).unwrap();
    assert_eq!(err.lines().count(), 1, "{err}");
    assert!(err.contains("LOCALAPPDATA is not set"), "{err}");
    assert!(err.contains("Frontier Developments/Elite Dangerous/Options"), "{err}");
    assert!(!tmp.path().join("p").exists());
}

#[test]
fn v01_c3_missing_default_dir_exits_3_naming_the_path() {
    let tmp = tempfile::tempdir().unwrap();
    let err = bin()
        .env("LOCALAPPDATA", tmp.path())
        .args(["check-binds", "--devices", ""])
        .assert()
        .code(3)
        .get_output()
        .stderr
        .clone();
    let err = String::from_utf8(err).unwrap();
    let tried = tmp.path().join("Frontier Developments/Elite Dangerous/Options/Bindings");
    assert_eq!(err, format!("error: {}: Bindings dir not found; pass the Bindings dir\n", tried.display()));
}

#[test]
fn v01_c3_binds_commands_and_apply_default_too() {
    let (tmp, opts) = local_appdata();
    let lad = tmp.path().join("LocalAppData");
    let bindings = opts.join("Bindings");
    let out = stdout_of(
        bin().env("LOCALAPPDATA", &lad).args(["check-binds", "--devices", "231D0126,231D0127,231D2204,045e0b22"]),
        0,
    );
    assert!(out.starts_with(&format!("bindings dir: {}\n", bindings.display())), "{out}");

    fs::copy(fixture().join("Bindings/StartPreset.4.start.broken"), bindings.join("StartPreset.4.start")).unwrap();
    bin().env("LOCALAPPDATA", &lad).arg("restore-preset").assert().success();
    assert_eq!(fs::read(bindings.join("StartPreset.4.start")).unwrap(), b"Custom\r\nCustom\r\nCustom\r\nCustom\r\n");

    let profile = laptop_profile(tmp.path(), &fixture(), "\n[machine.laptop]\nmax_fps = 120\n");
    bin()
        .env("LOCALAPPDATA", &lad)
        .arg("apply")
        .arg(&profile)
        .args(["--machine", "laptop"])
        .assert()
        .success();
    assert!(opts.join("_loadout-backup").is_dir());
    assert!(fs::read_to_string(opts.join("Graphics/DisplaySettings.xml")).unwrap().contains("<MaxFramesPerSecond>120<"));
}
