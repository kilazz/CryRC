// Copyright 2001-2026 Crytek GmbH / Crytek Group. All rights reserved.
// CryTIF Interactive Slint UI Bridge & Real-Time Texture Compression Preview

use cry_core::CfgFile;
use cry_image::{
    BumpProperties, CPixelFormats, ColorMetric, CompressionOptions, EInputColorSpace,
    EOutputColorSpace, EPixelFormat, FitStrategy, Format as TfFormat, ImageCompiler,
    ImageProperties, MipmapFilter, MipmapLevel, NormalFilterType, NormalProcessing, QualityLevel,
    compute_max_mip_count, decompress_image, get_storage_requirements, map_engine_format_to_tf,
};
use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer, VecModel};
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

slint::include_modules!();

pub struct CryTifGui;

impl CryTifGui {
    /// Launches the interactive CryTIF settings and real-time preview dialog.
    pub fn run_dialog(
        source_tif: &Path,
        target_dds: &Path,
        ini_file: &CfgFile,
    ) -> Result<bool, String> {
        let dyn_img = image::open(source_tif)
            .map_err(|e| format!("Failed to open image for CryTIF preview: {}", e))?;
        let src_rgba = dyn_img.to_rgba8();
        let (width, height) = src_rgba.dimensions();

        let ui = CryTifDialog::new().map_err(|e| e.to_string())?;

        // 1. Presets Management
        let filename = source_tif.file_name().unwrap_or_default().to_string_lossy();
        let all_presets = load_all_engine_presets(ini_file);
        let matching_presets = filter_matching_presets(&filename, &all_presets);

        let default_preset = detect_default_preset(source_tif, &all_presets);
        let default_idx = all_presets
            .iter()
            .position(|p| p == &default_preset)
            .unwrap_or(0) as i32;

        let slint_all_presets: Vec<slint::SharedString> =
            all_presets.iter().map(|s| s.as_str().into()).collect();
        let slint_matching_presets: Vec<slint::SharedString> =
            matching_presets.iter().map(|s| s.as_str().into()).collect();

        ui.set_presets(Rc::new(VecModel::from(slint_all_presets.clone())).into());
        ui.set_selected_preset_index(default_idx);
        ui.set_current_preset(default_preset.as_str().into());
        ui.set_list_all_presets(true);

        // Platform target indicators
        let platform_reduces = Rc::new(std::cell::RefCell::new([0i32, 0i32, 0i32, 1i32]));

        // Sync initial preset values
        sync_ui_with_preset(&ui, &default_preset, ini_file);

        let (_, _, _, _, fmt_name) =
            resolve_preset_format(&default_preset, ui.get_format_override_index(), ini_file);
        update_platform_res_labels(&ui, width, height, &platform_reduces.borrow(), &fmt_name);

        let info_text = get_preset_config_text(&default_preset, ini_file);
        ui.set_preset_info_text(info_text.into());

        // 2. Initial Previews Computation (Fully Synchronized)
        update_previews_both(
            &ui,
            &src_rgba,
            width as usize,
            height as usize,
            &default_preset,
            ini_file,
        );

        let ui_handle = ui.as_weak();
        let src_rgba_clone = Arc::new(src_rgba);
        let ini_file_clone = Arc::new(ini_file.clone());

        // Platform Resolution Tuning Callbacks
        {
            let ui_weak = ui_handle.clone();
            let p_red = Rc::clone(&platform_reduces);
            let ini_data = Arc::clone(&ini_file_clone);
            ui.on_higher_res_clicked(move |idx| {
                if let Some(ui) = ui_weak.upgrade() {
                    let mut arr = p_red.borrow_mut();
                    let plat_idx = (idx as usize).min(3);
                    arr[plat_idx] = (arr[plat_idx] - 1).max(-2);
                    let preset_name = ui.get_current_preset().to_string();
                    let (_, _, _, _, fmt_name) = resolve_preset_format(
                        &preset_name,
                        ui.get_format_override_index(),
                        &ini_data,
                    );
                    update_platform_res_labels(&ui, width, height, &arr, &fmt_name);
                }
            });
        }

        {
            let ui_weak = ui_handle.clone();
            let p_red = Rc::clone(&platform_reduces);
            let ini_data = Arc::clone(&ini_file_clone);
            ui.on_lower_res_clicked(move |idx| {
                if let Some(ui) = ui_weak.upgrade() {
                    let mut arr = p_red.borrow_mut();
                    let plat_idx = (idx as usize).min(3);
                    arr[plat_idx] = (arr[plat_idx] + 1).min(5);
                    let preset_name = ui.get_current_preset().to_string();
                    let (_, _, _, _, fmt_name) = resolve_preset_format(
                        &preset_name,
                        ui.get_format_override_index(),
                        &ini_data,
                    );
                    update_platform_res_labels(&ui, width, height, &arr, &fmt_name);
                }
            });
        }

        // Callback: List all presets toggle
        {
            let ui_weak = ui_handle.clone();
            let all_list = slint_all_presets;
            let match_list = slint_matching_presets;

            ui.on_list_all_toggled(move |show_all| {
                if let Some(ui) = ui_weak.upgrade() {
                    let cur = ui.get_current_preset().to_string();
                    if show_all {
                        ui.set_presets(Rc::new(VecModel::from(all_list.clone())).into());
                        if let Some(pos) = all_list.iter().position(|p| p.as_str() == cur) {
                            ui.set_selected_preset_index(pos as i32);
                        }
                    } else {
                        ui.set_presets(Rc::new(VecModel::from(match_list.clone())).into());
                        if let Some(pos) = match_list.iter().position(|p| p.as_str() == cur) {
                            ui.set_selected_preset_index(pos as i32);
                        }
                    }
                }
            });
        }

        // Callback: Preset Changed
        {
            let ui_weak = ui_handle.clone();
            let src_data = Arc::clone(&src_rgba_clone);
            let ini_data = Arc::clone(&ini_file_clone);
            let p_red = Rc::clone(&platform_reduces);

            ui.on_preset_selected(move |preset_name| {
                if let Some(ui) = ui_weak.upgrade() {
                    if preset_name.starts_with("---") {
                        return;
                    }
                    sync_ui_with_preset(&ui, preset_name.as_str(), &ini_data);
                    let info_text = get_preset_config_text(preset_name.as_str(), &ini_data);
                    ui.set_preset_info_text(info_text.into());

                    let (_, _, _, _, fmt_name) = resolve_preset_format(
                        preset_name.as_str(),
                        ui.get_format_override_index(),
                        &ini_data,
                    );
                    update_platform_res_labels(&ui, width, height, &p_red.borrow(), &fmt_name);

                    // Reset Mip to 0 on preset change
                    ui.set_mip_level(0);
                    update_previews_both(
                        &ui,
                        &src_data,
                        width as usize,
                        height as usize,
                        preset_name.as_str(),
                        &ini_data,
                    );
                }
            });
        }

        // Callback: Settings / Sliders / Channels / Checkboxes / Mips Changed
        {
            let ui_weak = ui_handle.clone();
            let src_data = Arc::clone(&src_rgba_clone);
            let ini_data = Arc::clone(&ini_file_clone);
            let p_red = Rc::clone(&platform_reduces);

            ui.on_settings_changed(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    let preset_name = ui.get_current_preset();
                    let (_, _, _, _, fmt_name) = resolve_preset_format(
                        preset_name.as_str(),
                        ui.get_format_override_index(),
                        &ini_data,
                    );
                    update_platform_res_labels(&ui, width, height, &p_red.borrow(), &fmt_name);

                    update_previews_both(
                        &ui,
                        &src_data,
                        width as usize,
                        height as usize,
                        preset_name.as_str(),
                        &ini_data,
                    );
                }
            });
        }

        // Zoom Callbacks
        {
            let ui_weak = ui_handle.clone();
            let src_data = Arc::clone(&src_rgba_clone);
            let ini_data = Arc::clone(&ini_file_clone);

            ui.on_zoom_in(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    let cur = ui.get_zoom_percent();
                    let new_z = (cur * 2).min(1600);
                    ui.set_zoom_percent(new_z);
                    let preset_name = ui.get_current_preset();
                    update_previews_both(
                        &ui,
                        &src_data,
                        width as usize,
                        height as usize,
                        preset_name.as_str(),
                        &ini_data,
                    );
                }
            });
        }

        {
            let ui_weak = ui_handle.clone();
            let src_data = Arc::clone(&src_rgba_clone);
            let ini_data = Arc::clone(&ini_file_clone);

            ui.on_zoom_out(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    let cur = ui.get_zoom_percent();
                    let new_z = (cur / 2).max(25);
                    ui.set_zoom_percent(new_z);
                    let preset_name = ui.get_current_preset();
                    update_previews_both(
                        &ui,
                        &src_data,
                        width as usize,
                        height as usize,
                        preset_name.as_str(),
                        &ini_data,
                    );
                }
            });
        }

        {
            let ui_weak = ui_handle.clone();
            let src_data = Arc::clone(&src_rgba_clone);
            let ini_data = Arc::clone(&ini_file_clone);

            ui.on_zoom_reset(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_zoom_percent(100);
                    let preset_name = ui.get_current_preset();
                    update_previews_both(
                        &ui,
                        &src_data,
                        width as usize,
                        height as usize,
                        preset_name.as_str(),
                        &ini_data,
                    );
                }
            });
        }

        // Action Callback: Generate Output
        let src_path_buf = source_tif.to_path_buf();
        let dst_path_buf = target_dds.to_path_buf();
        let ini_data = Arc::clone(&ini_file_clone);

        {
            let ui_weak = ui_handle.clone();
            let src_p = src_path_buf.clone();
            let dst_p = dst_path_buf.clone();
            let ini_d = Arc::clone(&ini_data);

            ui.on_generate_clicked(move || {
                if let Some(ui) = ui_weak.upgrade()
                    && execute_compilation_from_ui(&ui, &src_p, &dst_p, &ini_d).is_ok()
                {
                    let cur_info = ui.get_target_info().to_string();
                    ui.set_target_info(format!("[Generated OK] {}", cur_info).into());
                }
            });
        }

        // Action Callback: OK
        let accepted = Rc::new(std::cell::Cell::new(false));
        {
            let ui_weak = ui_handle.clone();
            let acc = Rc::clone(&accepted);
            let src_p = src_path_buf.clone();
            let dst_p = dst_path_buf.clone();
            let ini_d = Arc::clone(&ini_data);

            ui.on_ok_clicked(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    acc.set(true);
                    let _ = execute_compilation_from_ui(&ui, &src_p, &dst_p, &ini_d);
                    let _ = ui.hide();
                }
            });
        }

        // Action Callback: Cancel
        {
            let ui_weak = ui_handle.clone();
            ui.on_cancel_clicked(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    let _ = ui.hide();
                }
            });
        }

        ui.run().map_err(|e| e.to_string())?;
        Ok(accepted.get())
    }
}

