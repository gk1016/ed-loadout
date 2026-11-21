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
