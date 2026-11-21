//! ed-loadout: capture, diff and apply an Elite Dangerous settings bundle.
//!
//! Exit codes: 0 ok / identical, 1 differences or problems found,
//! 2 refused (game running) or usage error, 3 error.

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
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.cmd {
        Cmd::Capture { options, out, machine } => capture::run(&options, &out, &machine),
        Cmd::Diff { profile_a, profile_b } => diff::run(&profile_a, &profile_b),
    };
    match result {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(3)
        }
    }
}