/// Resolves the exact texture format and settings matching `rc.ini` and GUI overrides.
pub fn resolve_preset_format(
    preset_name: &str,
    format_override_idx: i32,
    ini_file: &CfgFile,
) -> (TfFormat, bool, EPixelFormat, bool, String) {
    let resolved_preset = ImageCompiler::resolve_preset_alias(ini_file, preset_name);

    // 1. Check Format Override from Inspector Tab
    let override_fmt = match format_override_idx {
        1 => Some((
            TfFormat::Bc1,
            false,
            EPixelFormat::BC1,
            false,
            "BC1".to_string(),
        )),
        2 => Some((
            TfFormat::Bc1,
            false,
            EPixelFormat::BC1a,
            false,
            "BC1a".to_string(),
        )),
        3 => Some((
            TfFormat::Bc3,
            false,
            EPixelFormat::BC3,
            false,
            "BC3 (DXT5)".to_string(),
        )),
        4 => Some((
            TfFormat::Bc4,
            false,
            EPixelFormat::BC4,
            false,
            "BC4".to_string(),
        )),
        5 => Some((
            TfFormat::Bc5,
            true,
            EPixelFormat::BC5s,
            true,
            "BC5s (Normals)".to_string(),
        )),
        6 => Some((
            TfFormat::Bc6h,
            false,
            EPixelFormat::BC6UH,
            false,
            "BC6H (HDR)".to_string(),
        )),
        7 => Some((
            TfFormat::Bc7,
            false,
            EPixelFormat::BC7,
            false,
            "BC7".to_string(),
        )),
        8 => Some((
            TfFormat::Bc7,
            false,
            EPixelFormat::A8R8G8B8,
            false,
            "RGBA8 (Lossless)".to_string(),
        )),
        _ => None,
    };

    if let Some(res) = override_fmt {
        return res;
    }

    // 2. Lookup Preset Section in rc.ini
    if let Some(sec_idx) = ini_file.find_section(&resolved_preset) {
        let sec = &ini_file.sections[sec_idx];
        let mut pixel_format_name = String::new();
        let mut is_normal =
            resolved_preset.starts_with("Normals") || resolved_preset.contains("Normal");

        for entry in &sec.entries {
            if entry.key.eq_ignore_ascii_case("pixelformat") {
                pixel_format_name = entry.value.trim().to_string();
            }
            if entry.key.eq_ignore_ascii_case("bumptype")
                || entry.key.eq_ignore_ascii_case("mipnormalize")
            {
                is_normal = true;
            }
        }

        if let Some(engine_fmt) = CPixelFormats::find_pixel_format_by_name(&pixel_format_name) {
            let (tf_fmt, is_signed) = map_engine_format_to_tf(engine_fmt);
            let disp = pixel_format_name.clone();
            return (tf_fmt, is_signed, engine_fmt, is_normal, disp);
        }
    }

    // 3. Fallback Heuristics
    let p_lower = resolved_preset.to_ascii_lowercase();
    if p_lower.contains("opacity") || p_lower.contains("decal") || p_lower.contains("detail") {
        (
            TfFormat::Bc7,
            false,
            EPixelFormat::BC7,
            false,
            "BC7".to_string(),
        )
    } else if p_lower.starts_with("normal") || p_lower.contains("bump") {
        (
            TfFormat::Bc5,
            true,
            EPixelFormat::BC5s,
            true,
            "BC5s".to_string(),
        )
    } else if p_lower.contains("hdr") || p_lower.contains("probe") {
        (
            TfFormat::Bc6h,
            false,
            EPixelFormat::BC6UH,
            false,
            "BC6H".to_string(),
        )
    } else if p_lower.contains("displacement")
        || p_lower.contains("greyscale")
        || p_lower.contains("mask")
        || p_lower.contains("opacity")
    {
        (
            TfFormat::Bc4,
            false,
            EPixelFormat::BC4,
            false,
            "BC4".to_string(),
        )
    } else if p_lower.contains("lossless") || p_lower.contains("uncompressed") {
        (
            TfFormat::Bc7,
            false,
            EPixelFormat::A8R8G8B8,
            false,
            "RGBA8".to_string(),
        )
    } else {
        (
            TfFormat::Bc1,
            false,
            EPixelFormat::BC1,
            false,
            "BC1".to_string(),
        )
    }
}

