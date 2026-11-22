//! ed-loadout: capture, diff and apply an Elite Dangerous settings bundle.
//!
//! Exit codes: 0 ok / identical, 1 differences or problems found,
//! 2 refused (game running) or usage error, 3 error.

mod apply;
mod binds;
mod bundle;
mod capture;
mod diff;
mod facts;
mod tiers;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "ed-loadout", version, about = "Capture, diff and apply an Elite Dangerous settings bundle")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Copy the settings bundle from an Options dir into a new profile.
    Capture {
        /// Elite Dangerous Options dir (holds Graphics/ and Bindings/).
        /// Default: %LOCALAPPDATA%/Frontier Developments/Elite Dangerous/Options.
        #[arg(long)]
        options: Option<PathBuf>,
        /// Profile dir to create.
        #[arg(long)]
        out: PathBuf,
        /// Name of the machine being captured.
        #[arg(long)]
        machine: String,
    },
    /// Compare two profiles. Exit 0 when identical, 1 when different.
    Diff {
        profile_a: PathBuf,
        profile_b: PathBuf,
    },
    /// Report bound devices that are missing and a clobbered StartPreset.4.start.
    /// Exit 1 when any device is missing or any preset line is not Custom.
    /// With neither --devices nor --from-log, the connected devices come from
    /// the device facts (detected on Windows, else --facts).
    #[command(group = clap::ArgGroup::new("source").args(["devices", "from_log"]))]
    CheckBinds {
        /// Bindings dir (holds the .binds files and StartPreset.4.start).
        /// Default: the Bindings dir under the default Options dir.
        bindings_dir: Option<PathBuf>,
        /// Comma-separated GUIDs of the devices connected now.
        #[arg(long)]
        devices: Option<String>,
        /// Take missing GUIDs from BindingLoadingErrors.log.
        #[arg(long)]
        from_log: bool,
        /// Binds file to check (default: named by the log, else newest Custom.4.*.binds).
        #[arg(long)]
        binds: Option<PathBuf>,
        /// Facts TOML to take the connected devices from instead of detecting.
        #[arg(long, conflicts_with_all = ["devices", "from_log"])]
        facts: Option<PathBuf>,
    },
    /// Rewrite StartPreset.4.start to four lines of the preset name (CRLF),
    /// after backing up the old file.
    RestorePreset {
        /// Bindings dir (holds StartPreset.4.start).
        /// Default: the Bindings dir under the default Options dir.
        bindings_dir: Option<PathBuf>,
        /// Preset name to write on every line.
        #[arg(long, default_value = "Custom")]
        preset: String,
    },
    /// Back up, write a profile into an Options dir, then apply the
    /// [machine.<name>] overrides from profile.toml. Exit 2 if the game is running.
    Apply {
        profile: PathBuf,
        /// Elite Dangerous Options dir to write into.
        /// Default: %LOCALAPPDATA%/Frontier Developments/Elite Dangerous/Options.
        #[arg(long)]
        options: Option<PathBuf>,
        /// Machine whose [machine.<name>] overrides apply.
        /// Default: the hostname from the device facts.
        #[arg(long)]
        machine: Option<String>,
        /// Facts TOML to use instead of detecting this machine.
        #[arg(long)]
        facts: Option<PathBuf>,
        /// Exit 1 and write nothing when a bound device is not connected.
        #[arg(long)]
        strict_binds: bool,
    },
    /// Print the panel, GPU and connected devices, and the values derived
    /// from them. Windows detects; elsewhere pass --facts.
    Detect {
        /// Facts TOML to use instead of detecting.
        #[arg(long)]
        facts: Option<PathBuf>,
        /// Write the facts as TOML to this new file.
        #[arg(long)]
        dump: Option<PathBuf>,
    },
}

/// The dir given on the command line, else `$LOCALAPPDATA/Frontier
/// Developments/Elite Dangerous/Options` (plus `sub`), which must exist.
/// The defaulted path is printed.
fn default_dir(given: Option<PathBuf>, sub: Option<&str>) -> Result<PathBuf, String> {
    if let Some(p) = given {
        return Ok(p);
    }
    let base = std::env::var_os("LOCALAPPDATA").filter(|v| !v.is_empty());
    let mut p = PathBuf::from(base.clone().unwrap_or_else(|| "$LOCALAPPDATA".into()));
    p.extend(["Frontier Developments", "Elite Dangerous", "Options"]);
    p.extend(sub);
    let what = if sub.is_some() { "Bindings" } else { "Options" };
    if base.is_none() {
        return Err(format!("LOCALAPPDATA is not set; tried {}; pass the {what} dir", p.display()));
    }
    if !p.is_dir() {
        return Err(format!("{}: {what} dir not found; pass the {what} dir", p.display()));
    }
    println!("{} dir: {}", what.to_ascii_lowercase(), p.display());
    Ok(p)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.cmd {
        Cmd::Capture { options, out, machine } => {
            default_dir(options, None).and_then(|o| capture::run(&o, &out, &machine))
        }
        Cmd::Diff { profile_a, profile_b } => diff::run(&profile_a, &profile_b),
        Cmd::CheckBinds { bindings_dir, devices, from_log, binds, facts } => {
            // The device source first: without one, that is the error to report.
            let devices = match devices {
                None if !from_log => facts::load(facts.as_deref()).and_then(|f| {
                    let f = f.ok_or_else(|| format!("no --devices or --from-log; {}", facts::NO_FACTS))?;
                    println!("devices: {} (from device facts)", f.devices.join(","));
                    Ok(Some(f.devices.join(",")))
                }),
                d => Ok(d),
            };
            devices.and_then(|devices| {
                default_dir(bindings_dir, Some("Bindings"))
                    .and_then(|d| binds::run(&d, devices.as_deref(), from_log, binds.as_deref()))
            })
        }
        Cmd::RestorePreset { bindings_dir, preset } => {
            default_dir(bindings_dir, Some("Bindings")).and_then(|d| binds::restore_preset(&d, &preset))
        }
        Cmd::Apply { profile, options, machine, facts, strict_binds } => default_dir(options, None)
            .and_then(|o| apply::run(&profile, &o, machine.as_deref(), facts.as_deref(), strict_binds)),
        Cmd::Detect { facts, dump } => facts::run_detect(facts.as_deref(), dump.as_deref()),
    };
    match result {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(3)
        }
    }
}
