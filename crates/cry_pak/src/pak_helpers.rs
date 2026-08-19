use std::cmp::Ordering;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ESortType {
    NoSort,
    Size,
    Streaming,
    Suffix,
    Alphabetically,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ESplitType {
    Original,
    Basedir,
    ExtensionMipmap,
    Suffix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ETextureType {
    Diffuse,
    Normal,
    Specular,
    Detail,
    Mask,
    SubSurfaceScattering,
    Cubemap,
    Colorchart,
    Displacement,
    Undefined,
}

#[derive(Debug, Clone)]
pub struct PakEntry {
    pub file_path: PathBuf,
    pub file_size: u64,
    pub is_last_mip: bool,
    pub streaming_suffix: String,
    pub extension: String,
    pub texture_type: ETextureType,
    pub base_name: String,
    pub inner_dir: String,
}

impl PakEntry {
    pub fn from_path(path: &Path, file_size: u64) -> Self {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
        let ext = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let (base_name, streaming_suffix) = Self::parse_streaming_suffix(&file_name);
        let texture_type = Self::detect_texture_type(&base_name);

        Self {
            file_path: path.to_path_buf(),
            file_size,
            is_last_mip: false,
            streaming_suffix,
            extension: ext,
            texture_type,
            base_name,
            inner_dir: path
                .parent()
                .unwrap_or(Path::new(""))
                .to_string_lossy()
                .to_string(),
        }
    }

    pub fn parse_streaming_suffix(filename: &str) -> (String, String) {
        if let Some(pos) = filename.rfind('.') {
            let suffix = &filename[pos + 1..];
            if suffix.chars().all(|c| c.is_ascii_digit() || c == 'a') {
                return (filename[..pos].to_string(), suffix.to_string());
            }
        }
        (filename.to_string(), String::new())
    }

    pub fn detect_texture_type(name: &str) -> ETextureType {
        let lower = name.to_ascii_lowercase();
        if lower.ends_with("_diff") {
            ETextureType::Diffuse
        } else if lower.ends_with("_ddn") || lower.ends_with("_ddna") {
            ETextureType::Normal
        } else if lower.ends_with("_spec") {
            ETextureType::Specular
        } else if lower.ends_with("_detail") {
            ETextureType::Detail
        } else if lower.ends_with("_mask") {
            ETextureType::Mask
        } else if lower.ends_with("_sss") {
            ETextureType::SubSurfaceScattering
        } else if lower.ends_with("_cm") || lower.ends_with("_cubemap") {
            ETextureType::Cubemap
        } else if lower.ends_with("_cch") {
            ETextureType::Colorchart
        } else if lower.ends_with("_displ") || lower.ends_with("_dmap") {
            ETextureType::Displacement
        } else {
            ETextureType::Undefined
        }
    }
}

pub struct PakSorter;

impl PakSorter {
    pub fn sort_entries(entries: &mut [PakEntry], sort_type: ESortType) {
        match sort_type {
            ESortType::Alphabetically => {
                entries.sort_by(|a, b| a.file_path.cmp(&b.file_path));
            }
            ESortType::Size => {
                entries.sort_by_key(|b| std::cmp::Reverse(b.file_size));
            }
            ESortType::Streaming => {
                entries.sort_by(|a, b| {
                    let ext_cmp = a.extension.cmp(&b.extension);
                    if ext_cmp != Ordering::Equal {
                        return ext_cmp;
                    }
                    let type_cmp = a.texture_type.cmp(&b.texture_type);
                    if type_cmp != Ordering::Equal {
                        return type_cmp;
                    }
                    a.base_name.cmp(&b.base_name)
                });
            }
            ESortType::Suffix => {
                entries.sort_by(|a, b| {
                    let sfx_cmp = a.streaming_suffix.cmp(&b.streaming_suffix);
                    if sfx_cmp != Ordering::Equal {
                        return sfx_cmp;
                    }
                    a.base_name.cmp(&b.base_name)
                });
            }
            ESortType::NoSort => {}
        }
    }
}
