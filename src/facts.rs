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
        use std::io::Write;
        let mut file = fs::OpenOptions::new().write(true).create_new(true).open(out).map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                format!("{}: already exists; refusing to overwrite", out.display())
            } else {
                format!("{}: {e}", out.display())
            }
        })?;
        file.write_all(to_toml(&f).as_bytes()).map_err(|e| format!("{}: {e}", out.display()))?;
        println!("wrote {}", out.display());
    }
    Ok(0)
}

#[cfg(not(windows))]
fn detect() -> Result<Option<Facts>, String> {
    Ok(None)
}

/// A failed detection is a notice and no facts, so `apply` still runs.
#[cfg(windows)]
fn detect() -> Result<Option<Facts>, String> {
    match win::detect() {
        Ok(f) => Ok(Some(f)),
        Err(e) => {
            eprintln!("notice: device detection failed: {e}");
            Ok(None)
        }
    }
}

/// `\\?\HID#VID_231D&PID_0126&MI_00#...` -> `231D0126`. Case-insensitive;
/// anything without `VID_xxxx&PID_xxxx` -> None.
#[cfg_attr(not(windows), allow(dead_code))]
fn vid_pid(device_name: &str) -> Option<String> {
    let up = device_name.to_ascii_uppercase();
    let i = up.find("VID_")?;
    let rest = &up[i + 4..];
    let (vid, rest) = (rest.get(..4)?, rest.get(4..)?);
    let pid = rest.strip_prefix("&PID_")?.get(..4)?;
    let hex = |s: &str| s.bytes().all(|b| b.is_ascii_hexdigit());
    (hex(vid) && hex(pid)).then(|| format!("{vid}{pid}"))
}

/// The Win32 backend: `EnumDisplaySettingsExW`, DXGI, Raw Input.
#[cfg(windows)]
mod win {
    use super::{Facts, vid_pid};
    use std::ffi::c_void;
    use std::ptr::null_mut;
    use windows_sys::Win32::Graphics::Gdi::{DEVMODEW, ENUM_CURRENT_SETTINGS, EnumDisplaySettingsExW};
    use windows_sys::Win32::UI::Input::{
        GetRawInputDeviceInfoW, GetRawInputDeviceList, RAWINPUTDEVICELIST, RIDI_DEVICENAME,
    };
    use windows_sys::core::{GUID, HRESULT};

    // DXGI is COM and not in windows-sys; the one entry point is declared
    // the way windows-link declares its own, and the two interfaces are
    // called through their vtables. No CoInitialize is needed for DXGI.
    #[link(name = "dxgi.dll", kind = "raw-dylib", modifiers = "+verbatim")]
    unsafe extern "system" {
        fn CreateDXGIFactory1(riid: *const GUID, factory: *mut *mut c_void) -> HRESULT;
    }

    const IID_IDXGIFACTORY1: GUID = GUID {
        data1: 0x770a_ae78,
        data2: 0xf26f,
        data3: 0x4dba,
        data4: [0xa8, 0x29, 0x25, 0x3c, 0x83, 0xd1, 0xb3, 0x87],
    };
    const DXGI_ERROR_NOT_FOUND: HRESULT = 0x887A_0002_u32 as i32;
    const DXGI_ADAPTER_FLAG_SOFTWARE: u32 = 2;

    type Release = unsafe extern "system" fn(*mut c_void) -> u32;

    /// IDXGIFactory1: IUnknown (0-2), IDXGIObject (3-6), IDXGIFactory (7-11), EnumAdapters1 = 12.
    #[repr(C)]
    struct FactoryVtbl {
        _query_add_ref: [usize; 2],
        release: Release,
        _skip: [usize; 9],
        enum_adapters1: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> HRESULT,
    }

    /// IDXGIAdapter1: IUnknown (0-2), IDXGIObject (3-6), IDXGIAdapter (7-9), GetDesc1 = 10.
    #[repr(C)]
    struct AdapterVtbl {
        _query_add_ref: [usize; 2],
        release: Release,
        _skip: [usize; 7],
        get_desc1: unsafe extern "system" fn(*mut c_void, *mut AdapterDesc1) -> HRESULT,
    }

    /// DXGI_ADAPTER_DESC1.
    #[repr(C)]
    struct AdapterDesc1 {
        description: [u16; 128],
        vendor_id: u32,
        device_id: u32,
        sub_sys_id: u32,
        revision: u32,
        dedicated_video_memory: usize,
        dedicated_system_memory: usize,
        shared_system_memory: usize,
        adapter_luid: [u32; 2],
        flags: u32,
    }

