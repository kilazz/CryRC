// Copyright 2016-2026 Crytek GmbH / Crytek Group. All rights reserved.

use byteorder::{ByteOrder, LittleEndian};
use rayon::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AstcBlockDim {
    pub x: usize,
    pub y: usize,
}

impl AstcBlockDim {
    pub fn parse(s: &str) -> Self {
        let parts: Vec<&str> = s.split('x').collect();
        if parts.len() == 2 {
            let x = parts[0].parse().unwrap_or(4).clamp(4, 12);
            let y = parts[1].parse().unwrap_or(4).clamp(4, 12);
            Self { x, y }
        } else {
            Self { x: 4, y: 4 }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AstcPixelFormat {
    LdrL,
    LdrA,
    LdrLA,
    LdrRG,
    LdrNormal,
    LdrRGB,
    LdrRGBA,
    HdrL,
    HdrRGB,
    HdrRGBA,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AstcQuality {
    VeryFast,
    Fast,
    Medium,
    Thorough,
    Exhaustive,
}

pub struct AstcCompressor;

impl AstcCompressor {
    pub fn compress(
        src_rgba: &[u8],
        width: usize,
        height: usize,
        block_dim: AstcBlockDim,
        format: AstcPixelFormat,
        _quality: AstcQuality,
    ) -> Vec<u8> {
        let x_blocks = width.div_ceil(block_dim.x);
        let y_blocks = height.div_ceil(block_dim.y);
        let block_bytes = 16;
        let total_size = x_blocks * y_blocks * block_bytes;
        let mut output = vec![0u8; total_size];

        output
            .par_chunks_exact_mut(x_blocks * block_bytes)
            .enumerate()
            .for_each(|(by, row_slice)| {
                let y_start = by * block_dim.y;
                for bx in 0..x_blocks {
                    let x_start = bx * block_dim.x;
                    let dst_block = &mut row_slice[bx * block_bytes..(bx + 1) * block_bytes];

                    let mut block_pixels = Vec::with_capacity(block_dim.x * block_dim.y * 4);
                    for y in 0..block_dim.y {
                        let cur_y = (y_start + y).min(height - 1);
                        for x in 0..block_dim.x {
                            let cur_x = (x_start + x).min(width - 1);
                            let off = (cur_y * width + cur_x) * 4;
                            block_pixels.extend_from_slice(&src_rgba[off..off + 4]);
                        }
                    }

                    Self::encode_astc_block(&block_pixels, block_dim, format, dst_block);
                }
            });

        output
    }

    fn encode_astc_block(
        block_pixels: &[u8],
        dim: AstcBlockDim,
        format: AstcPixelFormat,
        dst: &mut [u8],
    ) {
        let (mut min_r, mut min_g, mut min_b, mut min_a) = (255u8, 255u8, 255u8, 255u8);
        let (mut max_r, mut max_g, mut max_b, mut max_a) = (0u8, 0u8, 0u8, 0u8);

        for px in block_pixels.chunks_exact(4) {
            min_r = min_r.min(px[0]);
            max_r = max_r.max(px[0]);
            min_g = min_g.min(px[1]);
            max_g = max_g.max(px[1]);
            min_b = min_b.min(px[2]);
            max_b = max_b.max(px[2]);
            min_a = min_a.min(px[3]);
            max_a = max_a.max(px[3]);
        }

        let (cem, ep0, ep1, ep_bytes) = match format {
            AstcPixelFormat::LdrL | AstcPixelFormat::LdrA => {
                let (e0, e1) = if format == AstcPixelFormat::LdrA {
                    (min_a, max_a)
                } else {
                    (min_r, max_r)
                };
                (0u64, [e0, 0, 0, 0], [e1, 0, 0, 0], 2)
            }
            AstcPixelFormat::LdrLA => (4u64, [min_r, min_a, 0, 0], [max_r, max_a, 0, 0], 4),
            AstcPixelFormat::LdrRGB => {
                (8u64, [min_r, min_g, min_b, 0], [max_r, max_g, max_b, 0], 6)
            }
            _ => (
                12u64,
                [min_r, min_g, min_b, min_a],
                [max_r, max_g, max_b, max_a],
                8,
            ),
        };

        let span_r = (max_r as i32 - min_r as i32).max(1);
        let span_g = (max_g as i32 - min_g as i32).max(1);
        let span_b = (max_b as i32 - min_b as i32).max(1);
        let span_a = (max_a as i32 - min_a as i32).max(1);

        let mut weights = Vec::with_capacity(16);
        let step_x = (dim.x as f32) / 4.0;
        let step_y = (dim.y as f32) / 4.0;

        for wy in 0..4 {
            let sy = ((wy as f32 * step_y).floor() as usize).min(dim.y - 1);
            for wx in 0..4 {
                let sx = ((wx as f32 * step_x).floor() as usize).min(dim.x - 1);
                let p_off = (sy * dim.x + sx) * 4;

                let wr = (block_pixels[p_off] as i32 - min_r as i32) * 15 / span_r;
                let wg = (block_pixels[p_off + 1] as i32 - min_g as i32) * 15 / span_g;
                let wb = (block_pixels[p_off + 2] as i32 - min_b as i32) * 15 / span_b;
                let wa = (block_pixels[p_off + 3] as i32 - min_a as i32) * 15 / span_a;

                let avg_w = match format {
                    AstcPixelFormat::LdrL => wr.clamp(0, 15) as u8,
                    AstcPixelFormat::LdrA => wa.clamp(0, 15) as u8,
                    AstcPixelFormat::LdrLA => ((wr + wa) / 2).clamp(0, 15) as u8,
                    _ => ((wr + wg + wb + wa + 2) / 4).clamp(0, 15) as u8,
                };
                weights.push(avg_w);
            }
        }

        let mut low: u64 = 0;
        let mut high: u64 = 0;

        low |= 0x01 & 0x7FF; // 4x4 grid
        low |= (cem & 0x0F) << 13;

        let mut ep_bits: u64 = 0;
        for i in 0..ep_bytes / 2 {
            ep_bits |= (ep0[i] as u64) << (i * 8);
            ep_bits |= (ep1[i] as u64) << ((ep_bytes / 2 + i) * 8);
        }

        low |= ep_bits << 17;
        high |= ep_bits >> (64 - 17);

        let mut weight_bits: u64 = 0;
        for (i, &w) in weights.iter().enumerate() {
            weight_bits |= (w as u64 & 0x0F) << (i * 4);
        }

        high |= weight_bits << 16;

        LittleEndian::write_u64(&mut dst[0..8], low);
        LittleEndian::write_u64(&mut dst[8..16], high);
    }
}
