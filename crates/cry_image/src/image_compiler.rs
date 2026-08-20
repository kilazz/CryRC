// Copyright 2004-2026 Crytek GmbH / Crytek Group. All rights reserved.
// High-Level Resource Compiler Image Processing Pipeline

use super::color_types::ColorRGBAf;
use super::compressors::bc1::compress_bc1_block;
use super::compressors::bc2::compress_alpha_bc2;
use super::compressors::bc3::compress_alpha_bc3;
use super::compressors::bc4::{compress_bc4, compress_bc4_signed};
use super::compressors::bc5::{compress_bc5, compress_bc5_normals, compress_bc5_signed};
use super::compressors::bc6h::compress_bc6h_block;
use super::compressors::bc7::compress_bc7_block;
use super::compressors::ctx1::compress_ctx1_block;
use super::compressors::pvrtc_compressor::{PvrPixelFormat, PvrQuality, PvrtcCompressor};
use super::converters::normal_processing::NormalProcessing;
use super::filtering::cubemap_gen::{CubeMapProcessor, CubemapFilterType};
use super::flags::{
    ColorMetric, CompressionOptions, FitStrategy, Format as TfFormat, MipmapFilter, QualityLevel,
};
use super::formats::crytif_io::CryTifIO;
use super::formats::dds_io::DdsIO;
use super::formats::hdr_rgbe_io::HdrRgbeIO;
use super::image_properties::{EInputColorSpace, EOutputColorSpace, ImageProperties};
use super::math::vector::{Vec3, Vec4};
use super::pipeline::dither::dither_image_rgba;
use super::pipeline::mipmaps::{AlphaCoverageOptions, generate_mipmaps_rgba};
use super::pipeline::normalmap::{NormalMapOptions, compute_normal_map};
use super::pipeline::rdo::apply_rdo_optimization;
use super::pixel_formats::{CPixelFormats, DxgiFormat, EPixelFormat};
use super::streaming::{TextureSplitter, TextureSplitterConfig};
use super::tables::srgb::srgb_to_linear;
use cry_core::name_converter::matches_wildcards_ignore_case;
use cry_core::{CfgFile, CfgSection};
use std::path::{Path, PathBuf};

/// In-memory representation of a loaded source image.
#[derive(Debug, Clone, Default)]
pub struct LoadedSourceImage {
    pub width: usize,
    pub height: usize,
    pub raw_rgba: Vec<u8>,
    pub is_hdr: bool,
    pub hdr_pixels: Vec<Vec4>,
}

/// High-level Resource Compiler image processing pipeline.
pub struct ImageCompiler {
    pub props: ImageProperties,
    pub platform: String,
    pub special_instructions: String,
    pub split_for_streaming: bool,
    pub decompress: bool,
    pub swizzle: String,
    pub legacy_gloss: bool,
    pub is_cubemap: bool,
    pub cubemap_filter: CubemapFilterType,
    pub cubemap_edge_fixup: usize,
    pub cubemap_diffuse_preset: String,
    pub rdo_lambda: Option<f32>,
    pub quality: QualityLevel,
    pub dither: bool,
}

impl ImageCompiler {
    pub fn new(props: ImageProperties) -> Self {
        Self {
            props,
            platform: "pc".to_string(),
            special_instructions: String::new(),
            split_for_streaming: false,
            decompress: false,
            swizzle: String::new(),
            legacy_gloss: false,
            is_cubemap: false,
            cubemap_filter: CubemapFilterType::GGX,
            cubemap_edge_fixup: 0,
            cubemap_diffuse_preset: String::new(),
            rdo_lambda: None,
            quality: QualityLevel::Normal,
            dither: false,
        }
    }

    /// Resolves deprecated legacy preset names into modern engine equivalents.
    pub fn resolve_preset_alias(ini: &CfgFile, preset_name: &str) -> String {
        if let Some(alias_sec_idx) = ini.find_section("_presetAliases") {
            let sec = &ini.sections[alias_sec_idx];
            for entry in &sec.entries {
                if entry.key.eq_ignore_ascii_case(preset_name) {
                    return entry.value.trim().to_string();
                }
            }
        }
        preset_name.to_string()
    }

