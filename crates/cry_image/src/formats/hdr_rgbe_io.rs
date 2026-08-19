// Copyright 2001-2026 Crytek GmbH / Crytek Group. All rights reserved.
// Radiance RGBE / 32-bit Floating Point HDR Image I/O

use crate::math::vector::Vec4;
use std::path::Path;

pub struct HdrRgbeIO;

impl HdrRgbeIO {
    /// Loads a 32-bit floating-point HDR image (.hdr / .exr / .tif) while preserving the full dynamic range.
    pub fn load_hdr(path: &Path) -> Result<(usize, usize, Vec<Vec4>, String), String> {
        let dyn_img =
            image::open(path).map_err(|e| format!("Failed to open HDR image {:?}: {}", path, e))?;

        // Extract native 32-bit floating-point RGBA channels without 8-bit clamping
        let rgba_f32 = dyn_img.to_rgba32f();
        let (width, height) = rgba_f32.dimensions();

        let raw_floats = rgba_f32.into_raw();
        let pixels: Vec<Vec4> = raw_floats
            .chunks_exact(4)
            .map(|c| Vec4::new(c[0], c[1], c[2], c[3]))
            .collect();

        Ok((width as usize, height as usize, pixels, String::new()))
    }
}
