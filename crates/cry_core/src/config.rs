use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum EConfigPriority {
    Lowest = 1 << 0,
    File = 1 << 1,
    Preset = 1 << 2,
    RcIni = 1 << 3,
    Cmdline = 1 << 4,
    Property = 1 << 5,
    Job = 1 << 6,
    Highest = 1 << 7,
}

impl EConfigPriority {
    pub const ALL: u32 = 0xFF;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ConfigKey {
    pub name: String,
    pub priority: EConfigPriority,
}

impl Ord for ConfigKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let name_cmp = self
            .name
            .to_ascii_lowercase()
            .cmp(&other.name.to_ascii_lowercase());
        if name_cmp != std::cmp::Ordering::Equal {
            name_cmp
        } else {
            self.priority.cmp(&other.priority)
        }
    }
}

impl PartialOrd for ConfigKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    map: BTreeMap<ConfigKey, String>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    pub fn set_key_value(&mut self, pri: EConfigPriority, key: &str, value: &str) {
        if key.trim().is_empty() {
            return;
        }
        let k = ConfigKey {
            name: key.trim().to_string(),
            priority: pri,
        };
        self.map.insert(k, value.trim().to_string());
    }

    pub fn get_key_value(&self, key: &str, pri_mask: u32) -> Option<&str> {
        let key_lower = key.to_ascii_lowercase();
        for (k, val) in self.map.iter().rev() {
            if (k.priority as u32 & pri_mask) != 0 && k.name.to_ascii_lowercase() == key_lower {
                return Some(val.as_str());
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct CfgEntry {
    pub key: String,
    pub value: String,
}

impl CfgEntry {
    pub fn is_comment(&self) -> bool {
        let trimmed = self.value.trim();
        trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with(';')
    }
}

#[derive(Debug, Clone)]
pub struct CfgSection {
    pub name: String,
    pub entries: Vec<CfgEntry>,
}

#[derive(Debug, Clone, Default)]
pub struct CfgFile {
    pub file_name: String,
    pub sections: Vec<CfgSection>,
}

impl CfgFile {
    pub fn new() -> Self {
        let mut cfg = Self {
            file_name: String::new(),
            sections: Vec::new(),
        };
        cfg.sections.push(CfgSection {
            name: String::new(),
            entries: Vec::new(),
        });
        cfg
    }

    pub fn load_from_file(&mut self, path: &Path) -> io::Result<()> {
        let content = fs::read_to_string(path)?;
        self.file_name = path.to_string_lossy().to_string();
        self.load_from_str(&content);
        Ok(())
    }

    pub fn load_from_str(&mut self, content: &str) {
        self.sections.clear();
        self.sections.push(CfgSection {
            name: String::new(),
            entries: Vec::new(),
        });
        let mut current_sec_idx = 0;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with("//") {
                self.sections[current_sec_idx].entries.push(CfgEntry {
                    key: String::new(),
                    value: line.to_string(),
                });
                continue;
            }

            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                let sec_name = trimmed[1..trimmed.len() - 1].trim().to_string();
                self.sections.push(CfgSection {
                    name: sec_name,
                    entries: Vec::new(),
                });
                current_sec_idx = self.sections.len() - 1;
            } else if let Some(splitter) = trimmed.find('=') {
                let k = trimmed[..splitter].trim().to_string();
                let v = trimmed[splitter + 1..].trim().to_string();
                self.sections[current_sec_idx]
                    .entries
                    .push(CfgEntry { key: k, value: v });
            }
        }
    }

    pub fn find_section(&self, section_name: &str) -> Option<usize> {
        self.sections
            .iter()
            .position(|s| s.name.eq_ignore_ascii_case(section_name))
    }
}

#[derive(Debug, Clone, Default)]
pub struct MultiplatformConfig {
    pub platforms: Vec<String>,
    pub active_platform_idx: usize,
    pub configs: Vec<Config>,
}

impl MultiplatformConfig {
    pub fn new(platforms: Vec<String>, active_idx: usize) -> Self {
        let count = platforms.len();
        let configs = vec![Config::new(); count.max(1)];
        Self {
            platforms,
            active_platform_idx: active_idx,
            configs,
        }
    }

    pub fn set_key_value_all(&mut self, pri: EConfigPriority, key: &str, val: &str) {
        for cfg in &mut self.configs {
            cfg.set_key_value(pri, key, val);
        }
    }
}