    /// Matches the source texture filename against filemasks in rc.ini using longest-match priority.
    pub fn apply_ini_preset(&mut self, ini: &CfgFile, filename: &str) {
        let mut matched_section = None;
        let mut best_mask_len = 0;

        for sec in &ini.sections {
            if sec.name.is_empty() || sec.name.starts_with('_') {
                continue;
            }
            for entry in &sec.entries {
                if entry.key.eq_ignore_ascii_case("filemasks") {
                    for mask in entry.value.split([';', ',']) {
                        let trimmed_mask = mask.trim();
                        if !trimmed_mask.is_empty()
                            && matches_wildcards_ignore_case(filename, trimmed_mask)
                            && trimmed_mask.len() > best_mask_len
                        {
                            best_mask_len = trimmed_mask.len();
                            matched_section = Some(sec.clone());
                        }
                    }
                }
            }
        }

        if let Some(sec) = matched_section {
            self.props.preset = sec.name.clone();
            self.apply_section_properties(&sec);
            return;
        }

        let resolved = Self::resolve_preset_alias(ini, &self.props.preset);
        self.props.preset = resolved;

        if let Some(sec_idx) = ini.find_section(&self.props.preset) {
            let sec = ini.sections[sec_idx].clone();
            self.apply_section_properties(&sec);
        }
    }

    fn apply_section_properties(&mut self, sec: &CfgSection) {
        let platform_lower = self.platform.to_ascii_lowercase();

        for entry in &sec.entries {
            let key = entry.key.to_ascii_lowercase();
            let val = entry.value.trim();

            if let Some(pos) = key.find(':') {
                let base_key = &key[..pos];
                let target_plat = &key[pos + 1..];
                if target_plat.eq_ignore_ascii_case(&platform_lower) {
                    self.apply_single_property(base_key, val);
                }
            } else {
                self.apply_single_property(&key, val);
            }
        }
    }

    fn apply_single_property(&mut self, key: &str, val: &str) {
        match key {
            "imagecompressor" => {}
            "pixelformat" => {
                if let Some(fmt) = CPixelFormats::find_pixel_format_by_name(val) {
                    self.props.pixel_format = Some(fmt);
                }
            }
            "colorspace" => {
                let parts: Vec<&str> = val.split(',').collect();
                if parts.len() == 2 {
                    self.props.input_color_space = if parts[0].trim().eq_ignore_ascii_case("srgb") {
                        EInputColorSpace::Srgb
                    } else {
                        EInputColorSpace::Linear
                    };
                    self.props.output_color_space =
                        match parts[1].trim().to_ascii_lowercase().as_str() {
                            "srgb" => EOutputColorSpace::Srgb,
                            "auto" => EOutputColorSpace::Auto,
                            _ => EOutputColorSpace::Linear,
                        };
                }
            }
            "mipnormalize" => {
                self.props.mip_renormalize = val == "1" || val.eq_ignore_ascii_case("true")
            }
            "glossfromnormals" => {
                self.props.gloss_from_normals = val == "1" || val.eq_ignore_ascii_case("true")
            }
            "glosslegacydist" => self.legacy_gloss = val == "1" || val.eq_ignore_ascii_case("true"),
            "discardalpha" => {
                self.props.discard_alpha = val == "1" || val.eq_ignore_ascii_case("true")
            }
            "dynscale" => {
                self.props.normalize_range = val == "1" || val.eq_ignore_ascii_case("true")
            }
            "mipmaps" => {
                self.props.generate_mips = val != "0" && !val.eq_ignore_ascii_case("false")
            }
            "mintexturesize" => self.props.min_texture_size = val.parse().unwrap_or(0),
            "maxtexturesize" => self.props.max_texture_size = val.parse().unwrap_or(0),
            "reduce" => self.props.parse_reduce_string(val),
            "swizzle" => self.swizzle = val.to_string(),
            "cm" => self.is_cubemap = val == "1" || val.eq_ignore_ascii_case("true"),
            "cm_ftype" => {
                if val.eq_ignore_ascii_case("cosine") {
                    self.cubemap_filter = CubemapFilterType::Cosine;
                } else {
                    self.cubemap_filter = CubemapFilterType::GGX;
                }
            }
            "cm_edgefixup" => self.cubemap_edge_fixup = val.parse().unwrap_or(0),
            "cm_diffpreset" => self.cubemap_diffuse_preset = val.to_string(),
            "rdo_lambda" | "rdo" => {
                self.rdo_lambda = val.parse().ok();
            }
            "quality" => {
                self.quality = match val.to_ascii_lowercase().as_str() {
                    "ultrafast" => QualityLevel::Ultrafast,
                    "fast" => QualityLevel::Fast,
                    "slow" => QualityLevel::Slow,
                    "slowest" => QualityLevel::Slowest,
                    _ => QualityLevel::Normal,
                };
            }
            "alphacoverage" | "maintainalphacoverage" => {
                self.props.maintain_alpha_coverage = val == "1" || val.eq_ignore_ascii_case("true");
            }
            "dither" => {
                self.dither = val == "1" || val.eq_ignore_ascii_case("true");
            }
            _ => {}
        }
    }

