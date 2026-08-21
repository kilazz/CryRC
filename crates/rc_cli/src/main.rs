// Copyright 2001-2026 Crytek GmbH / Crytek Group. All rights reserved.
// CryEngine Resource Compiler (Rust Modular CLI Entrypoint)

mod crytif_gui;
mod extension_manager;
mod job_processor;

use clap::Parser;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use cry_alembic::AlembicCompiler;
use cry_asset::{AssetManager, CDictionary};
use cry_core::io_util::CgfUtil;
use cry_core::{
    CfgFile, DependencyList, EConfigPriority, ListFile, MultiplatformConfig, NameConverter,
    PropertyVars,
};
use cry_image::{ImageCompiler, ImageProperties, TextureSplitter, TextureSplitterConfig};
use cry_model::fbx::PureFbxScene;
use cry_model::{
    CharacterCompiler, ChunkCompiler, ColladaCompiler, LuaCompiler, StatCGFCompiler,
    StatCGFCompilerConfig,
};
use cry_pak::{PakFileInfo, PakWriter};
use cry_substance::{ISubstancePreset, SubstanceCompiler, SubstanceConverter};
use cry_xml::{ConvertContext, XMLCompiler, XmlFilter};

use extension_manager::ExtensionManager;
use job_processor::JobProcessor;

/// CryEngine Resource Compiler Command-Line Arguments
#[derive(Parser, Debug, Clone)]
#[command(
    name = "rc",
    author = "CryEngine Rust Team",
    version = "1.2.0",
    about = "CryEngine Resource Compiler CLI (Rust)"
)]
pub struct CliArgs {
    #[arg(help = "Source asset file, directory, or wildcard pattern")]
    pub source: Option<PathBuf>,

    #[arg(
        long = "job",
        alias = "RCJob",
        help = "Path to batch Job XML script (/job=filename.xml)"
    )]
    pub job: Option<PathBuf>,

    #[arg(
        short = 'o',
        long = "output",
        alias = "targetroot",
        help = "Target output file or directory (/targetroot=path)"
    )]
    pub output: Option<PathBuf>,

    #[arg(
        long = "sourceroot",
        help = "Root folder for source game assets (/sourceroot=path)"
    )]
    pub source_root: Option<PathBuf>,

    #[arg(
        short = 'p',
        long = "platform",
        default_value = "pc",
        help = "Target platform name (pc, ps4, xboxone, es3)"
    )]
    pub platform: String,

    #[arg(
        long = "xmlfilterfile",
        help = "Path to xmlfilter.txt configuration file"
    )]
    pub xml_filter_file: Option<PathBuf>,

    #[arg(
        long = "bigendian",
        default_value_t = false,
        help = "Compile assets using Big-Endian byte order"
    )]
    pub big_endian: bool,

    #[arg(
        short = 'f',
        long = "force",
        alias = "refresh",
        default_value_t = false,
        help = "Force recompile ignoring timestamps (/refresh=1)"
    )]
    pub force: bool,

    #[arg(
        short = 'j',
        long = "threads",
        default_value_t = 0,
        help = "Worker thread count for parallel batch compilation"
    )]
    pub threads: usize,

    #[arg(
        short = 'v',
        long = "verbosity",
        default_value_t = 1,
        help = "Logging verbosity level"
    )]
    pub verbosity: u8,

    #[arg(
        short = 'r',
        long = "recursive",
        default_value_t = false,
        help = "Recursively process input folders"
    )]
    pub recursive: bool,

    #[arg(
        long = "cryasset",
        default_value_t = false,
        help = "Generate .cryasset metadata manifests"
    )]
    pub cryasset: bool,

    #[arg(
        long = "stripMetadata",
        alias = "stripmetadata",
        default_value_t = false,
        help = "Strip and remove .cryasset metadata files"
    )]
    pub strip_metadata: bool,

    #[arg(
        long = "split",
        alias = "streaming",
        default_value_t = false,
        help = "Split DDS mipmaps for streaming (/streaming=1)"
    )]
    pub split_textures: bool,

    #[arg(
        long = "decompress",
        default_value_t = false,
        help = "Decompress DDS into TIF"
    )]
    pub decompress: bool,

    #[arg(
        long = "copyonly",
        default_value_t = false,
        help = "Copy source files directly without conversion"
    )]
    pub copy_only: bool,

    #[arg(
        long = "clean_targetroot",
        default_value_t = false,
        help = "Clean target folder of obsolete output files"
    )]
    pub clean_target_root: bool,

    #[arg(long = "listfile", help = "Path to asset list file (.txt)")]
    pub list_file: Option<PathBuf>,

    #[arg(
        long = "targetnameformat",
        help = "Target filename transformation rules"
    )]
    pub target_name_format: Option<String>,

    #[arg(long = "zip", help = "Package output files into a .pak archive")]
    pub zip_archive: Option<PathBuf>,

    #[arg(
        long = "zip_encrypt",
        default_value_t = false,
        help = "Encrypt .pak archive"
    )]
    pub zip_encrypt: bool,

    #[arg(
        long = "zip_encrypt_key",
        help = "128-bit hexadecimal key for .pak encryption"
    )]
    pub zip_encrypt_key: Option<String>,

    #[arg(
        long = "zip_alignment",
        default_value_t = 1,
        help = "File alignment inside .pak"
    )]
    pub zip_alignment: usize,

    // CryEngine Tool Integration Compatibility Flags
    #[arg(long = "userdialog", help = "Show interactive compilation dialog")]
    pub user_dialog: Option<String>,

    #[arg(long = "overwritefilename", help = "Override output filename")]
    pub overwrite_filename: Option<String>,

    #[arg(long = "overwriteextension", help = "Override output extension")]
    pub overwrite_extension: Option<String>,

    #[arg(long = "quiet", help = "Suppress console output")]
    pub quiet: Option<String>,

    #[arg(long = "log", help = "Log file path")]
    pub log: Option<String>,

    #[arg(long = "createmtl", help = "Create default material")]
    pub create_mtl: Option<String>,
}

