// Copyright 2004-2026 Crytek GmbH / Crytek Group. All rights reserved.
// Native Rust Photoshop CryTIF File Format Plugin (.8bi)

#![allow(non_snake_case)]

pub mod photoshop_abi;
mod rc_invoker;

use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use flate2::Compression;
use flate2::write::ZlibEncoder;
use photoshop_abi::*;
use rc_invoker::RcInvoker;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Entry point invoked directly by Adobe Photoshop via C-ABI.
///
/// # Safety
/// The host guarantees `format_param_block` and `result` are valid, aligned pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PluginMain(
    selector: i16,
    format_param_block: *mut FormatRecord,
    _data: *mut usize,
    result: *mut i16,
) {
    if format_param_block.is_null() || result.is_null() {
        return;
    }

    let record = unsafe { &mut *format_param_block };

    // Enable 32-bit coordinate fields for modern Photoshop versions (CS+)
    if record.host_supports_32bit_coordinates != 0 {
        record.plugin_using_32bit_coordinates = 1;
    }

    match selector {
        FORMAT_SELECTOR_ABOUT => {
            RcInvoker::show_error_dialog(
                "CryTIF Photoshop Export Plugin (Rust 64-bit)\nVersion 1.2.0\nCrytek GmbH / CryEngine Rust Toolchain",
            );
            unsafe {
                *result = NO_ERR;
            }
        }
        FORMAT_SELECTOR_READ_PREPARE
        | FORMAT_SELECTOR_WRITE_PREPARE
        | FORMAT_SELECTOR_OPTIONS_PREPARE
        | FORMAT_SELECTOR_OPTIONS_START
        | FORMAT_SELECTOR_ESTIMATE_PREPARE
        | FORMAT_SELECTOR_ESTIMATE_START => {
            record.max_data = 0;
            unsafe {
                *result = NO_ERR;
            }
        }
        FORMAT_SELECTOR_WRITE_START => {
            let write_result = unsafe { handle_photoshop_write(record) };
            unsafe {
                *result = match write_result {
                    Ok(_) => NO_ERR,
                    Err(e) => {
                        RcInvoker::show_error_dialog(&format!("CryTIF Export Failed:\n{}", e));
                        FORMAT_CANNOT_WRITE
                    }
                };
            }
        }
        FORMAT_SELECTOR_FILTER_FILE => unsafe {
            *result = NO_ERR;
        },
        _ => unsafe {
            *result = NO_ERR;
        },
    }
}