    /// Compiles an image asset from disk into a fully processed and compressed CryEngine DDS file.
    pub fn process_file(
        &mut self,
        input_path: &Path,
        output_path: &Path,
        ini_file: Option<&CfgFile>,
    ) -> Result<Vec<PathBuf>, String> {
        let filename = input_path.file_name().unwrap_or_default().to_string_lossy();
        let ext = input_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        // 1. Read CryTIF Photoshop 8BIM IPTC instructions
        if (ext == "tif" || ext == "tiff")
            && let Ok(instructions) = CryTifIO::read_special_instructions(input_path)
            && !instructions.is_empty()
        {
            self.special_instructions = instructions.clone();
            self.apply_special_instructions(&instructions);
        }

        // 2. Match rules and presets from rc.ini
        if let Some(ini) = ini_file {
            self.apply_ini_preset(ini, &filename);
        }

        // 3. Load source image data
        let src_img = self.load_source_image(input_path)?;
        if src_img.width == 0 || src_img.height == 0 {
            return Err(format!(
                "Image {:?} has invalid dimensions: {}x{}",
                input_path, src_img.width, src_img.height
            ));
        }

        let width = src_img.width;
        let height = src_img.height;
        let raw_rgba = src_img.raw_rgba;
        let is_hdr = src_img.is_hdr;
        let hdr_pixels = src_img.hdr_pixels;

        let has_attached_alpha =
            filename.contains("_ddna") || self.props.preset == "NormalsWithSmoothness";
        let is_normal_map = filename.contains("_ddn") || self.props.preset.starts_with("Normals");
        let is_bump = filename.contains("_bump") || self.props.preset == "NormalsFromDisplacement";

        let dest_format = self.props.pixel_format.unwrap_or(if is_normal_map {
            EPixelFormat::BC5s
        } else if is_hdr {
            EPixelFormat::BC6UH
        } else {
            EPixelFormat::BC1
        });

        // 4. HDR Pipeline (BC6H / Float16 / Environment Probes)
        if is_hdr || dest_format == EPixelFormat::BC6UH {
            return self.process_hdr_pipeline(width, height, &raw_rgba, &hdr_pixels, output_path);
        }

        // 5. Channel Pre-Processing: Swizzling, Bump-to-Normal, and Discard Alpha
        let mut processed_rgba = raw_rgba;

        if !self.swizzle.is_empty() {
            for px in processed_rgba.chunks_exact_mut(4) {
                let orig = [px[0], px[1], px[2], px[3]];
                let map_ch = |c: char| -> u8 {
                    match c {
                        'r' => orig[0],
                        'g' => orig[1],
                        'b' => orig[2],
                        'a' => orig[3],
                        '0' => 0,
                        '1' => 255,
                        _ => 0,
                    }
                };
                let chars: Vec<char> = self.swizzle.to_ascii_lowercase().chars().collect();
                if chars.len() >= 4 {
                    px[0] = map_ch(chars[0]);
                    px[1] = map_ch(chars[1]);
                    px[2] = map_ch(chars[2]);
                    px[3] = map_ch(chars[3]);
                }
            }
        }

        if is_bump {
            let mut heightmap = vec![0u8; width * height];
            for (i, p) in processed_rgba.chunks_exact(4).enumerate() {
                heightmap[i] = p[0];
            }
            processed_rgba =
                compute_normal_map(&heightmap, width, height, NormalMapOptions::default());
        }

        if self.props.discard_alpha && !has_attached_alpha {
            for p in processed_rgba.chunks_exact_mut(4) {
                p[3] = 255;
            }
        }

        // 6. Resolve Color Space and Generate Mipchain with Mitchell-Netravali Kernel
        let use_srgb = match self.props.output_color_space {
            EOutputColorSpace::Srgb => true,
            EOutputColorSpace::Linear => false,
            EOutputColorSpace::Auto => {
                if is_normal_map || is_bump || self.props.preset.starts_with("Normals") {
                    false
                } else {
                    self.props.input_color_space == EInputColorSpace::Srgb
                }
            }
        };

        let alpha_cov =
            if self.props.maintain_alpha_coverage || self.props.preset.contains("Coverage") {
                Some(AlphaCoverageOptions { alpha_cutoff: 0.5 })
            } else {
                None
            };

        let mut mip_chain = if self.props.generate_mips {
            generate_mipmaps_rgba(
                &processed_rgba,
                width,
                height,
                MipmapFilter::MitchellNetravali,
                use_srgb,
                alpha_cov,
            )
        } else {
            vec![crate::pipeline::mipmaps::MipLevel {
                width,
                height,
                data: processed_rgba,
            }]
        };

        // 7. Apply Platform Resolution Reduction
        let reduce = self
            .props
            .get_resolution_reduce_for_platform(&self.platform);
        if reduce > 0 && mip_chain.len() > reduce {
            mip_chain.drain(0..reduce);
        }

        // 8. Normal Map Toksvig Gloss and Legacy Gloss Processing
        if self.props.gloss_from_normals || has_attached_alpha {
            for level in &mut mip_chain {
                let mut float_pixels: Vec<ColorRGBAf> = level
                    .data
                    .chunks_exact(4)
                    .map(|p| {
                        ColorRGBAf::new(
                            p[0] as f32 / 255.0,
                            p[1] as f32 / 255.0,
                            p[2] as f32 / 255.0,
                            p[3] as f32 / 255.0,
                        )
                    })
                    .collect();

                NormalProcessing::gloss_from_normals(&mut float_pixels, true);

                if self.legacy_gloss {
                    NormalProcessing::convert_legacy_gloss(&mut float_pixels);
                }

                for (i, p) in float_pixels.iter().enumerate() {
                    level.data[i * 4 + 3] = (p.a * 255.0).round().clamp(0.0, 255.0) as u8;
                }
            }
        }

        // 9. Dithering (if enabled)
        if self.dither {
            for level in &mut mip_chain {
                let mut dithered = vec![0u8; level.data.len()];
                dither_image_rgba(
                    &level.data,
                    &level.data,
                    level.width,
                    level.height,
                    &mut dithered,
                );
                level.data = dithered;
            }
        }

        // 10. Mobile Codec Dispatch for Mobile Targets
        if self.platform.eq_ignore_ascii_case("es3")
            || self.platform.eq_ignore_ascii_case("android")
        {
            return self.process_mobile_compression(
                &mip_chain,
                dest_format,
                is_normal_map,
                use_srgb,
                output_path,
            );
        }

        // 11. Uncompressed Formats vs Block Compression Pipeline
        let is_uncompressed = matches!(
            dest_format,
            EPixelFormat::A8R8G8B8 | EPixelFormat::X8R8G8B8 | EPixelFormat::R8G8B8
        );

        let mut main_compressed_payload = Vec::new();
        let mut alpha_compressed_payload = Vec::new();

        if is_uncompressed {
            // Direct BGRA / BGRX uncompressed stream (used for Gradients, UI, ColorCharts)
            for level in &mip_chain {
                let mut bgra_data = Vec::with_capacity(level.width * level.height * 4);
                for p in level.data.chunks_exact(4) {
                    bgra_data.push(p[2]); // B
                    bgra_data.push(p[1]); // G
                    bgra_data.push(p[0]); // R
                    bgra_data.push(if dest_format == EPixelFormat::X8R8G8B8 {
                        255
                    } else {
                        p[3]
                    }); // A
                }
                main_compressed_payload.extend_from_slice(&bgra_data);
            }
        } else {
            // Desktop Block Compression (BC1..BC7 / CTX1)
            let (tf_format, is_signed) = map_engine_format_to_tf(dest_format);
            let is_1bit_alpha =
                dest_format == EPixelFormat::BC1a || self.props.maintain_alpha_coverage;
            let bytes_per_block = tf_format.bytes_per_block();

            let rdo_lambda = self.rdo_lambda.unwrap_or(0.0);

            let compress_opts = CompressionOptions {
                format: tf_format,
                metric: if is_normal_map {
                    ColorMetric::Unit
                } else if use_srgb {
                    ColorMetric::Perceptual
                } else {
                    ColorMetric::Uniform
                },
                strategy: FitStrategy::Cluster(8),
                quality: self.quality,
                weight_by_alpha: !is_normal_map
                    && !self.props.discard_alpha
                    && matches!(
                        dest_format,
                        EPixelFormat::BC2
                            | EPixelFormat::BC3
                            | EPixelFormat::BC7
                            | EPixelFormat::BC7t
                    ),
                is_1bit_alpha,
                alpha_iterative_fit: true,
                is_signed,
                is_normal_map,
                srgb: use_srgb,
                dither_rgb: false,
                dither_a: false,
                rdo_lambda,
                rdo_ultrasmooth: true,
                rdo_lookback_window: 256,
                rdo_try_two_matches: true,
                rdo_smooth_block_scale: None,
            };

            // 1. Compress base BC5s / color blocks
            for level in &mip_chain {
                let bw = level.width.div_ceil(4);
                let bh = level.height.div_ceil(4);
                let mut level_blocks = vec![0u8; bw * bh * bytes_per_block];

                for by in 0..bh {
                    for bx in 0..bw {
                        let mut px_16 = [[0u8; 4]; 16];
                        let mut mask = 0u16;

                        for py in 0..4 {
                            for px in 0..4 {
                                let x = bx * 4 + px;
                                let y = by * 4 + py;
                                let idx = py * 4 + px;

                                if x < level.width && y < level.height {
                                    let off = (y * level.width + x) * 4;
                                    px_16[idx] = [
                                        level.data[off],
                                        level.data[off + 1],
                                        level.data[off + 2],
                                        level.data[off + 3],
                                    ];
                                    mask |= 1 << idx;
                                }
                            }
                        }

                        let off = (by * bw + bx) * bytes_per_block;
                        compress_single_block_dispatch(
                            &px_16,
                            mask,
                            compress_opts,
                            &mut level_blocks[off..off + bytes_per_block],
                        );
                    }
                }

                if compress_opts.rdo_lambda > 0.0 {
                    apply_rdo_optimization(
                        &mut level_blocks,
                        &level.data,
                        level.width,
                        level.height,
                        &compress_opts,
                    );
                }

                main_compressed_payload.extend_from_slice(&level_blocks);
            }

            // 2. Compress attached BC4 Alpha stream for _ddna
            if has_attached_alpha {
                for level in &mip_chain {
                    let bw = level.width.div_ceil(4);
                    let bh = level.height.div_ceil(4);
                    let mut alpha_blocks = vec![0u8; bw * bh * 8];

                    for by in 0..bh {
                        for bx in 0..bw {
                            let mut alphas = [0u8; 16];
                            let mut mask = 0u16;

                            for py in 0..4 {
                                for px in 0..4 {
                                    let x = bx * 4 + px;
                                    let y = by * 4 + py;
                                    let idx = py * 4 + px;

                                    if x < level.width && y < level.height {
                                        let off = (y * level.width + x) * 4;
                                        alphas[idx] = level.data[off + 3];
                                        mask |= 1 << idx;
                                    }
                                }
                            }

                            let off = (by * bw + bx) * 8;
                            compress_bc4(
                                &alphas,
                                mask,
                                1 << 15,
                                (&mut alpha_blocks[off..off + 8]).try_into().unwrap(),
                            );
                        }
                    }

                    alpha_compressed_payload.extend_from_slice(&alpha_blocks);
                }
            }
        }

        let mut dxgi_format = CPixelFormats::get_pixel_format_info(dest_format).dxgi_format;
        if use_srgb {
            dxgi_format = match dxgi_format {
                DxgiFormat::BC1Unorm => DxgiFormat::BC1UnormSrgb,
                DxgiFormat::BC2Unorm => DxgiFormat::BC2UnormSrgb,
                DxgiFormat::BC3Unorm => DxgiFormat::BC3UnormSrgb,
                DxgiFormat::BC7Unorm => DxgiFormat::BC7UnormSrgb,
                DxgiFormat::B8G8R8A8Unorm => DxgiFormat::B8G8R8A8UnormSrgb,
                other => other,
            };
        }

        let is_file_single = filename.contains("_ddn") && !has_attached_alpha;
        let out_dds = output_path.with_extension("dds");

        DdsIO::save_dds_file(
            &out_dds,
            mip_chain[0].width as u32,
            mip_chain[0].height as u32,
            mip_chain.len() as u32,
            dxgi_format,
            use_srgb,
            self.is_cubemap,
            has_attached_alpha,
            self.props.mip_renormalize,
            is_file_single,
            &main_compressed_payload,
            if has_attached_alpha {
                Some(&alpha_compressed_payload)
            } else {
                None
            },
        )
        .map_err(|e| format!("Failed to write DDS: {}", e))?;

        let mut produced_files = vec![out_dds.clone()];

        if self.split_for_streaming && !self.is_cubemap {
            let splitter = TextureSplitter::new(TextureSplitterConfig::default());
            if let Ok(chunks) = splitter.process_dds_file(&out_dds, &out_dds) {
                for chunk in chunks {
                    if !produced_files.contains(&chunk) {
                        produced_files.push(chunk);
                    }
                }
            }
        }

        Ok(produced_files)
    }

