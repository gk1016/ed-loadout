//! Tier table: GPU name -> tier, and the values derived from device facts.
//! Pure; no I/O. Plan: claude/plans/2026-11-22-loadout-v0.2.md.

use crate::facts::Facts;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tier {
    T1,
    T2,
    T3,
}

/// What `apply` does with `Graphics/GraphicsConfigurationOverride.xml`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OverrideTier {
    /// Write `tiers/t1.xml`.
    T1,
    /// Write `tiers/t2.xml`.
    T2,
    /// Write the profile's own file (v0.1 behaviour).
    Captured,
    /// Write nothing; the target's file is left alone.
    None,
}

impl OverrideTier {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "t1" => Some(Self::T1),
            "t2" => Some(Self::T2),
            "captured" => Some(Self::Captured),
            "none" => Some(Self::None),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::T1 => "t1",
            Self::T2 => "t2",
            Self::Captured => "captured",
            Self::None => "none",
        }
    }
}

/// Internal-render budget in megapixels. T1 is the measured anchor:
/// devmachine02 (desktop RTX 5090) holds a flat 120 Hz at 5120x1440 x SS 1.5
/// (~16.6 Mpx) with 81-85% GPU use.
const T1_BUDGET_MPX: f64 = 16.6;
const T2_BUDGET_MPX: f64 = 10.0;
const SSAA_MIN: f64 = 1.0;
const SSAA_MAX: f64 = 1.5;

/// T1 `RTX (5090|5080|4090)` not followed by `Laptop`; T2 `RTX (5090|5080)
/// Laptop|RTX 5070 Ti|RTX 4080( SUPER)?|RTX 4070 Ti SUPER`; T3 otherwise.
/// Substring matches, case-sensitive, as the plan's patterns read. Any other
/// name containing `Laptop` is T3 (v0.3: `RTX 4080 Laptop GPU` and `RTX 5070
/// Ti Laptop GPU` no longer take the desktop parts' T2).
pub fn tier(gpu_name: &str) -> Tier {
    for model in ["RTX 5090", "RTX 5080", "RTX 4090"] {
        for (i, _) in gpu_name.match_indices(model) {
            if !gpu_name[i + model.len()..].trim_start().starts_with("Laptop") {
                return Tier::T1;
            }
        }
    }
    if ["RTX 5090 Laptop", "RTX 5080 Laptop"].iter().any(|p| gpu_name.contains(p)) {
        return Tier::T2;
    }
    if gpu_name.contains("Laptop") {
        return Tier::T3;
    }
    let t2 = ["RTX 5070 Ti", "RTX 4080", "RTX 4070 Ti SUPER"];
    if t2.iter().any(|p| gpu_name.contains(p)) {
        return Tier::T2;
    }
    Tier::T3
}

/// Frame cap for a panel refresh rate. Up to 170 Hz the cap is the refresh
/// itself; above it, the refresh divided (integer division) by the smallest
/// integer `k` that brings it to 170 or under. Returns (cap, k).
pub fn max_fps(refresh_hz: i64) -> (i64, i64) {
    const CEILING: i64 = 170;
    let mut k = 1;
    while refresh_hz / k > CEILING {
        k += 1;
    }
    (refresh_hz / k, k)
}