fn update_platform_res_labels(
    ui: &CryTifDialog,
    w: u32,
    h: u32,
    reduces: &[i32; 4],
    fmt_name: &str,
) {
    let compute_res = |red: i32| -> (u32, u32) {
        if red < 0 {
            let mult = 1 << (-red as u32);
            (w * mult, h * mult)
        } else {
            let div = 1 << (red as u32);
            ((w / div).max(1), (h / div).max(1))
        }
    };

    let (pc_w, pc_h) = compute_res(reduces[0]);
    let (ps4_w, ps4_h) = compute_res(reduces[1]);
    let (xb_w, xb_h) = compute_res(reduces[2]);
    let (es3_w, es3_h) = compute_res(reduces[3]);

    let mobile_fmt = if fmt_name.starts_with("BC5") {
        "EAC_RG11"
    } else if fmt_name.starts_with("BC4") {
        "EAC_R11"
    } else if fmt_name.contains("Opacity") || fmt_name == "BC7" || fmt_name == "BC7t" {
        "ETC2A"
    } else {
        "ETC2"
    };

    ui.set_pc_res_info(format!("{} x {} ({}, red:{})", pc_w, pc_h, fmt_name, reduces[0]).into());
    ui.set_ps4_res_info(format!("{} x {} ({}, red:{})", ps4_w, ps4_h, fmt_name, reduces[1]).into());
    ui.set_xbox_res_info(format!("{} x {} ({}, red:{})", xb_w, xb_h, fmt_name, reduces[2]).into());
    ui.set_mobile_res_info(
        format!("{} x {} ({}, red:{})", es3_w, es3_h, mobile_fmt, reduces[3]).into(),
    );
}