    fn process_hdr_pipeline(
        &mut self,
        width: usize,
        height: usize,
        raw_rgba: &[u8],
        hdr_pixels: &[Vec4],
        output_path: &Path,
    ) -> Result<Vec<PathBuf>, String> {
        let mut rgb_f32 = Vec::with_capacity(width * height);
        if !hdr_pixels.is_empty() {
            for p in hdr_pixels {
                rgb_f32.push([p.x, p.y, p.z]);
            }
        } else {
            for p in raw_rgba.chunks_exact(4) {
                rgb_f32.push([
                    srgb_to_linear(p[0]),
                    srgb_to_linear(p[1]),
                    srgb_to_linear(p[2]),
                ]);
            }
        }

        if self.is_cubemap {
            let face_size = (width / 6).clamp(32, 256);
            let mut processor = CubeMapProcessor::new(face_size, 3);
            let sample_count = if self.cubemap_filter == CubemapFilterType::GGX {
                256
            } else {
                128
            };

            processor.filter_cubemap_mipchain(
                face_size,
                6,
                self.cubemap_filter,
                sample_count,
                self.cubemap_edge_fixup,
            );
        }

        let block_count_x = width.div_ceil(4);
        let block_count_y = height.div_ceil(4);
        let mut compressed_payload = vec![0u8; block_count_x * block_count_y * 16];
        let metric = Vec3::new(1.0, 1.0, 1.0);

        for by in 0..block_count_y {
            for bx in 0..block_count_x {
                let mut block_pixels = [[0.0f32; 3]; 16];
                let mut mask = 0u16;

                for py in 0..4 {
                    for px in 0..4 {
                        let x = bx * 4 + px;
                        let y = by * 4 + py;
                        let idx = py * 4 + px;

                        if x < width && y < height {
                            block_pixels[idx] = rgb_f32[y * width + x];
                            mask |= 1 << idx;
                        }
                    }
                }

                let off = (by * block_count_x + bx) * 16;
                let out_block: &mut [u8; 16] =
                    (&mut compressed_payload[off..off + 16]).try_into().unwrap();
                compress_bc6h_block(
                    &block_pixels,
                    mask,
                    &metric,
                    false,
                    FitStrategy::Cluster(4),
                    out_block,
                );
            }
        }

        let out_dds = output_path.with_extension("dds");
        DdsIO::save_dds_file(
            &out_dds,
            width as u32,
            height as u32,
            1,
            DxgiFormat::BC6HUf16,
            false,
            self.is_cubemap,
            false,
            false,
            false,
            &compressed_payload,
            None,
        )
        .map_err(|e| format!("Failed to write HDR DDS: {}", e))?;

        Ok(vec![out_dds])
    }

