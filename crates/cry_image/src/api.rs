// Copyright 2001-2026 Crytek GmbH / Crytek Group. All rights reserved.
// Unified Texture Processing & Block Compression API

use crate::compressors::bc1::compress_bc1_block;
use crate::compressors::bc2::{compress_alpha_bc2, decompress_alpha_bc2};
use crate::compressors::bc3::{compress_alpha_bc3, decompress_alpha_bc3};
use crate::compressors::bc4::{compress_bc4, compress_bc4_signed, decompress_bc4};
use crate::compressors::bc5::{
    compress_bc5, compress_bc5_normals, compress_bc5_signed, decompress_bc5,
};
use crate::compressors::bc6h::{compress_bc6h_block, decompress_bc6h_block};
use crate::compressors::bc7::{compress_bc7_block, decompress_bc7_block};
use crate::compressors::ctx1::{compress_ctx1_block, decompress_ctx1_block};
use crate::flags::{ColorMetric, CompressionOptions, Format};
use crate::math::vector::{Vec3, Vec4};
use crate::pipeline::rdo::apply_rdo_optimization;
use rayon::prelude::*;

/// Calculates the exact byte size required to store a compressed texture with the given dimensions.
#[inline(always)]
pub fn get_storage_requirements(width: usize, height: usize, format: Format) -> usize {
    let block_count_x = width.div_ceil(4);
    let block_count_y = height.div_ceil(4);
    block_count_x * block_count_y * format.bytes_per_block()
}

/// Compresses a 4x4 block of RGBA pixels.
pub fn compress_block(
    pixels: &[[u8; 4]; 16],
    mask: u16,
    options: CompressionOptions,
    out_block: &mut [u8],
) {
    match options.format {
        Format::Bc1 => {
            compress_bc1_block(pixels, mask, options, out_block.try_into().unwrap());
        }
        Format::Bc2 => {
            let (a, c) = out_block.split_at_mut(8);
            let mut alphas = [0u8; 16];
            for i in 0..16 {
                alphas[i] = pixels[i][3];
            }
            compress_alpha_bc2(&alphas, mask, a.try_into().unwrap());
            let mut c_opts = options;
            c_opts.format = Format::Bc1;
            c_opts.weight_by_alpha = false;
            compress_bc1_block(pixels, mask, c_opts, c.try_into().unwrap());
        }
        Format::Bc3 => {
            let (a, c) = out_block.split_at_mut(8);
            let mut alphas = [0u8; 16];
            for i in 0..16 {
                alphas[i] = pixels[i][3];
            }
            compress_alpha_bc3(&alphas, mask, 1 << 15, a.try_into().unwrap());
            let mut c_opts = options;
            c_opts.format = Format::Bc1;
            c_opts.weight_by_alpha = false;
            compress_bc1_block(pixels, mask, c_opts, c.try_into().unwrap());
        }
        Format::Bc4 => {
            let blk: &mut [u8; 8] = out_block.try_into().unwrap();
            if options.is_signed {
                let mut r = [0i8; 16];
                for i in 0..16 {
                    r[i] = (pixels[i][0] as i32 - 128).clamp(-127, 127) as i8;
                }
                compress_bc4_signed(&r, mask, 0, blk);
            } else {
                let mut r = [0u8; 16];
                for i in 0..16 {
                    r[i] = pixels[i][0];
                }
                compress_bc4(&r, mask, 1 << 15, blk);
            }
        }
        Format::Bc5 => {
            let (r_blk, g_blk) = out_block.split_at_mut(8);
            if options.is_signed {
                let mut r = [0i8; 16];
                let mut g = [0i8; 16];
                for i in 0..16 {
                    r[i] = (pixels[i][0] as i32 - 128).clamp(-127, 127) as i8;
                    g[i] = (pixels[i][1] as i32 - 128).clamp(-127, 127) as i8;
                }
                compress_bc5_signed(&r, &g, mask, 0, out_block.try_into().unwrap());
            } else {
                let mut r = [0u8; 16];
                let mut g = [0u8; 16];
                for i in 0..16 {
                    r[i] = pixels[i][0];
                    g[i] = pixels[i][1];
                }
                if options.is_normal_map {
                    compress_bc5_normals(
                        &r,
                        &g,
                        mask,
                        1 << 15,
                        r_blk.try_into().unwrap(),
                        g_blk.try_into().unwrap(),
                    );
                } else {
                    compress_bc5(&r, &g, mask, 1 << 15, out_block.try_into().unwrap());
                }
            }
        }
        Format::Bc6h => {
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
                options.is_signed,
                options.strategy,
                out_block.try_into().unwrap(),
            );
        }
        Format::Bc7 => {
            let metric = match options.metric {
                ColorMetric::Perceptual => Vec4::new(0.2126, 0.7152, 0.0722, 1.0),
                ColorMetric::Uniform => Vec4::splat(1.0),
                ColorMetric::Unit => Vec4::new(0.5, 0.5, 0.0, 1.0),
            };
            compress_bc7_block(
                pixels,
                mask,
                0,
                &metric,
                options.quality,
                out_block.try_into().unwrap(),
            );
        }
        Format::Ctx1 => {
            compress_ctx1_block(pixels, mask, 0, out_block.try_into().unwrap());
        }
    }
}