/// Reads scanline pixel data across planes from Photoshop using safe Row-by-Plane iteration
unsafe fn handle_photoshop_write(record: &mut FormatRecord) -> Result<(), String> {
    let width = if record.plugin_using_32bit_coordinates != 0 {
        record.image_size32.h as usize
    } else {
        record.image_size.h as usize
    };

    let height = if record.plugin_using_32bit_coordinates != 0 {
        record.image_size32.v as usize
    } else {
        record.image_size.v as usize
    };

    let planes = record.planes as usize;
    let depth = record.depth as usize;

    if width == 0 || height == 0 || (planes != 1 && planes != 3 && planes != 4) {
        return Err(format!(
            "Invalid format parameters: {}x{}, {} planes, {}-bit",
            width, height, planes, depth
        ));
    }

    let target_file_path = get_target_filename(record);
    let override_filename = target_file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // 1. Allocate scanline exchange buffer for ONE plane of ONE row (Robust against PS internal tiling)
    let row_bytes = (width * depth).div_ceil(8);
    let mut scanline_buffer = vec![0u8; row_bytes];
    record.data = scanline_buffer.as_mut_ptr() as *mut std::ffi::c_void;

    record.col_bytes = depth.div_ceil(8) as i16;
    record.row_bytes = row_bytes as i32;
    record.plane_bytes = 0;

    let advance_state = record
        .advance_state
        .ok_or_else(|| "Photoshop advanceState callback is null".to_string())?;

    // 2. Read existing preset from file or set default
    let mut iptc_instructions = String::new();
    if target_file_path.exists()
        && let Ok(existing) = cry_image::CryTifIO::read_special_instructions(&target_file_path)
        && !existing.is_empty()
    {
        iptc_instructions = existing;
    }

    if iptc_instructions.is_empty() {
        iptc_instructions = match planes {
            1 => "/preset=Greyscale /reduce=0 /colorspace=linear,linear".to_string(),
            3 => "/preset=Albedo /reduce=0 /colorspace=sRGB,auto".to_string(),
            _ => "/preset=AlbedoWithOpacity /reduce=0 /colorspace=sRGB,auto".to_string(),
        };
    }

    // 3. Scanline processing loop (Row -> Plane)
    if depth == 32 {
        // --- 32-bit Float -> 16-bit Half-Float CryTIF (IEEE FP16) ---
        let mut half_pixels = vec![0u16; width * height * 4];

        for row in 0..height {
            set_the_rect(record, row, width);

            for plane in 0..planes {
                record.lo_plane = plane as i16;
                record.hi_plane = plane as i16;

                let status = unsafe { advance_state() };
                if status != NO_ERR {
                    return Err(format!(
                        "advanceState failed at row {} plane {}",
                        row, plane
                    ));
                }

                let src_floats =
                    unsafe { std::slice::from_raw_parts(record.data as *const f32, width) };
                for (col, &val) in src_floats.iter().take(width).enumerate() {
                    let pixel_idx = (row * width + col) * 4 + plane;
                    half_pixels[pixel_idx] = half::f16::from_f32(val).to_bits();
                }
            }

            if planes == 3 {
                for col in 0..width {
                    let alpha_idx = (row * width + col) * 4 + 3;
                    half_pixels[alpha_idx] = half::f16::from_f32(1.0).to_bits();
                }
            }
        }

        save_crytif_16bit(
            &target_file_path,
            width as u32,
            height as u32,
            &half_pixels,
            &iptc_instructions,
        )?;
    } else {
        // --- 8-bit / 16-bit Unorm -> 8-bit RGBA CryTIF ---
        let mut rgba_pixels = vec![255u8; width * height * 4];

        for row in 0..height {
            set_the_rect(record, row, width);

            for plane in 0..planes {
                record.lo_plane = plane as i16;
                record.hi_plane = plane as i16;

                let status = unsafe { advance_state() };
                if status != NO_ERR {
                    return Err(format!(
                        "advanceState failed at row {} plane {}",
                        row, plane
                    ));
                }

                if depth == 16 {
                    // Remap Photoshop's internal 15-bit+1 range [0..32768] to 8-bit [0..255]
                    let src_u16 =
                        unsafe { std::slice::from_raw_parts(record.data as *const u16, width) };
                    for (col, &raw_val) in src_u16.iter().take(width).enumerate() {
                        let val = (((raw_val as u32) * 255 + 16384) / 32768).min(255) as u8;
                        rgba_pixels[(row * width + col) * 4 + plane] = val;
                    }
                } else {
                    let src_u8 =
                        unsafe { std::slice::from_raw_parts(record.data as *const u8, width) };
                    for (col, &raw_val) in src_u8.iter().take(width).enumerate() {
                        rgba_pixels[(row * width + col) * 4 + plane] = raw_val;
                    }
                }
            }

            if planes == 1 {
                for col in 0..width {
                    let grey = rgba_pixels[(row * width + col) * 4];
                    rgba_pixels[(row * width + col) * 4 + 1] = grey;
                    rgba_pixels[(row * width + col) * 4 + 2] = grey;
                    rgba_pixels[(row * width + col) * 4 + 3] = 255;
                }
            }
        }

        cry_image::CryTifIO::save_crytif_rgba8(
            &target_file_path,
            width as u32,
            height as u32,
            &rgba_pixels,
            &iptc_instructions,
        )
        .map_err(|e| e.to_string())?;
    }

    record.data = std::ptr::null_mut();

    // 4. Trigger Resource Compiler
    RcInvoker::invoke_rc(&target_file_path, &override_filename)?;
    Ok(())
}

#[inline]
fn set_the_rect(record: &mut FormatRecord, row: usize, width: usize) {
    if record.plugin_using_32bit_coordinates != 0 {
        record.the_rect32.left = 0;
        record.the_rect32.right = width as i32;
        record.the_rect32.top = row as i32;
        record.the_rect32.bottom = (row + 1) as i32;
    } else {
        record.the_rect.left = 0;
        record.the_rect.right = width.min(32767) as i16;
        record.the_rect.top = row.min(32767) as i16;
        record.the_rect.bottom = (row + 1).min(32767) as i16;
    }
}

fn get_target_filename(record: &FormatRecord) -> PathBuf {
    if let Some(spec2) = unsafe { record.file_spec2.as_ref() }
        && !spec2.m_reference.is_null()
    {
        let mut len = 0;
        unsafe {
            let ptr = spec2.m_reference;
            while *ptr.add(len) != 0 {
                len += 1;
            }
            let slice = std::slice::from_raw_parts(ptr, len);
            return PathBuf::from(String::from_utf16_lossy(slice));
        }
    }

    if let Some(spec) = unsafe { record.file_spec.as_ref() } {
        let name_len = spec.name[0] as usize;
        if name_len > 0 && name_len < 255 {
            let str_bytes = &spec.name[1..=name_len];
            return PathBuf::from(String::from_utf8_lossy(str_bytes).to_string());
        }
    }

    std::env::temp_dir().join("photoshop_export.tif")
}