    fn process_mobile_compression(
        &self,
        mip_chain: &[crate::pipeline::mipmaps::MipLevel],
        dest_format: EPixelFormat,
        _is_normal_map: bool,
        use_srgb: bool,
        output_path: &Path,
    ) -> Result<Vec<PathBuf>, String> {
        let mut compressed_payload = Vec::new();

        for level in mip_chain {
            let pvr_fmt = match dest_format {
                EPixelFormat::BC1 => PvrPixelFormat::ETC2,
                EPixelFormat::BC2 | EPixelFormat::BC3 | EPixelFormat::BC7 => PvrPixelFormat::ETC2a,
                EPixelFormat::BC4 | EPixelFormat::BC4s => PvrPixelFormat::EacR11,
                EPixelFormat::BC5 | EPixelFormat::BC5s => PvrPixelFormat::EacRg11,
                _ => PvrPixelFormat::ETC2,
            };

            let mut bgra_u8 = Vec::with_capacity(level.width * level.height * 4);
            for p in level.data.chunks_exact(4) {
                bgra_u8.push(p[2]); // B
                bgra_u8.push(p[1]); // G
                bgra_u8.push(p[0]); // R
                bgra_u8.push(p[3]); // A
            }

            let blocks = PvrtcCompressor::compress(
                &bgra_u8,
                level.width,
                level.height,
                pvr_fmt,
                PvrQuality::Normal,
                use_srgb,
            );
            compressed_payload.extend_from_slice(&blocks);
        }

        let out_dds = output_path.with_extension("dds");
        DdsIO::save_dds_file(
            &out_dds,
            mip_chain[0].width as u32,
            mip_chain[0].height as u32,
            mip_chain.len() as u32,
            DxgiFormat::R8G8B8A8Unorm,
            use_srgb,
            false,
            false,
            false,
            false,
            &compressed_payload,
            None,
        )
        .map_err(|e| format!("Failed to write mobile DDS: {}", e))?;

        Ok(vec![out_dds])
    }

