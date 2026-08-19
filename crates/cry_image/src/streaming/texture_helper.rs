pub struct TextureHelper;

impl TextureHelper {
    pub fn is_block_compressed(fourcc: &[u8; 4], dxgi_format: u32) -> bool {
        if fourcc == b"DX10" {
            // BC1..BC7 DXGI format ranges: 70..84, 94..99
            matches!(dxgi_format, 70..=84 | 94..=99)
        } else {
            matches!(
                fourcc,
                b"DXT1"
                    | b"DXT2"
                    | b"DXT3"
                    | b"DXT4"
                    | b"DXT5"
                    | b"ATI1"
                    | b"ATI2"
                    | b"BC4U"
                    | b"BC4S"
                    | b"BC5U"
                    | b"BC5S"
                    | b"CTX1"
            )
        }
    }

    pub fn bytes_per_block(fourcc: &[u8; 4], dxgi_format: u32) -> usize {
        if fourcc == b"DX10" {
            match dxgi_format {
                70 | 71 | 72 | 80 | 81 => 8, // BC1 (70..72) and BC4 (80..81) are 8 bytes/block
                _ => 16,                     // BC2, BC3, BC5, BC6H, BC7 are 16 bytes/block
            }
        } else {
            match fourcc {
                b"DXT1" | b"ATI1" | b"BC4U" | b"BC4S" | b"CTX1" => 8,
                _ => 16,
            }
        }
    }

    pub fn bytes_per_pixel(fourcc: &[u8; 4], dxgi_format: u32) -> usize {
        if fourcc == b"DX10" {
            match dxgi_format {
                1..=9 => 16,  // RGBA32F
                10..=14 => 8, // RGBA16F / RGBA16Unorm
                33..=35 => 4, // RG16F
                53..=56 => 2, // R16F
                60..=65 => 1, // R8 / A8
                _ => 4,       // Standard 32-bit RGBA8 / BGRA8
            }
        } else {
            match fourcc {
                b"A16B" => 8,
                b"A32B" => 16,
                _ => 4,
            }
        }
    }

    pub fn mip_data_size(
        width: usize,
        height: usize,
        depth: usize,
        fourcc: &[u8; 4],
        dxgi_format: u32,
    ) -> usize {
        let d = depth.max(1);
        if Self::is_block_compressed(fourcc, dxgi_format) {
            let blocks_x = width.div_ceil(4);
            let blocks_y = height.div_ceil(4);
            blocks_x * blocks_y * d * Self::bytes_per_block(fourcc, dxgi_format)
        } else {
            width.max(1) * height.max(1) * d * Self::bytes_per_pixel(fourcc, dxgi_format)
        }
    }

    pub fn texture_data_size(
        mut width: usize,
        mut height: usize,
        mut depth: usize,
        mip_count: usize,
        fourcc: &[u8; 4],
        dxgi_format: u32,
    ) -> usize {
        let mut total_size = 0;
        let mut cur_mips = 0;

        while width > 0 || height > 0 || depth > 0 {
            let w = width.max(1);
            let h = height.max(1);
            let d = depth.max(1);

            total_size += Self::mip_data_size(w, h, d, fourcc, dxgi_format);

            width >>= 1;
            height >>= 1;
            depth >>= 1;

            cur_mips += 1;
            if cur_mips == mip_count {
                break;
            }
        }
        total_size
    }
}
