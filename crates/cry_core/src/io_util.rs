use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

pub struct CgfUtil;

impl CgfUtil {
    pub fn write_temp_rename(target_path: &Path, data: &[u8]) -> io::Result<()> {
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp_path = target_path.with_extension("$tmp$");
        {
            let mut file = File::create(&temp_path)?;
            file.write_all(data)?;
            file.flush()?;
        }
        if target_path.exists() {
            let _ = fs::remove_file(target_path);
        }
        fs::rename(&temp_path, target_path)
    }

    pub fn sync_file_time(source: &Path, target: &Path) {
        if let Ok(meta) = fs::metadata(source) {
            let mtime = filetime::FileTime::from_last_modification_time(&meta);
            let atime = filetime::FileTime::from_last_access_time(&meta);
            let _ = filetime::set_file_times(target, atime, mtime);
        }
    }
}