    fn apply_special_instructions(&mut self, instructions: &str) {
        for token in instructions.split_whitespace() {
            if let Some(stripped) = token.strip_prefix('/') {
                let parts: Vec<&str> = stripped.splitn(2, '=').collect();
                let key = parts[0].to_ascii_lowercase();
                let val = if parts.len() == 2 {
                    parts[1].trim_matches('"')
                } else {
                    ""
                };

                match key.as_str() {
                    "preset" => self.props.preset = val.to_string(),
                    "reduce" => self.props.parse_reduce_string(val),
                    "colorspace" => {
                        let cs_parts: Vec<&str> = val.split(',').collect();
                        if cs_parts.len() == 2 {
                            self.props.input_color_space =
                                if cs_parts[0].trim().eq_ignore_ascii_case("srgb") {
                                    EInputColorSpace::Srgb
                                } else {
                                    EInputColorSpace::Linear
                                };
                            self.props.output_color_space =
                                match cs_parts[1].trim().to_ascii_lowercase().as_str() {
                                    "srgb" => EOutputColorSpace::Srgb,
                                    "auto" => EOutputColorSpace::Auto,
                                    _ => EOutputColorSpace::Linear,
                                };
                        }
                    }
                    "dynscale" => self.props.normalize_range = val == "1" || val == "true",
                    "discardalpha" => self.props.discard_alpha = val == "1" || val == "true",
                    "mipnormalize" => self.props.mip_renormalize = val == "1" || val == "true",
                    "glossfromnormals" => {
                        self.props.gloss_from_normals = val == "1" || val == "true"
                    }
                    "glosslegacydist" => self.legacy_gloss = val == "1" || val == "true",
                    "swizzle" => self.swizzle = val.to_string(),
                    "rdo_lambda" | "rdo" => self.rdo_lambda = val.parse().ok(),
                    "dither" => self.dither = val == "1" || val == "true",
                    "quality" => {
                        self.quality = match val.to_ascii_lowercase().as_str() {
                            "ultrafast" => QualityLevel::Ultrafast,
                            "fast" => QualityLevel::Fast,
                            "slow" => QualityLevel::Slow,
                            "slowest" => QualityLevel::Slowest,
                            _ => QualityLevel::Normal,
                        };
                    }
                    _ => {}
                }
            }
        }
    }

