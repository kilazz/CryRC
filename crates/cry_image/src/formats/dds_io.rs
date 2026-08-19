// Copyright 2001-2026 Crytek GmbH / Crytek Group. All rights reserved.
// DirectDraw Surface (DDS) File Container I/O with DX10 Header & CryEngine Extensions

use crate::pixel_formats::DxgiFormat;
use byteorder::{LittleEndian, WriteBytesExt};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

pub struct DdsIO;

impl DdsIO {
    #[allow(clippy::too_many_arguments)]
    pub fn save_dds_file(
        path: &Path,
        width: u32,
        height: u32,
        mip_count: u32,
        dxgi_format: DxgiFormat,
        is_srgb: bool,
        is_cubemap: bool,
        compressed_payload: &[u8],
    ) -> io::Result<()> {
        let file = File::create(path)?;
        let mut w = io::BufWriter::new(file);

        // 1. Magic 'DDS '
        w.write_u32::<LittleEndian>(0x20534444)?;

        // 2. Standard DDS_HEADER (124 bytes)
        w.write_u32::<LittleEndian>(124)?;
        w.write_u32::<LittleEndian>(0x1 | 0x2 | 0x4 | 0x1000 | 0x20000 | 0x80000)?;
        w.write_u32::<LittleEndian>(height)?;
        w.write_u32::<LittleEndian>(width)?;

        // Accurate dwPitchOrLinearSize (Size of Mip 0 only)
        let block_w = width.div_ceil(4).max(1);
        let block_h = height.div_ceil(4).max(1);
        let bytes_per_block = match dxgi_format {
            DxgiFormat::BC1Unorm
            | DxgiFormat::BC1UnormSrgb
            | DxgiFormat::BC4Unorm
            | DxgiFormat::BC4Snorm => 8,
            _ => 16,
        };
        let mip0_linear_size = block_w * block_h * bytes_per_block;
        w.write_u32::<LittleEndian>(mip0_linear_size)?;

        w.write_u32::<LittleEndian>(0)?; // Depth
        w.write_u32::<LittleEndian>(mip_count)?;

        // CryEngine DDS extensions
        w.write_u32::<LittleEndian>(0x46595243)?; // 'CRYF'
        let srgb_flag = if is_srgb { 0x8 } else { 0x0 }; // EIF_SRGBREAD
        w.write_u32::<LittleEndian>(srgb_flag)?;
        for _ in 0..9 {
            w.write_u32::<LittleEndian>(0)?;
        }

        // DDS_PIXELFORMAT
        w.write_u32::<LittleEndian>(32)?;
        w.write_u32::<LittleEndian>(0x4)?; // DDPF_FOURCC
        w.write_all(b"DX10")?;
        w.write_u32::<LittleEndian>(0)?;
        w.write_u32::<LittleEndian>(0)?;
        w.write_u32::<LittleEndian>(0)?;
        w.write_u32::<LittleEndian>(0)?;
        w.write_u32::<LittleEndian>(0)?;

        // Caps
        w.write_u32::<LittleEndian>(0x1000 | 0x400000 | 0x8)?;
        w.write_u32::<LittleEndian>(if is_cubemap { 0xFE00 } else { 0 })?;
        w.write_u32::<LittleEndian>(0)?;
        w.write_u32::<LittleEndian>(0)?;
        w.write_u32::<LittleEndian>(0)?;

        // 3. DDS_HEADER_DXT10 (20 bytes)
        w.write_u32::<LittleEndian>(dxgi_format as u32)?;
        w.write_u32::<LittleEndian>(3)?; // D3D10_RESOURCE_DIMENSION_TEXTURE2D
        w.write_u32::<LittleEndian>(if is_cubemap { 0x4 } else { 0 })?;
        w.write_u32::<LittleEndian>(if is_cubemap { 6 } else { 1 })?;
        w.write_u32::<LittleEndian>(0)?;

        // 4. Data payload
        w.write_all(compressed_payload)?;
        w.flush()?;
        Ok(())
    }
}