fn sync_ui_with_preset(ui: &CryTifDialog, preset: &str, ini: &CfgFile) {
    let resolved = ImageCompiler::resolve_preset_alias(ini, preset);
    if let Some(sec_idx) = ini.find_section(&resolved) {
        let sec = &ini.sections[sec_idx];
        let mut rdo_val = 0.0f32;
        let mut discard_a = false;
        let mut coverage = false;
        let mut mips = true;
        let mut cs_idx = 0;
        let mut bump_type = 0;

        for entry in &sec.entries {
            match entry.key.to_ascii_lowercase().as_str() {
                "rdo_lambda" | "rdo" => rdo_val = entry.value.parse().unwrap_or(0.0),
                "discardalpha" => {
                    discard_a = entry.value == "1" || entry.value.eq_ignore_ascii_case("true")
                }
                "alphacoverage" => {
                    coverage = entry.value == "1" || entry.value.eq_ignore_ascii_case("true")
                }
                "mipmaps" => {
                    mips = entry.value != "0" && !entry.value.eq_ignore_ascii_case("false")
                }
                "bumptype" => bump_type = entry.value.parse().unwrap_or(1),
                "colorspace" => {
                    let parts: Vec<&str> = entry.value.split(',').collect();
                    if parts.len() == 2 {
                        cs_idx = match parts[1].trim().to_ascii_lowercase().as_str() {
                            "srgb" => 1,
                            "linear" => 2,
                            _ => 0,
                        };
                    }
                }
                _ => {}
            }
        }

        ui.set_rdo_lambda(rdo_val);
        ui.set_discard_alpha(discard_a);
        ui.set_maintain_alpha_coverage(coverage);
        ui.set_generate_mips(mips);
        ui.set_colorspace_index(cs_idx);
        ui.set_format_override_index(0); // Reset override to 'From Preset'
        ui.set_rgb_bump_type(bump_type);
    }
}

