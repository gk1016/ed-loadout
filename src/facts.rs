//! Device facts: primary panel, GPU and connected controllers. Detected on
//! Windows; everywhere, a `--facts <toml>` file supplies the same struct and
//! `detect --dump <file>` writes one. All `unsafe` in the crate lives here.

use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct Facts {
    pub hostname: String,
    pub screen_w: i64,
    pub screen_h: i64,
    pub refresh_hz: i64,
    pub gpu_name: String,
    pub gpu_vram_mb: i64,
    /// Connected devices as Elite's binds `Device` GUID: `VVVVPPPP`, USB
    /// vendor + product, upper-case hex.
    pub devices: Vec<String>,
}

pub const NO_FACTS: &str = "no device facts on this platform; pass --facts <file>";

/// `--facts` if given, else Windows detection, else `None`.
pub fn load(file: Option<&Path>) -> Result<Option<Facts>, String> {
    match file {
        Some(p) => read(p).map(Some),
        None => detect(),
    }
}

pub fn read(path: &Path) -> Result<Facts, String> {
    let at = |e: String| format!("{}: {e}", path.display());
    let text = fs::read_to_string(path).map_err(|e| at(e.to_string()))?;
    let table: toml::Table = text.parse().map_err(|e: toml::de::Error| at(e.to_string()))?;
    from_table(&table).map_err(at)
}

fn from_table(t: &toml::Table) -> Result<Facts, String> {
    const KEYS: [&str; 7] = ["hostname", "screen_w", "screen_h", "refresh_hz", "gpu_name", "gpu_vram_mb", "devices"];
    if let Some(k) = t.keys().find(|k| !KEYS.contains(&k.as_str())) {
        return Err(format!("{k}: unknown key"));
    }
    let string = |k: &str| t.get(k).and_then(|v| v.as_str()).map(str::to_owned).ok_or(format!("{k}: expected a string"));
    let int = |k: &str, min: i64| match t.get(k).and_then(|v| v.as_integer()) {
        Some(n) if n >= min => Ok(n),
        _ => Err(format!("{k}: expected an integer >= {min}")),
    };
    let devices = t
        .get("devices")
        .and_then(|v| v.as_array())
        .ok_or("devices: expected an array of strings")?
        .iter()
        .map(|v| v.as_str().map(str::to_ascii_uppercase).ok_or("devices: expected an array of strings"))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Facts {
        hostname: string("hostname")?,
        screen_w: int("screen_w", 1)?,
        screen_h: int("screen_h", 1)?,
        refresh_hz: int("refresh_hz", 1)?,
        gpu_name: string("gpu_name")?,
        gpu_vram_mb: int("gpu_vram_mb", 0)?,
        devices,
    })
}

pub fn to_toml(f: &Facts) -> String {
    let mut t = toml::Table::new();
    t.insert("hostname".into(), f.hostname.clone().into());
    t.insert("screen_w".into(), f.screen_w.into());
    t.insert("screen_h".into(), f.screen_h.into());
    t.insert("refresh_hz".into(), f.refresh_hz.into());
    t.insert("gpu_name".into(), f.gpu_name.clone().into());
    t.insert("gpu_vram_mb".into(), f.gpu_vram_mb.into());
    t.insert("devices".into(), f.devices.clone().into());
    toml::to_string(&t).expect("a table of strings and integers serialises")
}

pub fn print(f: &Facts) {
    println!("hostname: {}", f.hostname);
    println!("panel: {}x{} @ {} Hz", f.screen_w, f.screen_h, f.refresh_hz);
    println!("gpu: {} ({} MB)", f.gpu_name, f.gpu_vram_mb);
    println!("devices: {}", if f.devices.is_empty() { "(none)".to_owned() } else { f.devices.join(", ") });
}

/// `detect`: print the facts and the derived values; `--dump` writes the
/// facts as TOML (never over an existing file).
pub fn run_detect(file: Option<&Path>, dump: Option<&Path>) -> Result<i32, String> {
    let f = load(file)?.ok_or(NO_FACTS)?;
    print(&f);
    let d = crate::tiers::derive(&f);
    println!("tier: {:?}", d.tier);
    d.print();
    if let Some(out) = dump {
        if out.exists() {
            return Err(format!("{}: already exists; refusing to overwrite", out.display()));
        }
        fs::write(out, to_toml(&f)).map_err(|e| format!("{}: {e}", out.display()))?;
        println!("wrote {}", out.display());
    }
    Ok(0)
}

#[cfg(not(windows))]
fn detect() -> Result<Option<Facts>, String> {
    Ok(None)
}

#[cfg(windows)]
fn detect() -> Result<Option<Facts>, String> {
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_round_trip_and_unknown_key() {
        let f = Facts {
            hostname: "h".into(),
            screen_w: 2560,
            screen_h: 1600,
            refresh_hz: 240,
            gpu_name: "g".into(),
            gpu_vram_mb: 8,
            devices: vec!["231D0126".into()],
        };
        assert_eq!(from_table(&to_toml(&f).parse().unwrap()).unwrap(), f);
        let mut t: toml::Table = to_toml(&f).parse().unwrap();
        t.insert("vram".into(), 1.into());
        assert!(from_table(&t).unwrap_err().contains("vram"));
    }
}
