// Copyright 2001-2026 Crytek GmbH / Crytek Group. All rights reserved.
// DirectDraw Surface (DDS) File Container I/O with CryEngine Native FourCC & CExt/AttC Chunks

use crate::pixel_formats::DxgiFormat;
use byteorder::{LittleEndian, WriteBytesExt};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

pub struct DdsIO;

impl DdsIO {
    /// Saves a DDS container with the complete CryEngine header structure and optional embedded CExt/AttC chunks.
    #[allow(clippy::too_many_arguments)]
    pub fn save_dds_file(
        path: &Path,
        width: u32,
        height: u32,
        mip_count: u32,
        dxgi_format: DxgiFormat,
        is_srgb: bool,
        is_cubemap: bool,
        has_attached_alpha: bool,
        is_renormalized: bool,
        is_file_single: bool,
        main_payload: &[u8],
        attached_alpha_payload: Option<&[u8]>,
    ) -> io::Result<()> {
        let file = File::create(path)?;
        let mut w = io::BufWriter::new(file);

        // 1. Build CryEngine texture flags (dwReserved1[1])
        let mut cry_flags = 0u32;
        if is_cubemap {
            cry_flags |= 0x1; // EIF_Cubemap
        }
        if is_srgb {
            cry_flags |= 0x8; // EIF_SRGBRead
        }
        if is_file_single {
            cry_flags |= 0x10; // EIF_FileSingle
        }
        if has_attached_alpha {
            cry_flags |= 0x20; // EIF_AttachedAlpha
        }
        if is_renormalized {
            cry_flags |= 0x40000; // EIF_RenormalizedTexture
        }

        // Determine if the primary image uses a DX10 header
        let main_needs_dx10 = matches!(
            dxgi_format,
            DxgiFormat::BC1Unorm
                | DxgiFormat::BC1UnormSrgb
                | DxgiFormat::BC2Unorm
                | DxgiFormat::BC2UnormSrgb
                | DxgiFormat::BC3Unorm
                | DxgiFormat::BC3UnormSrgb
                | DxgiFormat::BC4Unorm
                | DxgiFormat::BC4Snorm
                | DxgiFormat::BC5Unorm
                | DxgiFormat::BC5Snorm
                | DxgiFormat::BC6HUf16
                | DxgiFormat::BC6HSf16
                | DxgiFormat::BC7Unorm
                | DxgiFormat::BC7UnormSrgb
        );

        // 2. Write primary DDS stream
        Self::write_single_dds_stream(
            &mut w,
            width,
            height,
            mip_count,
            dxgi_format,
            is_cubemap,
            cry_flags,
            main_needs_dx10,
            main_payload,
        )?;

        // 3. Write Crytek Extended Data chunk (CExt / AttC / CEnd) for attached alpha streams (_ddna)
        if has_attached_alpha && let Some(alpha_bytes) = attached_alpha_payload {
            w.write_all(b"CExt")?; // Marker for Crytek Extended data
            w.write_all(b"AttC")?; // Attached Channel chunk

            // Build full nested BC4 DDS container in memory
            let mut alpha_stream = Vec::new();
            let alpha_flags = cry_flags & (0x1 | 0x4); // Inherit cubemap and decal flags

            // CRITICAL: If the main image is forced to DX10, the attached image MUST also be DX10!
            Self::write_single_dds_stream(
                &mut alpha_stream,
                width,
                height,
                mip_count,
                DxgiFormat::BC4Unorm,
                is_cubemap,
                alpha_flags,
                main_needs_dx10,
                alpha_bytes,
            )?;

            // Write size of attached stream followed by the nested DDS file itself
            w.write_u32::<LittleEndian>(alpha_stream.len() as u32)?;
            w.write_all(&alpha_stream)?;
            w.write_all(b"CEnd")?; // Marker for end of Crytek Extended data
        } else {
            w.write_all(b"CExt")?;
            w.write_all(b"CEnd")?;
        }

        w.flush()?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn write_single_dds_stream<W: Write>(
        w: &mut W,
        width: u32,
        height: u32,
        mip_count: u32,
        dxgi_format: DxgiFormat,
        is_cubemap: bool,
        cry_flags: u32,
        force_dx10: bool,
        payload: &[u8],
    ) -> io::Result<()> {
        let is_uncompressed = matches!(
            dxgi_format,
            DxgiFormat::B8G8R8A8Unorm
                | DxgiFormat::B8G8R8A8UnormSrgb
                | DxgiFormat::B8G8R8X8Unorm
                | DxgiFormat::B8G8R8X8UnormSrgb
                | DxgiFormat::R8G8B8A8Unorm
                | DxgiFormat::R8G8B8A8UnormSrgb
                | DxgiFormat::R8G8Unorm
                | DxgiFormat::R8Unorm
                | DxgiFormat::A8Unorm
        );

        // 1. Magic 'DDS '
        w.write_u32::<LittleEndian>(0x20534444)?;

        // 2. DDS_HEADER (124 bytes)
        w.write_u32::<LittleEndian>(124)?;

        let flags = if !is_uncompressed {
            // DDSD_CAPS | DDSD_HEIGHT | DDSD_WIDTH | DDSD_PIXELFORMAT | DDSD_MIPMAPCOUNT | DDSD_LINEARSIZE
            0x1 | 0x2 | 0x4 | 0x1000 | 0x20000 | 0x80000
        } else {
            // DDSD_CAPS | DDSD_HEIGHT | DDSD_WIDTH | DDSD_PITCH | DDSD_PIXELFORMAT | DDSD_MIPMAPCOUNT
            0x1 | 0x2 | 0x4 | 0x8 | 0x1000 | 0x20000
        };
        w.write_u32::<LittleEndian>(flags)?;
        w.write_u32::<LittleEndian>(height)?;
        w.write_u32::<LittleEndian>(width)?;

        let pitch_or_linear_size = if !is_uncompressed {
            let block_w = width.div_ceil(4).max(1);
            let block_h = height.div_ceil(4).max(1);
            let bytes_per_block = match dxgi_format {
                DxgiFormat::BC1Unorm
                | DxgiFormat::BC1UnormSrgb
                | DxgiFormat::BC4Unorm
                | DxgiFormat::BC4Snorm => 8,
                _ => 16,
            };
            block_w * block_h * bytes_per_block
        } else {
            width * 4 // Row pitch for 32-bit uncompressed
        };
        w.write_u32::<LittleEndian>(pitch_or_linear_size)?;

        w.write_u32::<LittleEndian>(0)?; // Depth
        w.write_u32::<LittleEndian>(mip_count)?;

        // --- CryEngine dwReserved1[11] Overlay (Offset 32..76, 44 bytes) ---
        w.write_u32::<LittleEndian>(0x43525946)?; // dwTextureStage = MAKEFOURCC('F','Y','R','C')
        w.write_u32::<LittleEndian>(cry_flags)?; // dwReserved1 = imageFlags

        // bNumPersistentMips | (bCompressedBlockWidth << 8) | (bCompressedBlockHeight << 16)
        let block_dim: u8 = if is_uncompressed { 1 } else { 4 };
        let persistent_mips = mip_count.min(255);
        let packed_meta: u32 =
            persistent_mips | ((block_dim as u32) << 8) | ((block_dim as u32) << 16);
        w.write_u32::<LittleEndian>(packed_meta)?;

        // cMinColor (Vec4 / ColorF: 0.0, 0.0, 0.0, 0.0)
        w.write_f32::<LittleEndian>(0.0)?;
        w.write_f32::<LittleEndian>(0.0)?;
        w.write_f32::<LittleEndian>(0.0)?;
        w.write_f32::<LittleEndian>(0.0)?;

        // cMaxColor (Vec4 / ColorF: 1.0, 1.0, 1.0, 1.0)
        w.write_f32::<LittleEndian>(1.0)?;
        w.write_f32::<LittleEndian>(1.0)?;
        w.write_f32::<LittleEndian>(1.0)?;
        w.write_f32::<LittleEndian>(1.0)?;

        // 3. DDS_PIXELFORMAT (32 bytes)
        if force_dx10 {
            w.write_u32::<LittleEndian>(32)?;
            w.write_u32::<LittleEndian>(0x4)?; // DDPF_FOURCC
            w.write_all(b"DX10")?;
            w.write_u32::<LittleEndian>(0)?;
            w.write_u32::<LittleEndian>(0)?;
            w.write_u32::<LittleEndian>(0)?;
            w.write_u32::<LittleEndian>(0)?;
            w.write_u32::<LittleEndian>(0)?;
        } else if is_uncompressed {
            let has_alpha = !matches!(
                dxgi_format,
                DxgiFormat::B8G8R8X8Unorm | DxgiFormat::B8G8R8X8UnormSrgb
            );
            w.write_u32::<LittleEndian>(32)?;
            w.write_u32::<LittleEndian>(if has_alpha { 0x41 } else { 0x40 })?; // DDPF_RGB | DDPF_ALPHAPIXELS
            w.write_u32::<LittleEndian>(0)?;
            w.write_u32::<LittleEndian>(32)?;
            w.write_u32::<LittleEndian>(0x00FF_0000)?; // R mask
            w.write_u32::<LittleEndian>(0x0000_FF00)?; // G mask
            w.write_u32::<LittleEndian>(0x0000_00FF)?; // B mask
            w.write_u32::<LittleEndian>(if has_alpha { 0xFF00_0000 } else { 0 })?; // A mask
        } else {
            let fourcc = match dxgi_format {
                DxgiFormat::BC1Unorm | DxgiFormat::BC1UnormSrgb => *b"DXT1",
                DxgiFormat::BC2Unorm | DxgiFormat::BC2UnormSrgb => *b"DXT3",
                DxgiFormat::BC3Unorm | DxgiFormat::BC3UnormSrgb => *b"DXT5",
                DxgiFormat::BC4Unorm => *b"ATI1",
                DxgiFormat::BC5Unorm => *b"ATI2",
                _ => *b"DXT1",
            };
            w.write_u32::<LittleEndian>(32)?;
            w.write_u32::<LittleEndian>(0x4)?; // DDPF_FOURCC
            w.write_all(&fourcc)?;
            w.write_u32::<LittleEndian>(0)?;
            w.write_u32::<LittleEndian>(0)?;
            w.write_u32::<LittleEndian>(0)?;
            w.write_u32::<LittleEndian>(0)?;
            w.write_u32::<LittleEndian>(0)?;
        }

        // Caps
        let mut caps = 0x1000; // DDSCAPS_TEXTURE
        if mip_count > 1 {
            caps |= 0x400000 | 0x8; // DDSCAPS_MIPMAP | DDSCAPS_COMPLEX
        }
        w.write_u32::<LittleEndian>(caps)?;
        w.write_u32::<LittleEndian>(if is_cubemap { 0xFE00 } else { 0 })?; // DDSCAPS2_CUBEMAP | ALL_FACES
        w.write_u32::<LittleEndian>(0)?;
        w.write_u32::<LittleEndian>(0)?;
        w.write_f32::<LittleEndian>(0.0)?; // dwReserved2 (fAvgBrightness)

        // 4. Optional DDS_HEADER_DXT10 (20 bytes)
        if force_dx10 {
            w.write_u32::<LittleEndian>(dxgi_format as u32)?;
            w.write_u32::<LittleEndian>(3)?; // D3D10_RESOURCE_DIMENSION_TEXTURE2D
            w.write_u32::<LittleEndian>(if is_cubemap { 0x4 } else { 0 })?; // D3D10_RESOURCE_MISC_TEXTURECUBE
            w.write_u32::<LittleEndian>(if is_cubemap { 6 } else { 1 })?;
            w.write_u32::<LittleEndian>(0)?;
        }

        // 5. Data payload
        w.write_all(payload)?;
        Ok(())
    }
}