/// Renders a procedural canvas with a neutral checkerboard and normalized UV viewport mapping.
fn render_preview_canvas(
    src_rgba: &[u8],
    width: usize,
    height: usize,
    channel_mode: i32,
    zoom_percent: i32,
    tiled: bool,
) -> (Vec<u8>, usize, usize) {
    let canvas_size = 1024usize;
    let mut out = vec![255u8; canvas_size * canvas_size * 4];
    let zoom = (zoom_percent as f32) / 100.0;

    let chk_light = [46u8, 48u8, 56u8];
    let chk_dark = [26u8, 27u8, 32u8];

    let u_min = 0.5 - (zoom * 0.5);
    let u_max = 0.5 + (zoom * 0.5);
    let v_min = 0.5 - (zoom * 0.5);
    let v_max = 0.5 + (zoom * 0.5);

    for cy in 0..canvas_size {
        let norm_y = cy as f32 / canvas_size as f32;

        for cx in 0..canvas_size {
            let norm_x = cx as f32 / canvas_size as f32;
            let dst_idx = (cy * canvas_size + cx) * 4;

            let is_light = ((cx / 8) + (cy / 8)) % 2 == 0;
            let bg = if is_light { chk_light } else { chk_dark };

            let (has_texel, src_x, src_y) = if tiled {
                let u = norm_x / zoom;
                let v = norm_y / zoom;
                let u_fract = u.rem_euclid(1.0);
                let v_fract = v.rem_euclid(1.0);
                let tx = ((u_fract * width as f32).floor() as usize).min(width - 1);
                let ty = ((v_fract * height as f32).floor() as usize).min(height - 1);
                (true, tx, ty)
            } else if norm_x >= u_min && norm_x < u_max && norm_y >= v_min && norm_y < v_max {
                let local_u = (norm_x - u_min) / zoom;
                let local_v = (norm_y - v_min) / zoom;
                let tx = ((local_u * width as f32).floor() as usize).min(width - 1);
                let ty = ((local_v * height as f32).floor() as usize).min(height - 1);
                (true, tx, ty)
            } else {
                (false, 0, 0)
            };

            if has_texel {
                let src_idx = (src_y * width + src_x) * 4;
                let r = src_rgba[src_idx];
                let g = src_rgba[src_idx + 1];
                let b = src_rgba[src_idx + 2];
                let a = src_rgba[src_idx + 3];

                match channel_mode {
                    0 => {
                        out[dst_idx] = r;
                        out[dst_idx + 1] = g;
                        out[dst_idx + 2] = b;
                        out[dst_idx + 3] = 255;
                    }
                    1 => {
                        out[dst_idx] = a;
                        out[dst_idx + 1] = a;
                        out[dst_idx + 2] = a;
                        out[dst_idx + 3] = 255;
                    }
                    2 => {
                        let alpha_f = a as f32 / 255.0;
                        let inv_a = 1.0 - alpha_f;
                        out[dst_idx] = (r as f32 * alpha_f + bg[0] as f32 * inv_a).round() as u8;
                        out[dst_idx + 1] =
                            (g as f32 * alpha_f + bg[1] as f32 * inv_a).round() as u8;
                        out[dst_idx + 2] =
                            (b as f32 * alpha_f + bg[2] as f32 * inv_a).round() as u8;
                        out[dst_idx + 3] = 255;
                    }
                    3 => {
                        let nx = r as f32 / 127.5 - 1.0;
                        let ny = g as f32 / 127.5 - 1.0;
                        let len_sq = nx * nx + ny * ny;
                        if len_sq > 1.0 {
                            out[dst_idx] = 255;
                            out[dst_idx + 1] = 40;
                            out[dst_idx + 2] = 40;
                        } else {
                            let nz = (1.0 - len_sq).sqrt();
                            out[dst_idx] = ((nx * 0.5 + 0.5) * 255.0).round() as u8;
                            out[dst_idx + 1] = ((ny * 0.5 + 0.5) * 255.0).round() as u8;
                            out[dst_idx + 2] = ((nz * 0.5 + 0.5) * 255.0).round() as u8;
                        }
                        out[dst_idx + 3] = 255;
                    }
                    4 | 5 => {
                        if a >= 128 {
                            out[dst_idx] = r;
                            out[dst_idx + 1] = g;
                            out[dst_idx + 2] = b;
                        } else {
                            out[dst_idx] = bg[0];
                            out[dst_idx + 1] = bg[1];
                            out[dst_idx + 2] = bg[2];
                        }
                        out[dst_idx + 3] = 255;
                    }
                    _ => {
                        out[dst_idx] = r;
                        out[dst_idx + 1] = g;
                        out[dst_idx + 2] = b;
                        out[dst_idx + 3] = 255;
                    }
                }
            } else {
                out[dst_idx] = bg[0];
                out[dst_idx + 1] = bg[1];
                out[dst_idx + 2] = bg[2];
                out[dst_idx + 3] = 255;
            }
        }
    }

    (out, canvas_size, canvas_size)
}