/// Values derived from the facts, each with the reason printed beside it.
#[derive(Debug, Clone, PartialEq)]
pub struct Derived {
    pub tier: Tier,
    pub screen_width: i64,
    pub screen_height: i64,
    pub refresh_hz: i64,
    pub max_fps: i64,
    pub ssaa: f64,
    pub terrain_quality: i64,
    pub override_tier: OverrideTier,
    /// (key, value, reason), in print order.
    pub lines: Vec<(&'static str, String, String)>,
}

/// `1.5` -> "1.5", `1.0` -> "1.0", `1.25` -> "1.25".
pub fn fmt_ssaa(v: f64) -> String {
    if v.fract() == 0.0 { format!("{v:.1}") } else { format!("{v}") }
}

pub fn derive(f: &Facts) -> Derived {
    let tier = tier(&f.gpu_name);
    let panel = format!("{}x{}", f.screen_w, f.screen_h);
    let (ssaa, ssaa_reason) = match tier {
        Tier::T3 => (1.0, "T3 (GPU not in the tier table): fixed 1.0".to_owned()),
        Tier::T1 | Tier::T2 => {
            let (name, budget) = if tier == Tier::T1 { ("T1", T1_BUDGET_MPX) } else { ("T2", T2_BUDGET_MPX) };
            let px = f.screen_w as f64 * f.screen_h as f64;
            let rounded = ((budget * 1e6 / px).sqrt() * 20.0).round() / 20.0;
            let v = rounded.clamp(SSAA_MIN, SSAA_MAX);
            let clamp = if v != rounded { format!(", {} clamped to {}", fmt_ssaa(rounded), fmt_ssaa(v)) } else { String::new() };
            (
                v,
                format!("{name} budget {budget:.1} Mpx over {panel}{clamp}; starting point: raise while GPU use stays under ~85%"),
            )
        }
    };
    let (terrain_quality, override_tier, ot_reason) = match tier {
        Tier::T1 => (4, OverrideTier::T1, "T1: the devmachine02 beyond-Ultra override"),
        Tier::T2 => (3, OverrideTier::T2, "T2: t1 without DirectionalShadows_Ultra, SpotShadows_Ultra, Volumetrics"),
        Tier::T3 => (3, OverrideTier::None, "T3: no override XML; the target's file is left alone"),
    };
    let tname = format!("{tier:?}");
    let (max_fps, k) = max_fps(f.refresh_hz);
    let fps_reason = if k == 1 {
        "= refresh_hz; VRR is not detected, VSync is left as captured".to_owned()
    } else {
        format!(
            "= refresh_hz / {k} (k = {k}, the smallest divisor that brings {} Hz to 170 or under); VRR is not detected, VSync is left as captured",
            f.refresh_hz
        )
    };
    let lines = vec![
        ("screen_width", f.screen_w.to_string(), format!("panel {panel}")),
        ("screen_height", f.screen_h.to_string(), format!("panel {panel}")),
        ("refresh_hz", f.refresh_hz.to_string(), "panel refresh rate".to_owned()),
        ("max_fps", max_fps.to_string(), fps_reason),
        ("ssaa", fmt_ssaa(ssaa), ssaa_reason),
        ("terrain_quality", terrain_quality.to_string(), tname),
        ("override_tier", override_tier.as_str().to_owned(), ot_reason.to_owned()),
    ];
    Derived {
        tier,
        screen_width: f.screen_w,
        screen_height: f.screen_h,
        refresh_hz: f.refresh_hz,
        max_fps,
        ssaa,
        terrain_quality,
        override_tier,
        lines,
    }
}

impl Derived {
    pub fn print(&self) {
        for (key, value, reason) in &self.lines {
            say!("derived {key} = {value}  ({reason})");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_table() {
        for (name, t) in [
            ("NVIDIA GeForce RTX 5090", Tier::T1),
            ("NVIDIA GeForce RTX 5080", Tier::T1),
            ("NVIDIA GeForce RTX 4090", Tier::T1),
            ("NVIDIA GeForce RTX 5090 Laptop GPU", Tier::T2),
            ("NVIDIA GeForce RTX 5080 Laptop GPU", Tier::T2),
            ("NVIDIA GeForce RTX 5070 Ti", Tier::T2),
            ("NVIDIA GeForce RTX 4080", Tier::T2),
            ("NVIDIA GeForce RTX 4080 SUPER", Tier::T2),
            ("NVIDIA GeForce RTX 4070 Ti SUPER", Tier::T2),
            ("NVIDIA GeForce RTX 4070 Ti", Tier::T3),
            ("NVIDIA GeForce RTX 5070", Tier::T3),
            ("AMD Radeon RX 7800 XT", Tier::T3),
            // v0.3: Laptop parts other than the 5090 / 5080 are T3.
            ("NVIDIA GeForce RTX 4080 Laptop GPU", Tier::T3),
            ("NVIDIA GeForce RTX 5070 Ti Laptop GPU", Tier::T3),
            ("NVIDIA GeForce RTX 4090 Laptop GPU", Tier::T3),
            ("NVIDIA GeForce RTX 4070 Laptop GPU", Tier::T3),
        ] {
            assert_eq!(tier(name), t, "{name}");
        }
    }

    fn facts(w: i64, h: i64, gpu: &str) -> Facts {
        Facts {
            hostname: "h".into(),
            screen_w: w,
            screen_h: h,
            refresh_hz: 60,
            gpu_name: gpu.into(),
            gpu_vram_mb: 0,
            devices: vec![],
        }
    }

    #[test]
    fn max_fps_is_the_refresh_up_to_170_then_divided_by_the_smallest_k() {
        for (hz, cap, k) in [(240, 120, 2), (360, 120, 3), (175, 87, 2), (165, 165, 1), (144, 144, 1), (120, 120, 1), (170, 170, 1), (171, 85, 2), (60, 60, 1)] {
            assert_eq!(max_fps(hz), (cap, k), "{hz}");
        }
        let mut f = facts(2560, 1600, "RTX 5090 Laptop GPU");
        f.refresh_hz = 240;
        let d = derive(&f);
        assert_eq!((d.max_fps, d.refresh_hz), (120, 240));
        let line = d.lines.iter().find(|l| l.0 == "max_fps").unwrap();
        assert_eq!(line.1, "120");
        assert!(line.2.contains("k = 2"), "{}", line.2);
        f.refresh_hz = 144;
        let d = derive(&f);
        assert_eq!(d.max_fps, 144);
        assert!(d.lines.iter().find(|l| l.0 == "max_fps").unwrap().2.starts_with("= refresh_hz;"));
    }

    #[test]
    fn ssaa_rounds_to_005_and_clamps() {
        assert_eq!(derive(&facts(5120, 1440, "RTX 5090")).ssaa, 1.5);
        assert_eq!(derive(&facts(3840, 2160, "RTX 4080")).ssaa, 1.1);
        assert_eq!(derive(&facts(2560, 1600, "RTX 5090 Laptop GPU")).ssaa, 1.5);
        assert_eq!(derive(&facts(7680, 4320, "RTX 5090")).ssaa, 1.0);
        assert_eq!(derive(&facts(3840, 2160, "RX 7800 XT")).ssaa, 1.0);
        // No overflow on absurd but parseable facts (Codex check 5).
        assert_eq!(derive(&facts(i64::MAX, i64::MAX, "RTX 5090")).ssaa, 1.0);
        assert_eq!(fmt_ssaa(1.0), "1.0");
        assert_eq!(fmt_ssaa(1.1), "1.1");
        assert_eq!(fmt_ssaa(1.25), "1.25");
    }
}
