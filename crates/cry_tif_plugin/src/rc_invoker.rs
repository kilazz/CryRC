// Copyright 2004-2026 Crytek GmbH / Crytek Group. All rights reserved.
// CryEngine Resource Compiler Invocation and Registry Discovery

use std::path::{Path, PathBuf};
use std::process::Command;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};

const ERROR_SUCCESS: u32 = 0;
const REG_SETTINGS_KEY: &str = "Software\\Crytek\\Settings\0";

#[derive(Debug, Clone, Default)]
pub struct EngineSettings {
    pub root_path: PathBuf,
    pub rc_parameters: String,
    pub show_window: bool,
}

impl EngineSettings {
    /// Loads CryEngine settings from Windows Registry (`HKCU\Software\Crytek\Settings`).
    pub fn load_from_registry() -> Self {
        let mut settings = Self::default();

        let key_wide: Vec<u16> = REG_SETTINGS_KEY.encode_utf16().collect();
        let mut h_key: HKEY = unsafe { std::mem::zeroed() };

        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                key_wide.as_ptr(),
                0,
                KEY_READ,
                &mut h_key,
            )
        };

        if status == ERROR_SUCCESS {
            if let Some(val) = Self::read_reg_string(h_key, "ENG_RootPath") {
                settings.root_path = PathBuf::from(val);
            }
            if let Some(val) = Self::read_reg_string(h_key, "RC_Parameters") {
                settings.rc_parameters = val;
            }
            if let Some(val) = Self::read_reg_string(h_key, "RC_ShowWindow") {
                settings.show_window = val.eq_ignore_ascii_case("true") || val == "1";
            }
            unsafe {
                RegCloseKey(h_key);
            }
        }

        settings
    }

    fn read_reg_string(key: HKEY, value_name: &str) -> Option<String> {
        let name_wide: Vec<u16> = value_name.encode_utf16().chain(Some(0)).collect();
        let mut data_size: u32 = 0;
        let mut val_type: u32 = 0;

        let status = unsafe {
            RegQueryValueExW(
                key,
                name_wide.as_ptr(),
                std::ptr::null_mut(),
                &mut val_type,
                std::ptr::null_mut(),
                &mut data_size,
            )
        };

        if status == ERROR_SUCCESS && data_size > 0 {
            let mut buffer = vec![0u8; data_size as usize];
            let read_status = unsafe {
                RegQueryValueExW(
                    key,
                    name_wide.as_ptr(),
                    std::ptr::null_mut(),
                    &mut val_type,
                    buffer.as_mut_ptr(),
                    &mut data_size,
                )
            };

            if read_status == ERROR_SUCCESS {
                let u16_slice = unsafe {
                    std::slice::from_raw_parts(
                        buffer.as_ptr() as *const u16,
                        (data_size as usize) / 2,
                    )
                };
                let text = String::from_utf16_lossy(
                    u16_slice.split(|&c| c == 0).next().unwrap_or(u16_slice),
                );
                return Some(text);
            }
        }
        None
    }
}

pub struct RcInvoker;

impl RcInvoker {
    /// Discovers the absolute path to `rc.exe` across environment variables, registry, plugin folder, and PATH.
    pub fn find_rc_executable() -> Option<PathBuf> {
        // 1. Direct environment variable override
        if let Ok(env_path) = std::env::var("CRYENGINE_RC_PATH") {
            let p = PathBuf::from(env_path);
            if p.exists() {
                return Some(p);
            }
        }

        // 2. Check engine root path from Registry (ENG_RootPath)
        let settings = EngineSettings::load_from_registry();
        if !settings.root_path.as_os_str().is_empty() {
            let candidates = [
                settings.root_path.join("Tools").join("rc").join("rc.exe"),
                settings
                    .root_path
                    .join("bin")
                    .join("win_x64")
                    .join("rc")
                    .join("rc.exe"),
                settings.root_path.join("rc.exe"),
            ];
            for c in &candidates {
                if c.exists() {
                    return Some(c.clone());
                }
            }
        }

        // 3. Check current executable directory & parent directories
        if let Ok(mut cur) = std::env::current_exe() {
            while cur.pop() {
                let candidate = cur.join("Tools").join("rc").join("rc.exe");
                if candidate.exists() {
                    return Some(candidate);
                }
                let local_candidate = cur.join("rc.exe");
                if local_candidate.exists() {
                    return Some(local_candidate);
                }
            }
        }

        // 4. Check Current Directory
        if let Ok(cur_dir) = std::env::current_dir() {
            let local_rc = cur_dir.join("rc.exe");
            if local_rc.exists() {
                return Some(local_rc);
            }
        }

        None
    }

    /// Launches Resource Compiler modal processing for the saved TIFF.
    pub fn invoke_rc(tif_path: &Path, override_filename: &str) -> Result<(), String> {
        let rc_exe = Self::find_rc_executable().ok_or_else(|| {
            Self::show_error_dialog(
                "ResourceCompiler (rc.exe) was not found.\n\n\
                Please set the CRYENGINE_RC_PATH environment variable to your rc.exe path,\n\
                or run Tools\\SettingsMgr.exe to configure your Engine RootPath.",
            );
            "rc.exe not found".to_string()
        })?;

        // Convert to clean absolute path without extended Windows prefix
        let abs_tif_path = if tif_path.is_absolute() {
            tif_path.to_path_buf()
        } else if let Ok(canon) = std::fs::canonicalize(tif_path) {
            canon
        } else if let Ok(cur_dir) = std::env::current_dir() {
            cur_dir.join(tif_path)
        } else {
            tif_path.to_path_buf()
        };

        let path_str = abs_tif_path.to_string_lossy();
        let clean_path = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str);

        let settings = EngineSettings::load_from_registry();
        let mut cmd = Command::new(&rc_exe);

        cmd.arg(clean_path);
        cmd.arg("/userdialog=1");
        cmd.arg("/refresh=1");
        cmd.arg(format!("/overwritefilename={}", override_filename));
        cmd.arg("/overwriteextension=tif");

        if !settings.rc_parameters.is_empty() {
            for param in settings.rc_parameters.split_whitespace() {
                cmd.arg(param);
            }
        }

        if let Some(rc_dir) = rc_exe.parent() {
            cmd.current_dir(rc_dir);
        }

        match cmd.status() {
            Ok(status) if status.success() => Ok(()),
            Ok(status) => {
                let err_msg = format!("rc.exe exited with code: {:?}", status.code());
                Self::show_error_dialog(&err_msg);
                Err(err_msg)
            }
            Err(e) => {
                let err_msg = format!("Failed to execute rc.exe at {:?}: {}", rc_exe, e);
                Self::show_error_dialog(&err_msg);
                Err(err_msg)
            }
        }
    }

    pub fn show_error_dialog(message: &str) {
        let msg_wide: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
        let title_wide: Vec<u16> = "CryTIF Plugin Error\0".encode_utf16().collect();
        unsafe {
            MessageBoxW(
                std::ptr::null_mut() as HWND,
                msg_wide.as_ptr(),
                title_wide.as_ptr(),
                MB_ICONERROR | MB_OK,
            );
        }
    }
}