/// Computes the exact, synchronized compression preview for the target window and specific Mip level.
fn update_previews_both(
    ui: &CryTifDialog,
    src_rgba: &[u8],
    width: usize,
    height: usize,
    preset_name: &str,
    ini_file: &CfgFile,
) {
    let preview_on = ui.get_preview_on();
    let tiled = ui.get_tiled_preview();
    let channel_mode = ui.get_channel_mode_index();
    let zoom_percent = ui.get_zoom_percent();

    // 1. Update Helper Description Text
    let helper_desc = match channel_mode {
        0 => "Normal RGB preview mode (alpha channel ignored)",
        1 => "Alpha transparency channel in grayscale",
        2 => "RGB color composited with transparency over 8x8 checkerboard",
        3 => "Normal vector length deviation heatmap (Red = Non-unit vector error)",
        4 => "AlphaTest visualized with cutoff of 0.5 and background transparency",
        5 => "AlphaTest hard cutoff at 0.5 (no blending)",
        _ => "Color model conversion preview",
    };
    ui.set_channel_mode_desc(helper_desc.into());

    // 2. Resolve Base Configuration and Process Source Image
    let (_, _, mut engine_fmt, is_normal, _) =
        resolve_preset_format(preset_name, ui.get_format_override_index(), ini_file);

    let mut processed_rgba = src_rgba.to_vec();

    if ui.get_discard_alpha() && !is_normal {
        for px in processed_rgba.chunks_exact_mut(4) {
            px[3] = 255;
        }
    }

    let rgb_filter = match ui.get_rgb_bump_type() {
        1 => NormalFilterType::Scharr3x3,
        2 => NormalFilterType::Sobel3x3,
        3 => NormalFilterType::Farid5x5,
        4 => NormalFilterType::Gauss,
        _ => NormalFilterType::None,
    };

    if rgb_filter != NormalFilterType::None {
        let mut float_pixels: Vec<cry_image::ColorRGBAf> = processed_rgba
            .chunks_exact(4)
            .map(|c| cry_image::ColorRGBAf {
                r: c[0] as f32 / 255.0,
                g: c[1] as f32 / 255.0,
                b: c[2] as f32 / 255.0,
                a: c[3] as f32 / 255.0,
            })
            .collect();

        let props = BumpProperties {
            filter_type: rgb_filter,
            bump_strength: ui.get_rgb_bump_strength(),
            blur_amount: ui.get_rgb_bump_blur(),
            invert: ui.get_rgb_bump_invert(),
        };

        float_pixels =
            NormalProcessing::bump_to_normal_map(&float_pixels, width, height, &props, false);

        for (i, p) in float_pixels.iter().enumerate() {
            processed_rgba[i * 4] = (p.r * 255.0) as u8;
            processed_rgba[i * 4 + 1] = (p.g * 255.0) as u8;
            processed_rgba[i * 4 + 2] = (p.b * 255.0) as u8;
            processed_rgba[i * 4 + 3] = (p.a * 255.0) as u8;
        }
    }

    let alpha_filter = match ui.get_alpha_bump_type() {
        1 => NormalFilterType::Scharr3x3,
        2 => NormalFilterType::Sobel3x3,
        3 => NormalFilterType::Farid5x5,
        _ => NormalFilterType::None,
    };

    if alpha_filter != NormalFilterType::None {
        let mut float_pixels: Vec<cry_image::ColorRGBAf> = processed_rgba
            .chunks_exact(4)
            .map(|c| cry_image::ColorRGBAf {
                r: c[0] as f32 / 255.0,
                g: c[1] as f32 / 255.0,
                b: c[2] as f32 / 255.0,
                a: c[3] as f32 / 255.0,
            })
            .collect();

        let props = BumpProperties {
            filter_type: alpha_filter,
            bump_strength: ui.get_alpha_bump_strength(),
            blur_amount: ui.get_alpha_bump_blur(),
            invert: ui.get_alpha_bump_invert(),
        };

        float_pixels =
            NormalProcessing::bump_to_normal_map(&float_pixels, width, height, &props, true);

        for (i, p) in float_pixels.iter().enumerate() {
            processed_rgba[i * 4] = (p.r * 255.0) as u8;
            processed_rgba[i * 4 + 1] = (p.g * 255.0) as u8;
            processed_rgba[i * 4 + 2] = (p.b * 255.0) as u8;
            processed_rgba[i * 4 + 3] = (p.a * 255.0) as u8;
        }
    }

    // Smart 1-bit Alpha / Discard Alpha Cleanup
    let is_1bit_alpha = engine_fmt == EPixelFormat::BC1a || ui.get_maintain_alpha_coverage();
    let is_bc1_opaque = engine_fmt == EPixelFormat::BC1 && !is_1bit_alpha;

    if is_bc1_opaque || ui.get_discard_alpha() {
        for p in processed_rgba.chunks_exact_mut(4) {
            p[3] = 255;
        }
    } else if is_1bit_alpha {
        for p in processed_rgba.chunks_exact_mut(4) {
            p[3] = if p[3] < 127 { 0 } else { 255 };
        }
    }

    // 3. Mipmap Generation & Resolution
    let use_srgb = match ui.get_colorspace_index() {
        1 => true,
        2 => false,
        _ => !is_normal,
    };

    let alpha_cov = if ui.get_maintain_alpha_coverage() || preset_name.contains("Coverage") {
        Some(cry_image::AlphaCoverageOptions { alpha_cutoff: 0.5 })
    } else {
        None
    };

    // Dynamically query selected mipmap filter
    let filter_method = match ui.get_mip_filter_index() {
        0 => MipmapFilter::Box,
        1 => MipmapFilter::MitchellNetravali,
        2 => MipmapFilter::CatmullRom,
        3 => MipmapFilter::Lanczos3,
        4 => MipmapFilter::KaiserSinc,
        5 => MipmapFilter::Point,
        _ => MipmapFilter::MitchellNetravali,
    };

    let mut mip_chain = if ui.get_generate_mips() {
        cry_image::generate_mipmaps_rgba(
            &processed_rgba,
            width,
            height,
            filter_method,
            use_srgb,
            alpha_cov,
        )
    } else {
        vec![MipmapLevel {
            width,
            height,
            data: processed_rgba.clone(),
        }]
    };

    let is_uncompressed_initial = matches!(
        engine_fmt,
        EPixelFormat::A8R8G8B8 | EPixelFormat::X8R8G8B8 | EPixelFormat::R8G8B8
    );

    // Truncate to maximum engine supported mips
    let max_mips = compute_max_mip_count(width, height, !is_uncompressed_initial);
    if mip_chain.len() > max_mips {
        mip_chain.truncate(max_mips);
    }

    let total_mips = mip_chain.len();
    ui.set_mip_count(total_mips as i32);

    let mut mip_idx = ui.get_mip_level() as usize;
    if mip_idx >= total_mips {
        mip_idx = total_mips - 1;
        ui.set_mip_level(mip_idx as i32);
    }

    let preview_mip = &mip_chain[mip_idx];
    let mip_w = preview_mip.width;
    let mip_h = preview_mip.height;

    // 4. Render Source Preview (Left Window)
    let (processed_source, view_w, view_h) = render_preview_canvas(
        &preview_mip.data,
        mip_w,
        mip_h,
        channel_mode,
        zoom_percent,
        tiled,
    );

    let mut src_pixel_buffer = SharedPixelBuffer::<Rgba8Pixel>::new(view_w as u32, view_h as u32);
    src_pixel_buffer
        .make_mut_slice()
        .copy_from_slice(bytemuck_cast_slice(&processed_source));

    ui.set_source_preview(Image::from_rgba8(src_pixel_buffer.clone()));
    ui.set_source_info(
        format!(
            "{}x{} (Mip {}) Fmt: A8R8G8B8{}",
            mip_w,
            mip_h,
            mip_idx,
            if tiled { " [Tiled]" } else { "" }
        )
        .into(),
    );

    if !preview_on {
        ui.set_target_preview(Image::from_rgba8(src_pixel_buffer));
        ui.set_target_info("[Preview OFF] Compression Bypassed".into());
        return;
    }

    // 5. Smart Upgrade/Downgrade Engine Format
    let has_real_alpha =
        !ui.get_discard_alpha() && preview_mip.data.chunks_exact(4).any(|p| p[3] < 255);
    engine_fmt = CPixelFormats::get_final_pixel_format(engine_fmt, has_real_alpha);

    let (tf_format, is_signed) = map_engine_format_to_tf(engine_fmt);
    let fmt_display_name = format!("{:?}", engine_fmt);

    let rdo_val = ui.get_rdo_lambda();
    let weight_by_alpha = !is_normal
        && !ui.get_discard_alpha()
        && matches!(
            engine_fmt,
            EPixelFormat::BC2 | EPixelFormat::BC3 | EPixelFormat::BC7 | EPixelFormat::BC7t
        );

    let compress_opts = CompressionOptions {
        format: tf_format,
        metric: if is_normal {
            ColorMetric::Unit
        } else if use_srgb {
            ColorMetric::Perceptual
        } else {
            ColorMetric::Uniform
        },
        strategy: FitStrategy::FastRange,
        quality: QualityLevel::Fast,
        weight_by_alpha,
        is_1bit_alpha,
        alpha_iterative_fit: true,
        is_signed,
        is_normal_map: is_normal,
        srgb: use_srgb,
        dither_rgb: false,
        dither_a: false,
        rdo_lambda: rdo_val,
        rdo_ultrasmooth: ui.get_rdo_ultrasmooth(),
        rdo_lookback_window: 256,
        rdo_try_two_matches: true,
        rdo_smooth_block_scale: None,
    };

    // 6. Compress and Decompress using Exact Pipeline (Only the selected Mip)
    let decompressed = if is_uncompressed_initial {
        preview_mip.data.clone()
    } else {
        let compressed = cry_image::compress_image(&preview_mip.data, mip_w, mip_h, compress_opts);
        decompress_image(&compressed, mip_w, mip_h, tf_format)
    };

    // Calculate Accurate DDS Size with Mipmaps (for full chain)
    let mut total_dds_bytes = 148usize; // DDS + DX10 header
    let mut cur_w = width;
    let mut cur_h = height;
    for _ in 0..total_mips {
        total_dds_bytes += get_storage_requirements(cur_w, cur_h, tf_format);
        cur_w = (cur_w / 2).max(1);
        cur_h = (cur_h / 2).max(1);
    }

    // 7. Render Target Preview (Right Window)
    let (processed_target, t_w, t_h) = render_preview_canvas(
        &decompressed,
        mip_w,
        mip_h,
        channel_mode,
        zoom_percent,
        tiled,
    );

    let mut target_pixel_buffer = SharedPixelBuffer::<Rgba8Pixel>::new(t_w as u32, t_h as u32);
    target_pixel_buffer
        .make_mut_slice()
        .copy_from_slice(bytemuck_cast_slice(&processed_target));

    ui.set_target_preview(Image::from_rgba8(target_pixel_buffer));
    ui.set_target_info(
        format!(
            "{}x{} Mips: {} Fmt: {} Mdl: {}{} ({} KB)",
            mip_w,
            mip_h,
            total_mips,
            fmt_display_name,
            if is_normal {
                "Linear"
            } else if use_srgb {
                "sRGB"
            } else {
                "Linear"
            },
            if tiled { " [Tiled]" } else { "" },
            total_dds_bytes / 1024
        )
        .into(),
    );
}