fn save_crytif_16bit(
    path: &Path,
    width: u32,
    height: u32,
    samples: &[u16],
    instructions: &str,
) -> Result<(), String> {
    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut w = io::BufWriter::new(file);

    let mut raw_bytes = Vec::with_capacity(samples.len() * 2);
    for &sample in samples {
        raw_bytes.write_u16::<LittleEndian>(sample).unwrap();
    }

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw_bytes).map_err(|e| e.to_string())?;
    let compressed_strip = encoder.finish().map_err(|e| e.to_string())?;

    let mut iptc = cry_image::IptcHeader::new();
    iptc.fields.insert(
        cry_image::FIELD_SPECIAL_INSTRUCTIONS,
        vec![instructions.as_bytes().to_vec()],
    );
    let iptc_bytes = iptc.build_bytes();

    let mut photoshop_data = Vec::new();
    photoshop_data.extend_from_slice(b"8BIM");
    photoshop_data.write_u16::<BigEndian>(0x0404).unwrap();
    photoshop_data.write_u16::<BigEndian>(0x0000).unwrap();
    photoshop_data
        .write_u32::<BigEndian>(iptc_bytes.len() as u32)
        .unwrap();
    photoshop_data.extend_from_slice(&iptc_bytes);
    if photoshop_data.len() % 2 != 0 {
        photoshop_data.push(0);
    }

    w.write_all(b"II\x2A\x00").map_err(|e| e.to_string())?;
    w.write_u32::<LittleEndian>(8).unwrap();

    let num_entries = 15u16; // 15 Sorted TIFF Tags
    w.write_u16::<LittleEndian>(num_entries).unwrap();

    let mut extra_data = Vec::new();
    let bits_per_sample_off = 8 + 2 + (num_entries as u32 * 12) + 4 + extra_data.len() as u32;
    for _ in 0..4 {
        extra_data.write_u16::<LittleEndian>(16).unwrap();
    }

    let ps_data_off = 8 + 2 + (num_entries as u32 * 12) + 4 + extra_data.len() as u32;
    extra_data.write_all(&photoshop_data).unwrap();

    let strip_off = 8 + 2 + (num_entries as u32 * 12) + 4 + extra_data.len() as u32;
    extra_data.write_all(&compressed_strip).unwrap();

    let mut write_tag = |tag: u16, t_type: u16, count: u32, val_or_off: u32| -> io::Result<()> {
        w.write_u16::<LittleEndian>(tag)?;
        w.write_u16::<LittleEndian>(t_type)?;
        w.write_u32::<LittleEndian>(count)?;
        w.write_u32::<LittleEndian>(val_or_off)?;
        Ok(())
    };

    write_tag(256, 4, 1, width).map_err(|e| e.to_string())?;
    write_tag(257, 4, 1, height).map_err(|e| e.to_string())?;
    write_tag(258, 3, 4, bits_per_sample_off).map_err(|e| e.to_string())?;
    write_tag(259, 3, 1, 8).map_err(|e| e.to_string())?;
    write_tag(262, 3, 1, 2).map_err(|e| e.to_string())?;
    write_tag(273, 4, 1, strip_off).map_err(|e| e.to_string())?;
    write_tag(274, 3, 1, 1).map_err(|e| e.to_string())?;
    write_tag(277, 3, 1, 4).map_err(|e| e.to_string())?;
    write_tag(278, 4, 1, height).map_err(|e| e.to_string())?;
    write_tag(279, 4, 1, compressed_strip.len() as u32).map_err(|e| e.to_string())?;
    write_tag(284, 3, 1, 1).map_err(|e| e.to_string())?;
    write_tag(317, 3, 1, 1).map_err(|e| e.to_string())?;
    write_tag(338, 3, 1, 2).map_err(|e| e.to_string())?; // Tag 338: ExtraSamples = 2 (Unassociated Alpha)
    write_tag(339, 3, 1, 3).map_err(|e| e.to_string())?; // Tag 339: SampleFormat = 3 (IEEE Float)
    write_tag(34377, 1, photoshop_data.len() as u32, ps_data_off).map_err(|e| e.to_string())?;

    w.write_u32::<LittleEndian>(0).unwrap();
    w.write_all(&extra_data).map_err(|e| e.to_string())?;
    w.flush().map_err(|e| e.to_string())?;
    Ok(())
}
