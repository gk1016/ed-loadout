//! loadout-core: capture, diff and apply an Elite Dangerous settings bundle.
//! One library under two front ends: the `ed-loadout` CLI
//! (`src/bin/ed-loadout.rs`) and the LOADOUT desktop app (Qt, `../cpp`),
//! which calls it through `bridge.rs`.

// Never panic in an extern "Rust" fn: a panic crossing cxx aborts the
// process. As in frago-core; the tests may unwrap.
#![warn(clippy::unwrap_used, clippy::expect_used)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

#[macro_use]
pub mod log;

pub mod apply;
pub mod binds;
pub mod bridge;
pub mod bundle;
pub mod capture;
pub mod diff;
pub mod facts;
pub mod tiers;
