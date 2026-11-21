//! ed-loadout: capture, diff and apply an Elite Dangerous settings bundle.
//!
//! Exit codes: 0 ok / identical, 1 differences or problems found,
//! 2 refused (game running) or usage error, 3 error.

mod binds;
mod bundle;
mod capture;
mod diff;

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
        #[arg(long)]
        options: PathBuf,
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
    #[command(group = clap::ArgGroup::new("source").required(true).args(["devices", "from_log"]))]
    CheckBinds {
        /// Bindings dir (holds the .binds files and StartPreset.4.start).
        bindings_dir: PathBuf,
        /// Comma-separated GUIDs of the devices connected now.
        #[arg(long)]
        devices: Option<String>,
        /// Take missing GUIDs from BindingLoadingErrors.log.
        #[arg(long)]
        from_log: bool,
        /// Binds file to check (default: named by the log, else newest Custom.4.*.binds).
        #[arg(long)]
        binds: Option<PathBuf>,
    },
    /// Rewrite StartPreset.4.start to four lines of the preset name (CRLF),
    /// after backing up the old file.
    RestorePreset {
        /// Bindings dir (holds StartPreset.4.start).
        bindings_dir: PathBuf,
        /// Preset name to write on every line.
        #[arg(long, default_value = "Custom")]
        preset: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.cmd {
        Cmd::Capture { options, out, machine } => capture::run(&options, &out, &machine),
        Cmd::Diff { profile_a, profile_b } => diff::run(&profile_a, &profile_b),
        Cmd::CheckBinds { bindings_dir, devices, from_log, binds } => {
            binds::run(&bindings_dir, devices.as_deref(), from_log, binds.as_deref())
        }
        Cmd::RestorePreset { bindings_dir, preset } => binds::restore_preset(&bindings_dir, &preset),
    };
    match result {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(3)
        }
    }
}