fn load_all_engine_presets(ini_file: &CfgFile) -> Vec<String> {
    let mut preset_names: Vec<String> = Vec::new();
    for sec in &ini_file.sections {
        if !sec.name.is_empty() && !sec.name.starts_with('_') {
            preset_names.push(sec.name.clone());
        }
    }

    if preset_names.len() <= 1 {
        preset_names = vec![
            "Albedo".to_string(),
            "AlbedoWithOpacity".to_string(),
            "AlbedoWithCoverage".to_string(),
            "AlbedoWithGenericAlpha".to_string(),
            "AlbedoWithGenericAlphaNoMip".to_string(),
            "Reflectance".to_string(),
            "Reflectance_Linear".to_string(),
            "ReflectanceWithSmoothness_Legacy".to_string(),
            "Normals".to_string(),
            "NormalsWithSmoothness".to_string(),
            "NormalsWithSmoothness_Legacy".to_string(),
            "NormalsFromDisplacement".to_string(),
            "Displacement".to_string(),
            "Terrain_Albedo".to_string(),
            "Decal_AlbedoWithOpacity".to_string(),
            "Detail_MergedAlbedoNormalsSmoothness".to_string(),
            "Detail_MergedAlbedoNormalsSmoothness_Lossless".to_string(),
            "EnvironmentProbeHDR".to_string(),
            "EnvironmentProbeHDR_Irradiance".to_string(),
            "SkyboxLDR".to_string(),
            "SkyboxHDR".to_string(),
            "ColorChart".to_string(),
            "Greyscale".to_string(),
            "Opacity".to_string(),
            "Uncompressed".to_string(),
            "UserInterface_Lossless".to_string(),
            "UserInterface_Compressed".to_string(),
        ];
    }
    preset_names
}

