// Copyright 2013-2026 Crytek GmbH / Crytek Group. All rights reserved.

use rayon::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PvrPixelFormat {
    PVRTC2,
    PVRTC4,
    ETC2,
    ETC2a,
    EacR11,
    EacRg11,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PvrQuality {
    Fastest,
    Fast,
    Normal,
    High,
}

pub struct PvrtcCompressor;

impl PvrtcCompressor {
    pub fn compress(
        src_bgra: &[u8],
        width: usize,
        height: usize,
        format: PvrPixelFormat,
        _quality: PvrQuality,
        _srgb: bool,
    ) -> Vec<u8> {
        match format {
            PvrPixelFormat::ETC2 => Self::compress_etc2_rgb(src_bgra, width, height),
            PvrPixelFormat::ETC2a => Self::compress_etc2_rgba(src_bgra, width, height),
            PvrPixelFormat::EacR11 => Self::compress_eac_r11(src_bgra, width, height),
            PvrPixelFormat::EacRg11 => Self::compress_eac_rg11(src_bgra, width, height),
            PvrPixelFormat::PVRTC4 => Self::compress_pvrtc_4bpp(src_bgra, width, height),
            PvrPixelFormat::PVRTC2 => Self::compress_pvrtc_2bpp(src_bgra, width, height),
        }
    }

    fn compress_pvrtc_4bpp(src_bgra: &[u8], width: usize, height: usize) -> Vec<u8> {
        let blocks_x = (width / 4).max(2);
        let blocks_y = (height / 4).max(2);
        let total_blocks = blocks_x * blocks_y;
        let mut output = vec![0u8; total_blocks * 8];

        let mut block_colors_min = vec![[255u8; 4]; total_blocks];
        let mut block_colors_max = vec![[0u8; 4]; total_blocks];

        for by in 0..blocks_y {
            for bx in 0..blocks_x {
                let block_idx = Self::morton_2d(bx, by);
                if block_idx >= total_blocks {
                    continue;
                }

                let mut min_c = [255u8; 4];
                let mut max_c = [0u8; 4];

                for y in 0..4 {
                    let py = ((by * 4 + y).min(height - 1)) * width;
                    for x in 0..4 {
                        let px = (py + (bx * 4 + x).min(width - 1)) * 4;
                        for c in 0..4 {
                            min_c[c] = min_c[c].min(src_bgra[px + c]);
                            max_c[c] = max_c[c].max(src_bgra[px + c]);
                        }
                    }
                }
                block_colors_min[block_idx] = min_c;
                block_colors_max[block_idx] = max_c;
            }
        }

        output
            .par_chunks_exact_mut(8)
            .enumerate()
            .for_each(|(b_idx, dst_word)| {
                let (bx, by) = Self::unmorton_2d(b_idx);
                if bx >= blocks_x || by >= blocks_y {
                    return;
                }

                let col_a = block_colors_min[b_idx];
                let col_b = block_colors_max[b_idx];

                let a_val = ((col_a[2] as u16 >> 3) << 10)
                    | ((col_a[1] as u16 >> 3) << 5)
                    | (col_a[0] as u16 >> 3)
                    | if col_a[3] > 127 { 0x8000 } else { 0 };

                let b_val = ((col_b[2] as u16 >> 3) << 10)
                    | ((col_b[1] as u16 >> 3) << 5)
                    | (col_b[0] as u16 >> 3)
                    | if col_b[3] > 127 { 0x8000 } else { 0 };

                let mut modulation: u32 = 0;
                for y in 0..4 {
                    let py = ((by * 4 + y).min(height - 1)) * width;
                    for x in 0..4 {
                        let px = (py + (bx * 4 + x).min(width - 1)) * 4;
                        let lum = (src_bgra[px] as u32
                            + src_bgra[px + 1] as u32 * 2
                            + src_bgra[px + 2] as u32)
                            / 4;
                        let lum_a = (col_a[0] as u32 + col_a[1] as u32 * 2 + col_a[2] as u32) / 4;
                        let lum_b = (col_b[0] as u32 + col_b[1] as u32 * 2 + col_b[2] as u32) / 4;

                        let span = (lum_b as i32 - lum_a as i32).abs().max(1);
                        let mod_val =
                            (((lum as i32 - lum_a as i32).abs() * 3) / span).clamp(0, 3) as u32;
                        let bit_pos = (y * 4 + x) * 2;
                        modulation |= (mod_val & 3) << bit_pos;
                    }
                }

                dst_word[0..4].copy_from_slice(&modulation.to_le_bytes());
                dst_word[4..6].copy_from_slice(&a_val.to_le_bytes());
                dst_word[6..8].copy_from_slice(&b_val.to_le_bytes());
            });

        output
    }

    fn compress_pvrtc_2bpp(src_bgra: &[u8], width: usize, height: usize) -> Vec<u8> {
        let blocks_x = (width / 8).max(2);
        let blocks_y = (height / 4).max(2);
        let total_blocks = blocks_x * blocks_y;
        let mut output = vec![0u8; total_blocks * 8];

        output
            .par_chunks_exact_mut(8)
            .enumerate()
            .for_each(|(b_idx, dst_word)| {
                let bx = b_idx % blocks_x;
                let by = b_idx / blocks_x;

                let mut min_c = [255u8; 4];
                let mut max_c = [0u8; 4];

                for y in 0..4 {
                    let py = ((by * 4 + y).min(height - 1)) * width;
                    for x in 0..8 {
                        let px = (py + (bx * 8 + x).min(width - 1)) * 4;
                        for c in 0..4 {
                            min_c[c] = min_c[c].min(src_bgra[px + c]);
                            max_c[c] = max_c[c].max(src_bgra[px + c]);
                        }
                    }
                }

                let a_val = ((min_c[2] as u16 >> 3) << 10)
                    | ((min_c[1] as u16 >> 3) << 5)
                    | (min_c[0] as u16 >> 3)
                    | 0x8000;
                let b_val = ((max_c[2] as u16 >> 3) << 10)
                    | ((max_c[1] as u16 >> 3) << 5)
                    | (max_c[0] as u16 >> 3)
                    | 0x8000;

                let mut modulation: u32 = 0;
                let lum_a = (min_c[0] as u32 + min_c[1] as u32 * 2 + min_c[2] as u32) / 4;
                let lum_b = (max_c[0] as u32 + max_c[1] as u32 * 2 + max_c[2] as u32) / 4;
                let mid = (lum_a + lum_b) / 2;

                for y in 0..4 {
                    let py = ((by * 4 + y).min(height - 1)) * width;
                    for x in 0..8 {
                        let px = (py + (bx * 8 + x).min(width - 1)) * 4;
                        let lum = (src_bgra[px] as u32
                            + src_bgra[px + 1] as u32 * 2
                            + src_bgra[px + 2] as u32)
                            / 4;
                        if lum >= mid {
                            modulation |= 1 << (y * 8 + x);
                        }
                    }
                }

                dst_word[0..4].copy_from_slice(&modulation.to_le_bytes());
                dst_word[4..6].copy_from_slice(&a_val.to_le_bytes());
                dst_word[6..8].copy_from_slice(&b_val.to_le_bytes());
            });

        output
    }

    #[inline]
    fn morton_2d(mut x: usize, mut y: usize) -> usize {
        x = (x | (x << 8)) & 0x00FF00FF;
        x = (x | (x << 4)) & 0x0F0F0F0F;
        x = (x | (x << 2)) & 0x33333333;
        x = (x | (x << 1)) & 0x55555555;
        y = (y | (y << 8)) & 0x00FF00FF;
        y = (y | (y << 4)) & 0x0F0F0F0F;
        y = (y | (y << 2)) & 0x33333333;
        y = (y | (y << 1)) & 0x55555555;
        x | (y << 1)
    }

    #[inline]
    fn unmorton_2d(code: usize) -> (usize, usize) {
        let mut x = code & 0x55555555;
        let mut y = (code >> 1) & 0x55555555;
        x = (x | (x >> 1)) & 0x33333333;
        x = (x | (x >> 2)) & 0x0F0F0F0F;
        x = (x | (x >> 4)) & 0x00FF00FF;
        x = (x | (x >> 8)) & 0x0000FFFF;
        y = (y | (y >> 1)) & 0x33333333;
        y = (y | (y >> 2)) & 0x0F0F0F0F;
        y = (y | (y >> 4)) & 0x00FF00FF;
        y = (y | (y >> 8)) & 0x0000FFFF;
        (x, y)
    }

    fn compress_etc2_rgb(src_bgra: &[u8], width: usize, height: usize) -> Vec<u8> {
        let bw = width.div_ceil(4);
        let bh = height.div_ceil(4);
        let mut output = vec![0u8; bw * bh * 8];

        output
            .par_chunks_exact_mut(bw * 8)
            .enumerate()
            .for_each(|(by, row)| {
                for bx in 0..bw {
                    let dst = &mut row[bx * 8..(bx + 1) * 8];
                    let off = ((by * 4).min(height - 1) * width + (bx * 4).min(width - 1)) * 4;
                    dst[0] = src_bgra[off + 2]; // R
                    dst[1] = src_bgra[off + 1]; // G
                    dst[2] = src_bgra[off]; // B
                    dst[3] = 0x00;
                    dst[4] = 0x20;
                }
            });
        output
    }

    fn compress_etc2_rgba(src_bgra: &[u8], width: usize, height: usize) -> Vec<u8> {
        let bw = width.div_ceil(4);
        let bh = height.div_ceil(4);
        let mut output = vec![0u8; bw * bh * 16];

        output
            .par_chunks_exact_mut(bw * 16)
            .enumerate()
            .for_each(|(by, row)| {
                for bx in 0..bw {
                    let dst = &mut row[bx * 16..(bx + 1) * 16];
                    let off = ((by * 4).min(height - 1) * width + (bx * 4).min(width - 1)) * 4;
                    dst[0] = src_bgra[off + 3]; // Alpha
                    dst[8] = src_bgra[off + 2]; // R
                    dst[9] = src_bgra[off + 1]; // G
                    dst[10] = src_bgra[off]; // B
                }
            });
        output
    }

    fn compress_eac_r11(src_bgra: &[u8], width: usize, height: usize) -> Vec<u8> {
        let bw = width.div_ceil(4);
        let bh = height.div_ceil(4);
        let mut output = vec![0u8; bw * bh * 8];

        output
            .par_chunks_exact_mut(bw * 8)
            .enumerate()
            .for_each(|(by, row)| {
                for bx in 0..bw {
                    let dst = &mut row[bx * 8..(bx + 1) * 8];
                    let off = ((by * 4).min(height - 1) * width + (bx * 4).min(width - 1)) * 4;
                    dst[0] = src_bgra[off + 2];
                }
            });
        output
    }

    fn compress_eac_rg11(src_bgra: &[u8], width: usize, height: usize) -> Vec<u8> {
        let bw = width.div_ceil(4);
        let bh = height.div_ceil(4);
        let mut output = vec![0u8; bw * bh * 16];

        output
            .par_chunks_exact_mut(bw * 16)
            .enumerate()
            .for_each(|(by, row)| {
                for bx in 0..bw {
                    let dst = &mut row[bx * 16..(bx + 1) * 16];
                    let off = ((by * 4).min(height - 1) * width + (bx * 4).min(width - 1)) * 4;
                    dst[0] = src_bgra[off + 2]; // X Normal
                    dst[8] = src_bgra[off + 1]; // Y Normal
                }
            });
        output
    }
}
