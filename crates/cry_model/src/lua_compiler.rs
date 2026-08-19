use std::fs;
use std::path::{Path, PathBuf};

pub struct LuaCompiler {
    pub strip_debug_info: bool,
}

impl LuaCompiler {
    pub fn new(strip_debug_info: bool) -> Self {
        Self { strip_debug_info }
    }

    pub fn process(&self, source_path: &Path, output_path: &Path) -> Result<Vec<PathBuf>, String> {
        let content = fs::read_to_string(source_path)
            .map_err(|e| format!("Failed to read Lua script: {}", e))?;

        let mut stripped_lines = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("--") && !trimmed.is_empty() {
                stripped_lines.push(line);
            }
        }

        let output_text = stripped_lines.join("\n");
        if let Some(parent) = output_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        fs::write(output_path, output_text.as_bytes())
            .map_err(|e| format!("Failed to write compiled Lua: {}", e))?;

        Ok(vec![output_path.to_path_buf()])
    }
}