fn filter_matching_presets(filename: &str, all: &[String]) -> Vec<String> {
    let lower = filename.to_ascii_lowercase();
    let mut matching = Vec::new();

    for p in all {
        let p_lower = p.to_ascii_lowercase();
        if (lower.contains("_ddna") && p_lower.contains("smoothness"))
            || (lower.contains("_ddn") && p_lower.starts_with("normal"))
            || (lower.contains("_spec") && p_lower.contains("reflectance"))
            || (lower.contains("_displ") && p_lower.contains("displacement"))
            || (lower.contains("_diff") && p_lower.starts_with("albedo"))
        {
            matching.push(p.clone());
        }
    }

    if matching.is_empty() {
        matching = vec![
            "Albedo".to_string(),
            "AlbedoWithOpacity".to_string(),
            "AlbedoWithCoverage".to_string(),
            "Normals".to_string(),
            "NormalsWithSmoothness".to_string(),
            "Reflectance".to_string(),
        ];
    }

    matching.push("------------------------------------------------".to_string());
    for p in all {
        if !matching.contains(p) {
            matching.push(p.clone());
        }
    }

    matching
}

fn get_preset_config_text(preset: &str, ini: &CfgFile) -> String {
    let resolved = ImageCompiler::resolve_preset_alias(ini, preset);
    if let Some(sec_idx) = ini.find_section(&resolved) {
        let sec = &ini.sections[sec_idx];
        let mut out = format!("[{}]\n", sec.name);
        for entry in &sec.entries {
            if !entry.key.is_empty() {
                out.push_str(&format!("{} = {}\n", entry.key, entry.value));
            }
        }
        out
    } else {
        format!("[{}]\n; Built-in CryEngine preset configuration\n", preset)
    }
}

fn execute_compilation_from_ui(
    ui: &CryTifDialog,
    src_tif: &Path,
    dst_dds: &Path,
    ini_file: &CfgFile,
) -> Result<(), String> {
    let preset_name = ui.get_current_preset().to_string();

    let output_cs = match ui.get_colorspace_index() {
        1 => EOutputColorSpace::Srgb,
        2 => EOutputColorSpace::Linear,
        _ => EOutputColorSpace::Auto,
    };

    let max_size = match ui.get_max_size_index() {
        1 => 4096,
        2 => 2048,
        3 => 1024,
        4 => 512,
        5 => 256,
        _ => 0,
    };

    let (_, _, engine_fmt, _, _) =
        resolve_preset_format(&preset_name, ui.get_format_override_index(), ini_file);

    let props = ImageProperties {
        preset: preset_name,
        pixel_format: Some(engine_fmt),
        input_color_space: EInputColorSpace::Linear,
        output_color_space: output_cs,
        maintain_alpha_coverage: ui.get_maintain_alpha_coverage(),
        discard_alpha: ui.get_discard_alpha(),
        generate_mips: ui.get_generate_mips(),
        max_texture_size: max_size,
        ..Default::default()
    };

    let mut compiler = ImageCompiler::new(props);
    compiler.platform = "pc".to_string();
    compiler.split_for_streaming = ui.get_split_streaming();
    compiler.dither = ui.get_dither();

    let rdo_val = ui.get_rdo_lambda();
    compiler.rdo_lambda = if rdo_val > 0.0 {
        Some(rdo_val)
    } else {
        Some(0.0)
    };

    compiler.quality = match ui.get_quality_index() {
        0 => QualityLevel::Ultrafast,
        1 => QualityLevel::Fast,
        2 => QualityLevel::Normal,
        3 => QualityLevel::Slow,
        4 => QualityLevel::Slowest,
        _ => QualityLevel::Normal,
    };

    compiler.process_file(src_tif, dst_dds, Some(ini_file))?;
    Ok(())
}

fn detect_default_preset(path: &Path, available_presets: &[String]) -> String {
    let filename = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();

    if filename.contains("_ddna")
        && available_presets.contains(&"NormalsWithSmoothness".to_string())
    {
        return "NormalsWithSmoothness".to_string();
    }
    if filename.contains("_ddn") && available_presets.contains(&"Normals".to_string()) {
        return "Normals".to_string();
    }
    if filename.contains("_spec") && available_presets.contains(&"Reflectance".to_string()) {
        return "Reflectance".to_string();
    }
    if filename.contains("_displ") && available_presets.contains(&"Displacement".to_string()) {
        return "Displacement".to_string();
    }
    "Albedo".to_string()
}

fn bytemuck_cast_slice(src: &[u8]) -> &[Rgba8Pixel] {
    unsafe {
        std::slice::from_raw_parts(
            src.as_ptr() as *const Rgba8Pixel,
            src.len() / std::mem::size_of::<Rgba8Pixel>(),
        )
    }
}