    fn wide(buf: &[u16]) -> String {
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..end])
    }

    pub fn detect() -> Result<Facts, String> {
        let (screen_w, screen_h, refresh_hz) = panel()?;
        let (gpu_name, gpu_vram_mb) = gpu()?;
        Ok(Facts {
            hostname: std::env::var("COMPUTERNAME").unwrap_or_default(),
            screen_w,
            screen_h,
            refresh_hz,
            gpu_name,
            gpu_vram_mb,
            devices: devices()?,
        })
    }

    /// Current mode of the primary display.
    fn panel() -> Result<(i64, i64, i64), String> {
        // SAFETY: DEVMODEW is plain data; zeroed with dmSize set is the
        // documented input to EnumDisplaySettingsExW.
        let mut dm: DEVMODEW = unsafe { std::mem::zeroed() };
        dm.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        // SAFETY: null device name = the primary display; dm outlives the call.
        if unsafe { EnumDisplaySettingsExW(std::ptr::null(), ENUM_CURRENT_SETTINGS, &mut dm, 0) } == 0 {
            return Err("EnumDisplaySettingsExW failed".into());
        }
        Ok((dm.dmPelsWidth.into(), dm.dmPelsHeight.into(), dm.dmDisplayFrequency.into()))
    }

    /// Description and dedicated VRAM (MB) of the first non-software adapter.
    fn gpu() -> Result<(String, i64), String> {
        let mut factory: *mut c_void = null_mut();
        // SAFETY: valid IID and out-pointer.
        let hr = unsafe { CreateDXGIFactory1(&IID_IDXGIFACTORY1, &mut factory) };
        if hr < 0 || factory.is_null() {
            return Err(format!("CreateDXGIFactory1 failed (0x{hr:08X})"));
        }
        // SAFETY: a COM object's first field is its vtable pointer; the
        // layouts above follow dxgi.h. Every adapter and the factory are
        // released exactly once.
        unsafe {
            let fv = &**(factory as *const *const FactoryVtbl);
            let mut found = Err("no hardware DXGI adapter".to_owned());
            for i in 0.. {
                let mut adapter: *mut c_void = null_mut();
                let hr = (fv.enum_adapters1)(factory, i, &mut adapter);
                if hr == DXGI_ERROR_NOT_FOUND {
                    break;
                }
                if hr < 0 || adapter.is_null() {
                    found = Err(format!("EnumAdapters1({i}) failed (0x{hr:08X})"));
                    break;
                }
                let av = &**(adapter as *const *const AdapterVtbl);
                let mut desc: AdapterDesc1 = std::mem::zeroed();
                let hr = (av.get_desc1)(adapter, &mut desc);
                (av.release)(adapter);
                if hr >= 0 && desc.flags & DXGI_ADAPTER_FLAG_SOFTWARE == 0 {
                    found = Ok((wide(&desc.description), (desc.dedicated_video_memory / (1024 * 1024)) as i64));
                    break;
                }
            }
            (fv.release)(factory);
            found
        }
    }

    /// `VVVVPPPP` of every Raw Input device whose name carries VID/PID, deduped.
    fn devices() -> Result<Vec<String>, String> {
        let size = std::mem::size_of::<RAWINPUTDEVICELIST>() as u32;
        let mut n = 0u32;
        // SAFETY: a null list asks for the count only.
        if unsafe { GetRawInputDeviceList(null_mut(), &mut n, size) } == u32::MAX {
            return Err("GetRawInputDeviceList failed".into());
        }
        // SAFETY: plain data, overwritten by the call.
        let mut list: Vec<RAWINPUTDEVICELIST> = (0..n).map(|_| unsafe { std::mem::zeroed() }).collect();
        // SAFETY: list holds n entries of `size` bytes.
        let got = unsafe { GetRawInputDeviceList(list.as_mut_ptr(), &mut n, size) };
        if got == u32::MAX {
            return Err("GetRawInputDeviceList failed (device list changed?)".into());
        }
        let mut out: Vec<String> = Vec::new();
        for d in list.iter().take(got as usize) {
            let mut len = 0u32;
            // SAFETY: null data asks for the name length in characters.
            unsafe { GetRawInputDeviceInfoW(d.hDevice, RIDI_DEVICENAME, null_mut(), &mut len) };
            if len == 0 {
                continue;
            }
            let mut buf = vec![0u16; len as usize];
            // SAFETY: buf holds len characters.
            let r = unsafe { GetRawInputDeviceInfoW(d.hDevice, RIDI_DEVICENAME, buf.as_mut_ptr().cast(), &mut len) };
            if r == 0 || r == u32::MAX {
                continue;
            }
            if let Some(g) = vid_pid(&wide(&buf))
                && !out.contains(&g)
            {
                out.push(g);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vid_pid_parses_raw_input_names() {
        assert_eq!(vid_pid(r"\\?\HID#VID_231D&PID_0126&MI_00#8&1a2b3c&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}").as_deref(), Some("231D0126"));
        assert_eq!(vid_pid(r"\\?\HID#vid_045e&pid_0b22&IG_00#9&2c").as_deref(), Some("045E0B22"));
        assert_eq!(vid_pid(r"\\?\ACPI#PNP0303#4&1d401fb5&0#{884b96c3-56ef-11d1-bc8c-00a0c91405dd}"), None);
        assert_eq!(vid_pid("VID_12&PID_3456"), None);
        assert_eq!(vid_pid("VID_ZZZZ&PID_0001"), None);
    }

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