/// Compresses a contiguous 8-bit RGBA image buffer across all available CPU cores in parallel.
pub fn compress_image(
    rgba: &[u8],
    width: usize,
    height: usize,
    options: CompressionOptions,
) -> Vec<u8> {
    let bw = width.div_ceil(4);
    let bh = height.div_ceil(4);
    let bsize = options.format.bytes_per_block();
    let mut out = vec![0u8; bw * bh * bsize];

    out.par_chunks_exact_mut(bw * bsize)
        .enumerate()
        .for_each(|(by, row_slice)| {
            for bx in 0..bw {
                let mut blk = [[0u8; 4]; 16];
                let mut mask = 0u16;

                for py in 0..4 {
                    for px in 0..4 {
                        let x = bx * 4 + px;
                        let y = by * 4 + py;
                        let idx = py * 4 + px;
                        if x < width && y < height {
                            let off = (y * width + x) * 4;
                            blk[idx] = [rgba[off], rgba[off + 1], rgba[off + 2], rgba[off + 3]];
                            mask |= 1 << idx;
                        }
                    }
                }

                let off = bx * bsize;
                compress_block(&blk, mask, options, &mut row_slice[off..off + bsize]);
            }
        });

    if options.rdo_lambda > 0.0 {
        apply_rdo_optimization(&mut out, rgba, width, height, &options);
    }

    out
}

