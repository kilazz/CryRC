use super::name_converter::matches_wildcards_ignore_case;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub struct ListFile;

impl ListFile {
    pub fn process_list_file(
        list_file_path: &Path,
        wildcard_filters: &[String],
        formats: &[String],
        default_folder: &Path,
    ) -> Result<Vec<(PathBuf, PathBuf)>, String> {
        let file = File::open(list_file_path)
            .map_err(|e| format!("Failed to open list file {:?}: {}", list_file_path, e))?;
        let reader = BufReader::new(file);

        let mut results = Vec::new();
        let active_formats = if formats.is_empty() {
            vec!["{0}".to_string()]
        } else {
            formats.to_vec()
        };

        for line in reader.lines().map_while(Result::ok) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                continue;
            }

            let candidate_path = PathBuf::from(trimmed);
            let filename = candidate_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();

            let matched = if wildcard_filters.is_empty() {
                true
            } else {
                wildcard_filters
                    .iter()
                    .any(|w| matches_wildcards_ignore_case(&filename, w))
            };

            if matched {
                for fmt in &active_formats {
                    let formatted_name = fmt.replace("{0}", &filename);
                    let folder = candidate_path.parent().unwrap_or(default_folder);
                    results.push((folder.to_path_buf(), PathBuf::from(formatted_name)));
                }
            }
        }
        Ok(results)
    }
}
