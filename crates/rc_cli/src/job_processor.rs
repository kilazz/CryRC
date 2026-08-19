use crate::extension_manager::ExtensionManager;
use cry_asset::AssetManager;
use cry_core::name_converter::matches_wildcards_ignore_case;
use cry_core::{DependencyList, PropertyVars};
use cry_pak::{PakFileInfo, PakWriter};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, Default)]
pub struct RCJobTask {
    pub name: String,
    pub source_root: PathBuf,
    pub target_root: PathBuf,
    pub files: Vec<String>,
    pub recursive: bool,
    pub options: String,
    pub zip_archive: Option<PathBuf>,
    pub clean_target_root: bool,
    pub properties: PropertyVars,
}

pub struct JobProcessor;

impl JobProcessor {
    pub fn load_job_script(
        job_file: &Path,
        initial_props: &PropertyVars,
    ) -> Result<Vec<RCJobTask>, String> {
        let mut visited_files = HashSet::new();
        let mut global_props = initial_props.clone();
        let mut tasks = Vec::new();
        Self::parse_job_file_recursive(
            job_file,
            &mut global_props,
            &mut tasks,
            &mut visited_files,
        )?;
        Ok(tasks)
    }

    fn parse_job_file_recursive(
        job_file: &Path,
        current_props: &mut PropertyVars,
        tasks: &mut Vec<RCJobTask>,
        visited: &mut HashSet<PathBuf>,
    ) -> Result<(), String> {
        let canonical_path = job_file
            .canonicalize()
            .map_err(|e| format!("Failed to locate job file {:?}: {}", job_file, e))?;

        if !visited.insert(canonical_path) {
            return Err(format!(
                "Circular <Include> detected in job file: {:?}",
                job_file
            ));
        }

        let content = fs::read_to_string(job_file)
            .map_err(|e| format!("Failed to read job file {:?}: {}", job_file, e))?;

        let base_dir = job_file.parent().unwrap_or(Path::new(""));
        let mut reader = Reader::from_str(&content);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut condition_stack: Vec<bool> = Vec::new();
        let mut current_job_name = String::new();
        let mut in_properties = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let is_active = condition_stack.iter().all(|&c| c);

                    match tag.as_str() {
                        "If" => {
                            let mut cond_expr = String::new();
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"Condition"
                                    || attr.key.as_ref() == b"condition"
                                {
                                    cond_expr = String::from_utf8_lossy(&attr.value).to_string();
                                }
                            }
                            let _ = current_props.expand_properties(&mut cond_expr);
                            let passed = Self::evaluate_condition(&cond_expr);
                            condition_stack.push(passed);
                        }
                        "Include" if is_active => {
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"File" || attr.key.as_ref() == b"file" {
                                    let mut inc_file_str =
                                        String::from_utf8_lossy(&attr.value).to_string();
                                    let _ = current_props.expand_properties(&mut inc_file_str);
                                    let inc_path = base_dir.join(&inc_file_str);
                                    Self::parse_job_file_recursive(
                                        &inc_path,
                                        current_props,
                                        tasks,
                                        visited,
                                    )?;
                                }
                            }
                        }
                        "DefaultProperties" | "Properties" if is_active => {
                            in_properties = true;
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                                let mut val = String::from_utf8_lossy(&attr.value).to_string();
                                let _ = current_props.expand_properties(&mut val);
                                current_props.set_property(&key, &val);
                            }
                        }
                        "Property" if is_active && in_properties => {
                            let mut name = String::new();
                            let mut val = String::new();
                            for attr in e.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"Name" | b"name" => {
                                        name = String::from_utf8_lossy(&attr.value).to_string();
                                    }
                                    b"Value" | b"value" => {
                                        val = String::from_utf8_lossy(&attr.value).to_string();
                                    }
                                    _ => {}
                                }
                            }
                            if !name.is_empty() {
                                let _ = current_props.expand_properties(&mut val);
                                current_props.set_property(&name, &val);
                            }
                        }
                        "Job" if is_active => {
                            current_job_name = String::new();
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"Name" || attr.key.as_ref() == b"id" {
                                    current_job_name =
                                        String::from_utf8_lossy(&attr.value).to_string();
                                    let _ = current_props.expand_properties(&mut current_job_name);
                                }
                            }
                        }
                        "Run" | "Execute" | "Task" if is_active => {
                            let mut task = RCJobTask {
                                name: current_job_name.clone(),
                                properties: current_props.clone(),
                                ..Default::default()
                            };

                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                                let mut val = String::from_utf8_lossy(&attr.value).to_string();
                                let _ = current_props.expand_properties(&mut val);

                                match key.to_ascii_lowercase().as_str() {
                                    "sourceroot" => task.source_root = PathBuf::from(val),
                                    "targetroot" => task.target_root = PathBuf::from(val),
                                    "files" => {
                                        task.files = val
                                            .split([';', ','])
                                            .map(|s| s.trim().to_string())
                                            .collect()
                                    }
                                    "recursive" => {
                                        task.recursive =
                                            val == "1" || val.eq_ignore_ascii_case("true")
                                    }
                                    "options" => task.options = val,
                                    "zip" => task.zip_archive = Some(PathBuf::from(val)),
                                    "clean_targetroot" => {
                                        task.clean_target_root =
                                            val == "1" || val.eq_ignore_ascii_case("true")
                                    }
                                    _ => {
                                        task.properties.set_property(&key, &val);
                                    }
                                }
                            }

                            if task.files.is_empty() {
                                task.files.push("*.*".to_string());
                            }
                            tasks.push(task);
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    match tag.as_str() {
                        "If" => {
                            condition_stack.pop();
                        }
                        "DefaultProperties" | "Properties" => in_properties = false,
                        "Job" => current_job_name.clear(),
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(err) => return Err(format!("Error parsing Job XML {:?}: {}", job_file, err)),
                _ => {}
            }
            buf.clear();
        }
        Ok(())
    }

    fn evaluate_condition(expr: &str) -> bool {
        let clean = expr.trim();
        if clean.is_empty() {
            return true;
        }

        if let Some(pos) = clean.find("==") {
            let left = clean[..pos].trim().trim_matches(|c| c == '\'' || c == '"');
            let right = clean[pos + 2..]
                .trim()
                .trim_matches(|c| c == '\'' || c == '"');
            left.eq_ignore_ascii_case(right)
        } else if let Some(pos) = clean.find("!=") {
            let left = clean[..pos].trim().trim_matches(|c| c == '\'' || c == '"');
            let right = clean[pos + 2..]
                .trim()
                .trim_matches(|c| c == '\'' || c == '"');
            !left.eq_ignore_ascii_case(right)
        } else {
            match clean.to_ascii_lowercase().as_str() {
                "1" | "true" | "yes" | "enable" => true,
                "0" | "false" | "no" | "disable" => false,
                _ => !clean.is_empty(),
            }
        }
    }

    pub fn execute_job_tasks(
        tasks: &[RCJobTask],
        ext_mgr: &ExtensionManager,
        _asset_mgr: &mut AssetManager,
        verbosity: u8,
    ) -> Result<usize, String> {
        let mut total_compiled = 0;

        for (idx, task) in tasks.iter().enumerate() {
            if verbosity >= 1 {
                println!(
                    "\n===============================================================================\n\
                     [RC JOB Task {}/{}] '{}' (Source: {:?} -> Target: {:?})\n\
                     ===============================================================================",
                    idx + 1,
                    tasks.len(),
                    if task.name.is_empty() {
                        "BatchJob"
                    } else {
                        &task.name
                    },
                    task.source_root,
                    task.target_root
                );
            }

            let input_files =
                Self::collect_matching_files(&task.source_root, &task.files, task.recursive);
            if input_files.is_empty() {
                continue;
            }

            let dependency_tracker = Mutex::new(DependencyList::new());
            let compiled_count = AtomicUsize::new(0);

            input_files.par_iter().for_each(|input_path| {
                let rel_path = input_path
                    .strip_prefix(&task.source_root)
                    .unwrap_or(input_path);
                let target_path = task.target_root.join(rel_path);

                if let Some(parent) = target_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }

                let ext = input_path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if ext_mgr.find_converter(&ext).is_some() || ext == "mtl" {
                    let mut tracker = dependency_tracker.lock().unwrap();
                    tracker.add(input_path, &target_path);
                    compiled_count.fetch_add(1, Ordering::Relaxed);
                }
            });

            let compiled = compiled_count.load(Ordering::Relaxed);
            total_compiled += compiled;

            if let Some(ref pak_path) = task.zip_archive {
                let tracker = dependency_tracker.lock().unwrap();
                let files_to_pack: Vec<PakFileInfo> = tracker
                    .files
                    .iter()
                    .map(|pair| {
                        let rel = pair
                            .output_file
                            .strip_prefix(&task.target_root)
                            .unwrap_or(&pair.output_file);
                        PakFileInfo {
                            relative_path: rel.to_string_lossy().replace('\\', "/"),
                            disk_path: pair.output_file.clone(),
                        }
                    })
                    .collect();

                let _ = PakWriter::create_pak(pak_path, &files_to_pack, 1, false, None);
            }
        }

        Ok(total_compiled)
    }

    fn collect_matching_files(dir: &Path, patterns: &[String], recursive: bool) -> Vec<PathBuf> {
        let mut results = Vec::new();
        Self::scan_and_filter(dir, patterns, recursive, &mut results);
        results
    }

    fn scan_and_filter(
        dir: &Path,
        patterns: &[String],
        recursive: bool,
        results: &mut Vec<PathBuf>,
    ) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let filename = path.file_name().unwrap_or_default().to_string_lossy();
                    let matches = patterns.iter().any(|pat| {
                        pat == "*.*" || pat == "*" || matches_wildcards_ignore_case(&filename, pat)
                    });
                    if matches {
                        results.push(path);
                    }
                } else if path.is_dir() && recursive {
                    Self::scan_and_filter(&path, patterns, recursive, results);
                }
            }
        }
    }
}