    fn load_source_image(&self, path: &Path) -> Result<LoadedSourceImage, String> {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext == "hdr" {
            let (w, h, pixels, _) = HdrRgbeIO::load_hdr(path)?;
            let v4_pixels = pixels
                .iter()
                .map(|p| Vec4::new(p.x, p.y, p.z, p.w))
                .collect();
            Ok(LoadedSourceImage {
                width: w,
                height: h,
                raw_rgba: Vec::new(),
                is_hdr: true,
                hdr_pixels: v4_pixels,
            })
        } else {
            let dyn_img =
                image::open(path).map_err(|e| format!("Failed to open image {:?}: {}", path, e))?;
            let rgba = dyn_img.to_rgba8();
            let (w, h) = rgba.dimensions();
            Ok(LoadedSourceImage {
                width: w as usize,
                height: h as usize,
                raw_rgba: rgba.into_raw(),
                is_hdr: false,
                hdr_pixels: Vec::new(),
            })
        }
    }
}

fn map_engine_format_to_tf(fmt: EPixelFormat) -> (TfFormat, bool) {
    match fmt {
        EPixelFormat::BC1 => (TfFormat::Bc1, false),
        EPixelFormat::BC1a => (TfFormat::Bc1, false),
        EPixelFormat::BC2 | EPixelFormat::BC2t => (TfFormat::Bc2, false),
        EPixelFormat::BC3 | EPixelFormat::BC3t => (TfFormat::Bc3, false),
        EPixelFormat::BC4 => (TfFormat::Bc4, false),
        EPixelFormat::BC4s => (TfFormat::Bc4, true),
        EPixelFormat::BC5 => (TfFormat::Bc5, false),
        EPixelFormat::BC5s => (TfFormat::Bc5, true),
        EPixelFormat::BC6UH => (TfFormat::Bc6h, false),
        EPixelFormat::BC7 | EPixelFormat::BC7t => (TfFormat::Bc7, false),
        EPixelFormat::CTX1 => (TfFormat::Ctx1, false),
        _ => (TfFormat::Bc7, false),
    }
}