#[derive(Default)]
pub struct CompileStats {
    pub total_files: usize,
    pub compiled_files: AtomicUsize,
    pub skipped_files: AtomicUsize,
    pub failed_files: AtomicUsize,
}

fn main() -> ExitCode {
    let raw_args: Vec<String> = std::env::args().collect();

    // Fast-path handling for CryEngine SettingsManager / Editor version discovery
    if raw_args.iter().any(|a| {
        a.eq_ignore_ascii_case("/version")
            || a.eq_ignore_ascii_case("-version")
            || a.eq_ignore_ascii_case("--version")
    }) {
        println!("ResourceCompiler 64-bit");
        println!("Platform support: PC, XboxOne, PS4, PowerVR, Android");
        println!("Version 1.2.0.0 (Rust Engine Edition)");
        println!("Copyright (c) 2001-2026 Crytek GmbH / CryEngine Rust Team.");
        return ExitCode::SUCCESS;
    }

    // Fast-path handling for CLI help requests
    if raw_args.iter().any(|a| {
        a.eq_ignore_ascii_case("/help")
            || a.eq_ignore_ascii_case("/?")
            || a.eq_ignore_ascii_case("-help")
            || a.eq_ignore_ascii_case("-h")
    }) {
        println!("CryEngine Resource Compiler (Rust Modular v1.2.0)");
        println!("Usage: rc.exe [source] /p=<platform> [/job=file.xml] [/refresh=1] [/threads=N]");
        return ExitCode::SUCCESS;
    }

    // Robust CryEngine slash syntax normalizer
    let mut normalized_args = Vec::with_capacity(raw_args.len());
    if let Some(exe_name) = raw_args.first() {
        normalized_args.push(exe_name.clone());
    }

    for arg in raw_args.iter().skip(1) {
        if let Some(stripped) = arg.strip_prefix('/') {
            if let Some(pos) = stripped.find('=') {
                let key = &stripped[..pos];
                let val = &stripped[pos + 1..];

                if key.eq_ignore_ascii_case("p") {
                    normalized_args.push(format!("--platform={}", val));
                } else if key.eq_ignore_ascii_case("refresh") {
                    if val == "1" || val.eq_ignore_ascii_case("true") {
                        normalized_args.push("--force".to_string());
                    }
                } else {
                    normalized_args.push(format!("--{}={}", key, val));
                }
            } else {
                if stripped.eq_ignore_ascii_case("refresh") {
                    normalized_args.push("--force".to_string());
                } else if stripped.eq_ignore_ascii_case("userdialog") {
                    normalized_args.push("--userdialog=1".to_string());
                } else {
                    normalized_args.push(format!("--{}", stripped));
                }
            }
        } else {
            normalized_args.push(arg.clone());
        }
    }

    let start_time = Instant::now();
    let args = match CliArgs::try_parse_from(normalized_args) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
    };

    if args.threads > 0 {
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(args.threads)
            .build_global();
    }

    let is_quiet = args.quiet.as_deref() == Some("1")
        || raw_args
            .iter()
            .any(|a| a.eq_ignore_ascii_case("/quiet") || a.starts_with("/quiet="));

    if args.verbosity >= 1 && !is_quiet {
        println!("===============================================================================");
        println!("CryEngine Resource Compiler (Rust Modular v1.2.0)");
        println!("===============================================================================");
    }

    // 1. Search and load rc.ini
    let mut ini_file = CfgFile::new();
    let mut ini_candidates = vec![
        PathBuf::from("rc.ini"),
        PathBuf::from("Tools/rc/rc.ini"),
        PathBuf::from("../rc.ini"),
        PathBuf::from("../../rc.ini"),
    ];

    if let Ok(exe_path) = std::env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        ini_candidates.insert(0, exe_dir.join("rc.ini"));
    }

    let mut ini_loaded = false;
    for ini_path in &ini_candidates {
        if ini_path.exists() && ini_file.load_from_file(ini_path).is_ok() {
            if args.verbosity >= 1 && !is_quiet {
                println!("[RC] Loaded configuration from {:?}", ini_path);
            }
            ini_loaded = true;
            break;
        }
    }

    if !ini_loaded && args.verbosity >= 1 && !is_quiet {
        println!("[RC WARNING] No rc.ini found, using default engine presets.");
    }

    // 2. Initialize Platform and Configuration
    let platforms = vec![
        "pc".to_string(),
        "ps4".to_string(),
        "xboxone".to_string(),
        "es3".to_string(),
    ];
    let active_platform_idx = platforms
        .iter()
        .position(|p| p.eq_ignore_ascii_case(&args.platform))
        .unwrap_or(0);

    let mut multi_config = MultiplatformConfig::new(platforms, active_platform_idx);
    multi_config.set_key_value_all(EConfigPriority::Cmdline, "platform", &args.platform);
    if args.force {
        multi_config.set_key_value_all(EConfigPriority::Cmdline, "refresh", "1");
    }
    if args.split_textures {
        multi_config.set_key_value_all(EConfigPriority::Cmdline, "streaming", "1");
    }

    // 3. Initialize AssetManager and Detail Providers
    let mut asset_manager = AssetManager::new();

    if let Some(global_sec) = ini_file.sections.first() {
        for entry in &global_sec.entries {
            if entry.key.eq_ignore_ascii_case("assettypes") {
                asset_manager = AssetManager::new();
                let dict = CDictionary::from_string(&entry.value);
                for (k, _) in dict.entries {
                    asset_manager.register_detail_provider(&k, |_p, _d, _deps| true);
                }
            }
        }
    }

    let extension_manager = ExtensionManager::new();

    asset_manager.register_detail_provider("dds", |path, details, _deps| {
        cry_image::ImageDetails::collect_dds_details(path, details).is_ok()
    });
    asset_manager.register_detail_provider("tif", |path, details, _deps| {
        cry_image::ImageDetails::collect_tif_details(path, details).is_ok()
    });
    asset_manager.register_detail_provider("cgf", |path, details, deps| {
        cry_asset::AssetCollectors::collect_cgf_details(path, details, deps);
        true
    });
    asset_manager.register_detail_provider("mtl", |path, details, deps| {
        cry_asset::AssetCollectors::collect_mtl_details(path, details, deps).is_ok()
    });
    asset_manager.register_detail_provider("xml", |path, details, _deps| {
        cry_asset::AssetCollectors::collect_xml_details(path, details).is_ok()
    });
    asset_manager.register_detail_provider("cdf", |path, _details, deps| {
        cry_asset::AssetCollectors::collect_cdf_details(path, deps).is_ok()
    });

    // 4. Batch Job XML Script Mode (/job=Job.xml)
    if let Some(ref job_file) = args.job {
        let mut initial_props = PropertyVars::new();
        initial_props.set_property("Platform", &args.platform);
        if let Some(ref sr) = args.source_root {
            initial_props.set_property("SourceRoot", &sr.to_string_lossy());
        }
        if let Some(ref tr) = args.output {
            initial_props.set_property("TargetRoot", &tr.to_string_lossy());
        }

        match JobProcessor::load_job_script(job_file, &initial_props) {
            Ok(tasks) => {
                match JobProcessor::execute_job_tasks(
                    &tasks,
                    &extension_manager,
                    &mut asset_manager,
                    args.verbosity,
                ) {
                    Ok(count) => {
                        if !is_quiet {
                            println!(
                                "[RC] Finished Job XML script in {:.2?}. Processed {} items.",
                                start_time.elapsed(),
                                count
                            );
                        }
                        return ExitCode::SUCCESS;
                    }
                    Err(e) => {
                        eprintln!("[RC ERROR] Failed executing Job XML tasks: {}", e);
                        return ExitCode::FAILURE;
                    }
                }
            }
            Err(e) => {
                eprintln!("[RC ERROR] Failed to load Job XML: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    // 5. Standard Command-Line Input Mode
    let files_to_process = collect_all_inputs(&args);
    if files_to_process.is_empty() {
        eprintln!(
            "[RC ERROR] No matching input files found at path: {:?}",
            args.source
        );
        return ExitCode::FAILURE;
    }

    let stats = CompileStats {
        total_files: files_to_process.len(),
        ..Default::default()
    };

    let xml_filter = resolve_xml_filter(&args.xml_filter_file, &args.source_root);
    let mut name_converter = NameConverter::new();
    if let Some(ref rules) = args.target_name_format {
        let _ = name_converter.set_rules(rules);
    }

    let dependency_tracker = Mutex::new(DependencyList::new());

    let is_interactive = args.user_dialog.as_deref() == Some("1")
        || args.user_dialog.as_deref() == Some("true")
        || raw_args.iter().any(|a| {
            a.starts_with("/userdialog=1")
                || a.starts_with("--userdialog=1")
                || a == "/userdialog"
                || a == "--userdialog"
        });

    let process_file_item = |input_file: &PathBuf| match process_single_file(
        input_file,
        &args,
        &extension_manager,
        xml_filter.as_ref(),
        &name_converter,
        &ini_file,
        &raw_args,
    ) {
        Ok(ProcessStatus::Compiled(out_paths)) => {
            stats.compiled_files.fetch_add(1, Ordering::Relaxed);
            if args.verbosity >= 1 && !is_quiet {
                for path in &out_paths {
                    println!(
                        "[RC OK] Compiled: {} -> {}",
                        input_file.display(),
                        path.display()
                    );
                }
            }
            {
                let mut tracker = dependency_tracker.lock().unwrap();
                for path in &out_paths {
                    tracker.add(input_file, path);
                }
            }
            if args.cryasset || args.strip_metadata {
                let out_folder = args.output.as_deref();
                let _ = asset_manager.save_cryasset(
                    input_file,
                    &out_paths,
                    out_folder,
                    args.strip_metadata,
                    "",
                );
            }
        }
        Ok(ProcessStatus::SkippedUpToDate) => {
            stats.skipped_files.fetch_add(1, Ordering::Relaxed);
            if args.verbosity >= 2 && !is_quiet {
                println!("[RC SKIP] Up-to-date: {}", input_file.display());
            }
        }
        Ok(ProcessStatus::UnsupportedExtension) => {
            if args.verbosity >= 2 && !is_quiet {
                println!(
                    "[RC IGNORE] Unsupported asset format: {}",
                    input_file.display()
                );
            }
        }
        Err(err_msg) => {
            stats.failed_files.fetch_add(1, Ordering::Relaxed);
            eprintln!(
                "[RC ERROR] Failed processing {}: {}",
                input_file.display(),
                err_msg
            );
        }
    };

    if is_interactive {
        for input_file in &files_to_process {
            process_file_item(input_file);
        }
    } else {
        files_to_process.par_iter().for_each(process_file_item);
    }

    if args.clean_target_root
        && let Some(ref target_root) = args.output
    {
        let tracker = dependency_tracker.lock().unwrap();
        clean_target_folder(target_root, &tracker, args.verbosity);
    }

    if let Some(ref pak_path) = args.zip_archive {
        let tracker = dependency_tracker.lock().unwrap();
        let encryption_key = parse_tea_key(&args.zip_encrypt_key);

        let files_to_pack: Vec<PakFileInfo> = tracker
            .files
            .iter()
            .map(|pair| {
                let rel_path = pair
                    .output_file
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                PakFileInfo {
                    relative_path: rel_path,
                    disk_path: pair.output_file.clone(),
                }
            })
            .collect();

        let _ = PakWriter::create_pak(
            pak_path,
            &files_to_pack,
            args.zip_alignment,
            args.zip_encrypt,
            encryption_key.as_ref(),
        );
    }

    let compiled = stats.compiled_files.load(Ordering::Relaxed);
    let skipped = stats.skipped_files.load(Ordering::Relaxed);
    let failed = stats.failed_files.load(Ordering::Relaxed);

    if args.verbosity >= 1 && !is_quiet {
        println!("-------------------------------------------------------------------------------");
        println!(
            "RC Finished in {:.2?}. Total: {}, Compiled: {}, Up-to-date: {}, Failed: {}",
            start_time.elapsed(),
            stats.total_files,
            compiled,
            skipped,
            failed
        );
        println!("===============================================================================");
    }

    if failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

enum ProcessStatus {
    Compiled(Vec<PathBuf>),
    SkippedUpToDate,
    UnsupportedExtension,
}

fn process_single_file(
    input_file: &Path,
    args: &CliArgs,
    ext_mgr: &ExtensionManager,
    xml_filter: Option<&XmlFilter>,
    name_converter: &NameConverter,
    ini_file: &CfgFile,
    raw_args: &[String],
) -> Result<ProcessStatus, String> {
    let ext = input_file
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if ext_mgr.find_converter(&ext).is_none() && ext != "mtl" {
        return Ok(ProcessStatus::UnsupportedExtension);
    }

    let mut output_file = resolve_output_path(input_file, args, &ext)?;
    if name_converter.has_rules()
        && let Some(file_name) = output_file.file_name().and_then(|s| s.to_str())
    {
        let converted = name_converter.convert_name(file_name);
        output_file.set_file_name(converted);
    }

    let is_user_dialog = args.user_dialog.as_deref() == Some("1")
        || args.user_dialog.as_deref() == Some("true")
        || raw_args.iter().any(|a| {
            a.starts_with("/userdialog=1")
                || a.starts_with("--userdialog=1")
                || a == "/userdialog"
                || a == "--userdialog"
        });

    if !args.force && !is_user_dialog && is_file_up_to_date(input_file, &output_file) {
        return Ok(ProcessStatus::SkippedUpToDate);
    }

    if let Some(parent) = output_file.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if args.copy_only {
        fs::copy(input_file, &output_file).map_err(|e| format!("Failed to copy file: {}", e))?;
        CgfUtil::sync_file_time(input_file, &output_file);
        return Ok(ProcessStatus::Compiled(vec![output_file]));
    }

    match ext.as_str() {
        "xml" => {
            let ctx = ConvertContext {
                source_path: input_file,
                output_path: &output_file,
                filter: xml_filter,
                need_swap_endian: args.big_endian,
                force_recompile: args.force,
            };

            let compiler = XMLCompiler::new(ctx);
            compiler
                .process()
                .map_err(|e| format!("XML Compiler error: {}", e))?;
            CgfUtil::sync_file_time(input_file, &output_file);
            Ok(ProcessStatus::Compiled(vec![output_file]))
        }
        "crysub" => {
            let game_root = args
                .source_root
                .clone()
                .unwrap_or_else(|| input_file.parent().unwrap_or(input_file).to_path_buf());

            let out_folder = if output_file.is_dir() {
                output_file.clone()
            } else {
                output_file.parent().unwrap_or(&output_file).to_path_buf()
            };

            let mut converter = SubstanceConverter::new(game_root);
            let mut compiler = SubstanceCompiler::new(
                &mut converter,
                input_file.to_path_buf(),
                out_folder.clone(),
                args.force,
            );

            struct DummyPreset<'a>(&'a Path);
            impl<'a> ISubstancePreset for DummyPreset<'a> {
                fn get_file_name(&self) -> &str {
                    self.0.to_str().unwrap_or_default()
                }
                fn get_substance_archive(&self) -> &str {
                    ""
                }
                fn get_outputs(&self) -> Vec<cry_substance::SubstanceOutput> {
                    Vec::new()
                }
            }

            let dummy = DummyPreset(input_file);
            compiler
                .process(&dummy)
                .map_err(|e| format!("Substance Compiler error: {}", e))?;
            Ok(ProcessStatus::Compiled(vec![out_folder]))
        }
        "abc" => {
            let mut compiler = AlembicCompiler::new();
            compiler
                .compile(input_file, &output_file)
                .map_err(|e| format!("Alembic Compiler error: {}", e))?;
            CgfUtil::sync_file_time(input_file, &output_file);
            Ok(ProcessStatus::Compiled(vec![output_file]))
        }
        "i_caf" | "caf" => {
            let compiler = cry_model::cga::animation_compiler::AnimationCompiler::new();
            compiler
                .compile(input_file, &output_file)
                .map_err(|e| format!("Animation Compiler error: {}", e))?;
            CgfUtil::sync_file_time(input_file, &output_file);
            Ok(ProcessStatus::Compiled(vec![output_file]))
        }
        "cgf" | "cga" | "i_cgf" => {
            let config = StatCGFCompilerConfig {
                split_lods: args.recursive,
                use_qtangents: true,
                ..Default::default()
            };
            let compiler = StatCGFCompiler::new(config);
            let out_files = compiler
                .process(input_file, &output_file)
                .map_err(|e| format!("StatCGF Compiler error: {}", e))?;
            for path in &out_files {
                CgfUtil::sync_file_time(input_file, path);
            }
            Ok(ProcessStatus::Compiled(out_files))
        }
        "chr" | "skin" | "cdf" => {
            let compiler = CharacterCompiler::new();
            let out_files = compiler
                .process(input_file, &output_file)
                .map_err(|e| format!("Character Compiler error: {}", e))?;
            for path in &out_files {
                CgfUtil::sync_file_time(input_file, path);
            }
            Ok(ProcessStatus::Compiled(out_files))
        }
        "json" | "fbx" => {
            let req = cry_model::fbx::ImportRequest::load_from_file(input_file)
                .map_err(|e| format!("ImportRequest error: {}", e))?;
            let real_scene = PureFbxScene::load_from_file(input_file)
                .map_err(|e| format!("FBX Parsing error: {}", e))?;
            cry_model::fbx::FbxConverter::convert_scene(&real_scene, &req, &output_file)
                .map_err(|e| format!("FBX Conversion error: {}", e))?;
            CgfUtil::sync_file_time(input_file, &output_file);
            Ok(ProcessStatus::Compiled(vec![output_file]))
        }
        "dae" => {
            let compiler = ColladaCompiler::new();
            let out_files = compiler
                .process(input_file, &output_file)
                .map_err(|e| format!("Collada Compiler error: {}", e))?;
            for path in &out_files {
                CgfUtil::sync_file_time(input_file, path);
            }
            Ok(ProcessStatus::Compiled(out_files))
        }
        "lua" => {
            let compiler = LuaCompiler::new(true);
            let out_files = compiler
                .process(input_file, &output_file)
                .map_err(|e| format!("Lua Compiler error: {}", e))?;
            for path in &out_files {
                CgfUtil::sync_file_time(input_file, path);
            }
            Ok(ProcessStatus::Compiled(out_files))
        }
        "chunk" => {
            let compiler = ChunkCompiler::new(0x0746);
            let out_files = compiler
                .process(input_file, &output_file)
                .map_err(|e| format!("Chunk Compiler error: {}", e))?;
            for path in &out_files {
                CgfUtil::sync_file_time(input_file, path);
            }
            Ok(ProcessStatus::Compiled(out_files))
        }
        "tif" | "tiff" => {
            if is_user_dialog {
                let accepted =
                    crytif_gui::CryTifGui::run_dialog(input_file, &output_file, ini_file)
                        .map_err(|e| format!("CryTIF GUI error: {}", e))?;

                if !accepted {
                    return Ok(ProcessStatus::SkippedUpToDate);
                }
                CgfUtil::sync_file_time(input_file, &output_file);
                return Ok(ProcessStatus::Compiled(vec![output_file]));
            }

            let img_props = ImageProperties {
                input_color_space: cry_image::EInputColorSpace::Linear,
                ..Default::default()
            };
            let mut compiler = ImageCompiler::new(img_props);
            compiler.split_for_streaming = args.split_textures;
            compiler.platform = args.platform.clone();

            let out_files = compiler
                .process_file(input_file, &output_file, Some(ini_file))
                .map_err(|e| format!("ImageCompiler error: {}", e))?;
            for path in &out_files {
                CgfUtil::sync_file_time(input_file, path);
            }
            Ok(ProcessStatus::Compiled(out_files))
        }
        "dds" => {
            if args.decompress {
                let out_tif = TextureSplitter::decompress_dds_to_tif(input_file, &output_file)
                    .map_err(|e| format!("Decompress error: {}", e))?;
                return Ok(ProcessStatus::Compiled(vec![out_tif]));
            }
            if args.split_textures {
                let splitter = TextureSplitter::new(TextureSplitterConfig::default());
                let chunks = splitter
                    .process_dds_file(input_file, &output_file)
                    .map_err(|e| format!("TextureSplitter error: {}", e))?;
                return Ok(ProcessStatus::Compiled(chunks));
            }
            Ok(ProcessStatus::Compiled(vec![output_file]))
        }
        "mtl" => {
            if args.cryasset || args.strip_metadata {
                Ok(ProcessStatus::Compiled(vec![input_file.to_path_buf()]))
            } else {
                Ok(ProcessStatus::UnsupportedExtension)
            }
        }
        _ => Ok(ProcessStatus::UnsupportedExtension),
    }
}

fn resolve_output_path(input_file: &Path, args: &CliArgs, ext: &str) -> Result<PathBuf, String> {
    let target_ext = if let Some(ref over_ext) = args.overwrite_extension {
        over_ext.as_str()
    } else {
        match ext {
            "abc" => "cax",
            "crysub" => "tif",
            "xml" => "xml",
            "i_caf" => "caf",
            "i_cgf" => "cgf",
            "cdf" => "skin",
            "dae" => "cgf",
            "json" | "fbx" => "cgf",
            "tif" | "tiff" => "dds",
            other => other,
        }
    };

    if let Some(ref target) = args.output {
        if target.is_dir() || target.extension().is_none() {
            let rel_subpath = if let Some(ref src_root) = args.source_root {
                input_file.strip_prefix(src_root).unwrap_or(input_file)
            } else {
                input_file
            };

            let mut out = target.join(rel_subpath);
            if let Some(ref over_name) = args.overwrite_filename {
                out.set_file_name(over_name);
            }
            out.set_extension(target_ext);
            Ok(out)
        } else {
            Ok(target.clone())
        }
    } else {
        let mut out = input_file.to_path_buf();
        if let Some(ref over_name) = args.overwrite_filename {
            out.set_file_name(over_name);
        }
        out.set_extension(target_ext);
        Ok(out)
    }
}

fn is_file_up_to_date(source: &Path, target: &Path) -> bool {
    if !target.exists() {
        return false;
    }
    let src_meta = fs::metadata(source).ok();
    let tgt_meta = fs::metadata(target).ok();
    match (src_meta, tgt_meta) {
        (Some(src), Some(tgt)) => {
            let src_mtime = src.modified().ok();
            let tgt_mtime = tgt.modified().ok();
            match (src_mtime, tgt_mtime) {
                (Some(sm), Some(tm)) => tm >= sm,
                _ => false,
            }
        }
        _ => false,
    }
}

fn resolve_xml_filter(
    custom_path: &Option<PathBuf>,
    source_root: &Option<PathBuf>,
) -> Option<XmlFilter> {
    let candidate_paths = [
        custom_path.clone(),
        Some(PathBuf::from("xmlfilter.txt")),
        source_root.as_ref().map(|r| r.join("xmlfilter.txt")),
    ];

    for path_opt in candidate_paths.iter().flatten() {
        if path_opt.exists()
            && let Ok(filter) = XmlFilter::load_from_file(path_opt)
        {
            return Some(filter);
        }
    }
    None
}

fn parse_tea_key(key_str: &Option<String>) -> Option<[u32; 4]> {
    if let Some(s) = key_str {
        let clean = s.trim();
        if clean.len() == 32 {
            let mut key = [0u32; 4];
            for i in 0..4 {
                if let Ok(val) = u32::from_str_radix(&clean[i * 8..(i + 1) * 8], 16) {
                    key[i] = val;
                } else {
                    return None;
                }
            }
            return Some(key);
        }
    }
    None
}

fn collect_all_inputs(args: &CliArgs) -> Vec<PathBuf> {
    let mut results = Vec::new();
    if let Some(ref list_path) = args.list_file
        && let Ok(pairs) = ListFile::process_list_file(
            list_path,
            &[],
            &[],
            args.source
                .as_ref()
                .unwrap_or(&PathBuf::from("."))
                .as_path(),
        )
    {
        for (folder, file) in pairs {
            results.push(folder.join(file));
        }
        return results;
    }

    if let Some(ref src) = args.source {
        if src.is_file() {
            results.push(src.clone());
        } else if src.is_dir() {
            scan_dir(src, args.recursive, &mut results);
        }
    }
    results
}

fn scan_dir(dir: &Path, recursive: bool, list: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                list.push(path);
            } else if path.is_dir() && recursive {
                scan_dir(&path, recursive, list);
            }
        }
    }
}

fn clean_target_folder(target_root: &Path, tracker: &DependencyList, verbosity: u8) {
    if !target_root.exists() || !target_root.is_dir() {
        return;
    }
    let mut existing_files = Vec::new();
    scan_dir(target_root, true, &mut existing_files);

    for file in existing_files {
        if file.extension().and_then(|s| s.to_str()) == Some("cryasset") {
            continue;
        }
        let is_tracked = tracker.files.iter().any(|pair| pair.output_file == file);
        if !is_tracked {
            if verbosity >= 1 {
                println!(
                    "[RC CLEAN] Deleting obsolete target file: {}",
                    file.display()
                );
            }
            let _ = fs::remove_file(&file);
        }
    }
}
