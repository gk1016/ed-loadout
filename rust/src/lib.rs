//! loadout-core: capture, diff and apply an Elite Dangerous settings bundle.
//! One library under two front ends: the `ed-loadout` CLI
//! (`src/bin/ed-loadout.rs`) and the LOADOUT desktop app (Qt, `../cpp`).

pub mod apply;
pub mod binds;
pub mod bundle;
pub mod capture;
pub mod diff;
pub mod facts;
pub mod tiers;