/// Decompresses an 8-bit block-compressed texture into RGBA.
pub fn decompress_image(compressed: &[u8], width: usize, height: usize, format: Format) -> Vec<u8> {
    let bw = width.div_ceil(4);
    let bh = height.div_ceil(4);
    let bsize = format.bytes_per_block();
    let mut out = vec![255u8; width * height * 4];

    for by in 0..bh {
        for bx in 0..bw {
            let off = (by * bw + bx) * bsize;
            let blk = &compressed[off..off + bsize];
            let mut px = [[0u8; 4]; 16];

            match format {
                Format::Bc1 => {
                    crate::compressors::bc1::read_color_block_bc1_rgba(
                        blk.try_into().unwrap(),
                        &mut px,
                    );
                }
                Format::Bc2 => {
                    let (a_blk, c_blk) = blk.split_at(8);
                    let mut rgb = [[0u8; 3]; 16];
                    let mut alpha = [0u8; 16];
                    decompress_alpha_bc2(a_blk.try_into().unwrap(), &mut alpha);
                    crate::compressors::bc1::read_color_block_bc1(
                        c_blk.try_into().unwrap(),
                        &mut rgb,
                    );
                    for i in 0..16 {
                        px[i] = [rgb[i][0], rgb[i][1], rgb[i][2], alpha[i]];
                    }
                }
                Format::Bc3 => {
                    let (a_blk, c_blk) = blk.split_at(8);
                    let mut rgb = [[0u8; 3]; 16];
                    let mut alpha = [0u8; 16];
                    decompress_alpha_bc3(a_blk.try_into().unwrap(), &mut alpha);
                    crate::compressors::bc1::read_color_block_bc1(
                        c_blk.try_into().unwrap(),
                        &mut rgb,
                    );
                    for i in 0..16 {
                        px[i] = [rgb[i][0], rgb[i][1], rgb[i][2], alpha[i]];
                    }
                }
                Format::Bc4 => {
                    let mut r = [0u8; 16];
                    decompress_bc4(blk.try_into().unwrap(), &mut r);
                    for i in 0..16 {
                        px[i] = [r[i], r[i], r[i], 255];
                    }
                }
                Format::Bc5 => {
                    let mut red = [0u8; 16];
                    let mut green = [0u8; 16];
                    decompress_bc5(blk.try_into().unwrap(), &mut red, &mut green);
                    for i in 0..16 {
                        px[i] = [red[i], green[i], 0, 255];
                    }
                }
                Format::Bc6h => {
                    let mut rgb = [[0.0f32; 3]; 16];
                    decompress_bc6h_block(blk.try_into().unwrap(), false, &mut rgb);
                    for i in 0..16 {
                        px[i] = [
                            (rgb[i][0] * 255.0).round().clamp(0.0, 255.0) as u8,
                            (rgb[i][1] * 255.0).round().clamp(0.0, 255.0) as u8,
                            (rgb[i][2] * 255.0).round().clamp(0.0, 255.0) as u8,
                            255,
                        ];
                    }
                }
                Format::Bc7 => {
                    decompress_bc7_block(blk.try_into().unwrap(), &mut px);
                }
                Format::Ctx1 => {
                    decompress_ctx1_block(blk.try_into().unwrap(), &mut px);
                }
            }

            for py in 0..4 {
                for px_idx in 0..4 {
                    let x = bx * 4 + px_idx;
                    let y = by * 4 + py;
                    if x < width && y < height {
                        let dst_off = (y * width + x) * 4;
                        out[dst_off..dst_off + 4].copy_from_slice(&px[py * 4 + px_idx]);
                    }
                }
            }
        }
    }

    out
}

/// Compresses a 32-bit floating-point HDR RGB image into BC6H across all CPU cores in parallel.
pub fn compress_image_hdr(
    rgb: &[[f32; 3]],
    width: usize,
    height: usize,
    options: CompressionOptions,
) -> Vec<u8> {
    let bw = width.div_ceil(4);
    let bh = height.div_ceil(4);
    let mut out = vec![0u8; bw * bh * 16];
    let metric = Vec3::new(1.0, 1.0, 1.0);

    out.par_chunks_exact_mut(bw * 16)
        .enumerate()
        .for_each(|(by, row_slice)| {
            for bx in 0..bw {
                let mut blk = [[0.0f32; 3]; 16];
                let mut mask = 0u16;

                for py in 0..4 {
                    for px in 0..4 {
                        let x = bx * 4 + px;
                        let y = by * 4 + py;
                        let idx = py * 4 + px;
                        if x < width && y < height {
                            blk[idx] = rgb[y * width + x];
                            mask |= 1 << idx;
                        }
                    }
                }

                let off = bx * 16;
                compress_bc6h_block(
                    &blk,
                    mask,
                    &metric,
                    options.is_signed,
                    options.strategy,
                    (&mut row_slice[off..off + 16]).try_into().unwrap(),
                );
            }
        });

    out
}

/// Decompresses BC6H data into 32-bit float RGB.
pub fn decompress_image_hdr(
    compressed: &[u8],
    width: usize,
    height: usize,
    is_signed: bool,
) -> Vec<[f32; 3]> {
    let bw = width.div_ceil(4);
    let bh = height.div_ceil(4);
    let mut out = vec![[0.0f32; 3]; width * height];

    for by in 0..bh {
        for bx in 0..bw {
            let off = (by * bw + bx) * 16;
            let mut px = [[0.0f32; 3]; 16];
            decompress_bc6h_block(
                (&compressed[off..off + 16]).try_into().unwrap(),
                is_signed,
                &mut px,
            );
            for py in 0..4 {
                for px_idx in 0..4 {
                    let x = bx * 4 + px_idx;
                    let y = by * 4 + py;
                    if x < width && y < height {
                        out[y * width + x] = px[py * 4 + px_idx];
                    }
                }
            }
        }
    }

    out
}