fn compress_single_block_dispatch(
    pixels: &[[u8; 4]; 16],
    mask: u16,
    opts: CompressionOptions,
    out_block: &mut [u8],
) {
    match opts.format {
        TfFormat::Bc1 => {
            let blk: &mut [u8; 8] = out_block.try_into().unwrap();
            compress_bc1_block(pixels, mask, opts, blk);
        }
        TfFormat::Bc2 => {
            let (alpha_blk, color_blk) = out_block.split_at_mut(8);
            let mut alphas = [0u8; 16];
            for i in 0..16 {
                alphas[i] = pixels[i][3];
            }
            compress_alpha_bc2(&alphas, mask, alpha_blk.try_into().unwrap());
            let mut c_opts = opts;
            c_opts.format = TfFormat::Bc1;
            c_opts.weight_by_alpha = false;
            compress_bc1_block(pixels, mask, c_opts, color_blk.try_into().unwrap());
        }
        TfFormat::Bc3 => {
            let (alpha_blk, color_blk) = out_block.split_at_mut(8);
            let mut alphas = [0u8; 16];
            for i in 0..16 {
                alphas[i] = pixels[i][3];
            }
            compress_alpha_bc3(&alphas, mask, 1 << 15, alpha_blk.try_into().unwrap());
            let mut c_opts = opts;
            c_opts.format = TfFormat::Bc1;
            c_opts.weight_by_alpha = false;
            compress_bc1_block(pixels, mask, c_opts, color_blk.try_into().unwrap());
        }
        TfFormat::Bc4 => {
            let blk: &mut [u8; 8] = out_block.try_into().unwrap();
            if opts.is_signed {
                let mut reds = [0i8; 16];
                for i in 0..16 {
                    reds[i] = (pixels[i][0] as i32 - 128).clamp(-127, 127) as i8;
                }
                compress_bc4_signed(&reds, mask, 0, blk);
            } else {
                let mut reds = [0u8; 16];
                for i in 0..16 {
                    reds[i] = pixels[i][0];
                }
                compress_bc4(&reds, mask, 1 << 15, blk);
            }
        }
        TfFormat::Bc5 => {
            let (blk_r, blk_g) = out_block.split_at_mut(8);
            let mut reds = [0u8; 16];
            let mut greens = [0u8; 16];
            for i in 0..16 {
                reds[i] = pixels[i][0];
                greens[i] = pixels[i][1];
            }

            if opts.is_normal_map {
                // Native CryEngine 3Dc / ATI2 normal map compression using spherical deviance
                compress_bc5_normals(
                    &reds,
                    &greens,
                    mask,
                    1 << 15,
                    blk_r.try_into().unwrap(),
                    blk_g.try_into().unwrap(),
                );
            } else if opts.is_signed {
                let mut signed_r = [0i8; 16];
                let mut signed_g = [0i8; 16];
                for i in 0..16 {
                    signed_r[i] = (pixels[i][0] as i32 - 128).clamp(-127, 127) as i8;
                    signed_g[i] = (pixels[i][1] as i32 - 128).clamp(-127, 127) as i8;
                }
                compress_bc5_signed(&signed_r, &signed_g, mask, 0, out_block.try_into().unwrap());
            } else {
                compress_bc5(&reds, &greens, mask, 1 << 15, out_block.try_into().unwrap());
            }
        }
        TfFormat::Bc6h => {
            let mut rgb_f32 = [[0.0f32; 3]; 16];
            for i in 0..16 {
                rgb_f32[i] = [
                    pixels[i][0] as f32 / 255.0,
                    pixels[i][1] as f32 / 255.0,
                    pixels[i][2] as f32 / 255.0,
                ];
            }
            compress_bc6h_block(
                &rgb_f32,
                mask,
                &Vec3::splat(1.0),
                opts.is_signed,
                opts.strategy,
                out_block.try_into().unwrap(),
            );
        }
        TfFormat::Bc7 => {
            let metric = Vec4::new(0.2126, 0.7152, 0.0722, 1.0);
            compress_bc7_block(
                pixels,
                mask,
                0,
                &metric,
                opts.quality,
                out_block.try_into().unwrap(),
            );
        }
        TfFormat::Ctx1 => {
            compress_ctx1_block(pixels, mask, 0, out_block.try_into().unwrap());
        }
    }
}
