// Copyright 2001-2026 Crytek GmbH / Crytek Group. All rights reserved.
// Rate-Distortion Optimization with Format Protection & Robust MSE Bounds

use crate::compressors::bc1::read_color_block_bc1;
use crate::compressors::bc3::decompress_alpha_bc3;
use crate::compressors::bc4::decompress_bc4;
use crate::compressors::ctx1::decompress_ctx1_block;
use crate::flags::{CompressionOptions, Format};
use crate::pipeline::metrics::TrackedStat;
use crate::pipeline::ultrasmooth::compute_block_mse_scales;

pub const MIN_MATCH_LEN: usize = 3;
pub const LITERAL_BITS: f32 = 13.0;
pub const MATCH_CONTINUE_BITS: f32 = 1.0;
pub const MAX_BLOCK_SIZE_IN_BYTES: usize = 16;

const fn generate_small_dist_extra() -> [u8; 512] {
    let mut table = [0u8; 512];
    let mut i = 0;
    while i < 4 {
        table[i] = 0;
        i += 1;
    }
    while i < 8 {
        table[i] = 1;
        i += 1;
    }
    while i < 16 {
        table[i] = 2;
        i += 1;
    }
    while i < 32 {
        table[i] = 3;
        i += 1;
    }
    while i < 64 {
        table[i] = 4;
        i += 1;
    }
    while i < 128 {
        table[i] = 5;
        i += 1;
    }
    while i < 256 {
        table[i] = 6;
        i += 1;
    }
    while i < 512 {
        table[i] = 7;
        i += 1;
    }
    table
}

const fn generate_large_dist_extra() -> [u8; 128] {
    let mut table = [0u8; 128];
    let mut i = 0;
    while i < 2 {
        table[i] = 0;
        i += 1;
    }
    while i < 4 {
        table[i] = 8;
        i += 1;
    }
    while i < 8 {
        table[i] = 9;
        i += 1;
    }
    while i < 16 {
        table[i] = 10;
        i += 1;
    }
    while i < 32 {
        table[i] = 11;
        i += 1;
    }
    while i < 64 {
        table[i] = 12;
        i += 1;
    }
    while i < 128 {
        table[i] = 13;
        i += 1;
    }
    table
}

pub static TDEFL_SMALL_DIST_EXTRA: [u8; 512] = generate_small_dist_extra();
pub static TDEFL_LARGE_DIST_EXTRA: [u8; 128] = generate_large_dist_extra();

#[inline(always)]
pub fn compute_match_cost_estimate(dist: u32, match_len_in_bytes: u32) -> u32 {
    let len_cost = match match_len_in_bytes {
        l if l >= 12 => 9,
        l if l >= 8 => 8,
        l if l >= 6 => 7,
        _ => 6,
    };

    let mut dist_cost = 5;
    if dist < 512 {
        dist_cost += TDEFL_SMALL_DIST_EXTRA[(dist & 511) as usize] as u32;
    } else {
        dist_cost += TDEFL_LARGE_DIST_EXTRA[(dist.min(32767) >> 8) as usize] as u32;
        let mut d = dist;
        while d >= 32768 {
            dist_cost += 1;
            d >>= 1;
        }
    }
    len_cost + dist_cost
}

pub fn hash_hsieh(buf: &[u8], salt: u32) -> u32 {
    let len = buf.len();
    if len == 0 {
        return 0;
    }

    let mut h = (len as u32).wrapping_add(salt << 16);
    let bytes_left = len & 3;
    let words_len = len >> 2;

    for i in 0..words_len {
        let chunk = &buf[i * 4..(i + 1) * 4];
        let w0 = u16::from_le_bytes([chunk[0], chunk[1]]) as u32;
        let w1 = u16::from_le_bytes([chunk[2], chunk[3]]) as u32;

        h = h.wrapping_add(w0);
        let t = (w1 << 11) ^ h;
        h = (h << 16) ^ t;
        h = h.wrapping_add(h >> 11);
    }

    let offset = words_len * 4;
    match bytes_left {
        1 => {
            let b0 = buf[offset] as i8 as i32 as u32;
            h = h.wrapping_add(b0);
            h ^= h << 10;
            h = h.wrapping_add(h >> 1);
        }
        2 => {
            let w = u16::from_le_bytes([buf[offset], buf[offset + 1]]) as u32;
            h = h.wrapping_add(w);
            h ^= h << 11;
            h = h.wrapping_add(h >> 17);
        }
        3 => {
            let w = u16::from_le_bytes([buf[offset], buf[offset + 1]]) as u32;
            h = h.wrapping_add(w);
            h ^= h << 16;
            h ^= (buf[offset + 2] as i8 as i32 as u32) << 18;
            h = h.wrapping_add(h >> 11);
        }
        _ => {}
    }

    h ^= h << 3;
    h = h.wrapping_add(h >> 5);
    h ^= h << 4;
    h = h.wrapping_add(h >> 17);
    h ^= h << 25;
    h = h.wrapping_add(h >> 6);

    h
}

#[inline(always)]
pub fn compute_block_mse(
    orig: &[[u8; 4]; 16],
    decoded: &[[u8; 4]; 16],
    weights: [u32; 4],
    inv_total_weight: f32,
) -> f32 {
    let mut total_err: u64 = 0;
    for i in 0..16 {
        let dr = orig[i][0] as i32 - decoded[i][0] as i32;
        let dg = orig[i][1] as i32 - decoded[i][1] as i32;
        let db = orig[i][2] as i32 - decoded[i][2] as i32;
        let da = orig[i][3] as i32 - decoded[i][3] as i32;

        total_err += (weights[0] as i32 * dr * dr
            + weights[1] as i32 * dg * dg
            + weights[2] as i32 * db * db
            + weights[3] as i32 * da * da) as u64;
    }
    (total_err as f32) * (inv_total_weight / 16.0)
}

#[inline(always)]
fn compute_block_max_std_dev(pixels: &[[u8; 4]; 16]) -> f32 {
    let mut stats = [TrackedStat::new(); 4];
    for p in pixels {
        for c in 0..4 {
            stats[c].update(p[c] as f64);
        }
    }
    let mut max_sd = 0.0f32;
    for stat in stats.iter().take(3) {
        max_sd = max_sd.max(stat.std_dev() as f32);
    }
    max_sd
}

#[derive(Clone, Debug)]
pub struct ReduceEntropyParams {
    pub lambda: f32,
    pub lookback_window_size: usize,
    pub max_allowed_rms_increase_ratio: f32,
    pub max_smooth_block_std_dev: f32,
    pub smooth_block_max_mse_scale: f32,
    pub color_weights: [u32; 4],
    pub try_two_matches: bool,
}

impl Default for ReduceEntropyParams {
    fn default() -> Self {
        Self {
            lambda: 1.0,
            lookback_window_size: 256,
            max_allowed_rms_increase_ratio: 1.5,
            max_smooth_block_std_dev: 18.0,
            smooth_block_max_mse_scale: 10.0,
            color_weights: [1, 1, 1, 1],
            try_two_matches: true,
        }
    }
}

pub fn reduce_entropy<F>(
    blocks: &mut [u8],
    block_size: usize,
    orig_pixels: &[[[u8; 4]; 16]],
    params: &ReduceEntropyParams,
    block_mse_scales: Option<&[f32]>,
    mut unpack_fn: F,
) -> u32
where
    F: FnMut(&[u8], &mut [[u8; 4]; 16]) -> bool,
{
    let total_color_weight: u32 = params.color_weights.iter().sum();
    let inv_total_weight = 1.0 / total_color_weight as f32;

    let num_blocks = blocks.len() / block_size;
    let total_blocks_to_check = (params.lookback_window_size / block_size).max(1);

    let mut prev_match_window_ofs_to_favor_cont: isize = -1;
    let mut total_modified = 0u32;

    const HASH_SIZE: usize = 8192;
    let mut hash = [0u32; HASH_SIZE];

    for block_index in 0..num_blocks {
        if (block_index & 0xFF) == 0 {
            hash.fill(0);
        }

        let orig_block_ofs = block_index * block_size;
        let mut decoded_block = [[0u8; 4]; 16];

        if !unpack_fn(
            &blocks[orig_block_ofs..orig_block_ofs + block_size],
            &mut decoded_block,
        ) {
            continue;
        }

        let cur_mse = compute_block_mse(
            &orig_pixels[block_index],
            &decoded_block,
            params.color_weights,
            inv_total_weight,
        );

        if cur_mse == 0.0 {
            continue;
        }

        let max_std_dev = compute_block_max_std_dev(&orig_pixels[block_index]);
        let mut yl = (max_std_dev / params.max_smooth_block_std_dev).clamp(0.0, 1.0);
        yl *= yl;
        let mut smooth_block_mse_scale =
            params.smooth_block_max_mse_scale + (1.0 - params.smooth_block_max_mse_scale) * yl;

        if let Some(scale) = block_mse_scales
            .map(|s| s[block_index])
            .filter(|&s| s > 0.0)
        {
            smooth_block_mse_scale = scale;
        }

        let cur_bits = LITERAL_BITS * block_size as f32;
        let cur_t = cur_mse * smooth_block_mse_scale + cur_bits * params.lambda;

        let first_block_to_check = block_index.saturating_sub(total_blocks_to_check);
        let last_block_to_check = block_index.saturating_sub(1);

        let mut best_block = [0u8; MAX_BLOCK_SIZE_IN_BYTES];
        best_block[..block_size]
            .copy_from_slice(&blocks[orig_block_ofs..orig_block_ofs + block_size]);

        let mut best_t = cur_t;
        let mut best_match_len = 0usize;
        let mut best_match_src_window_ofs = 0usize;
        let mut best_match_dst_block_ofs = 0usize;

        let thresh_ms_err = (cur_mse * 1.3).min(cur_mse + 2.0);

        if block_index > 0 {
            for prev_block_index in (first_block_to_check..=last_block_to_check).rev() {
                let prev_block_ofs = prev_block_index * block_size;
                let match_dist = ((block_index - prev_block_index) * block_size) as u32;

                for len in (MIN_MATCH_LEN..=block_size).rev() {
                    let trial_match_bits =
                        compute_match_cost_estimate(match_dist, len as u32) as f32;
                    let trial_total_bits =
                        (block_size - len) as f32 * LITERAL_BITS + trial_match_bits;

                    for ofs in 0..=(block_size - len) {
                        let src_match_window_ofs = prev_block_index * block_size + ofs;
                        let mut trial_total_bits_to_use = trial_total_bits;

                        let hs = hash_hsieh(
                            &blocks[prev_block_ofs + ofs..prev_block_ofs + ofs + len],
                            ofs as u32,
                        );

                        if src_match_window_ofs as isize == prev_match_window_ofs_to_favor_cont
                            && ofs == 0
                        {
                            trial_total_bits_to_use =
                                (block_size - len) as f32 * LITERAL_BITS + MATCH_CONTINUE_BITS;
                        } else {
                            let hash_check = hash[(hs as usize) & (HASH_SIZE - 1)];
                            if (hash_check & 0xFF) == ((block_index as u32) & 0xFF)
                                && (hash_check >> 8) == (hs >> 8)
                            {
                                continue;
                            }
                        }

                        hash[(hs as usize) & (HASH_SIZE - 1)] =
                            (hs & 0xFFFFFF00) | ((block_index as u32) & 0xFF);

                        let mut trial_block = [0u8; MAX_BLOCK_SIZE_IN_BYTES];
                        trial_block[..block_size]
                            .copy_from_slice(&blocks[orig_block_ofs..orig_block_ofs + block_size]);
                        trial_block[ofs..ofs + len].copy_from_slice(
                            &blocks[prev_block_ofs + ofs..prev_block_ofs + ofs + len],
                        );

                        let mut decoded_trial_block = [[0u8; 4]; 16];
                        if !unpack_fn(&trial_block[..block_size], &mut decoded_trial_block) {
                            continue;
                        }

                        let trial_mse = compute_block_mse(
                            &orig_pixels[block_index],
                            &decoded_trial_block,
                            params.color_weights,
                            inv_total_weight,
                        );

                        if trial_mse < thresh_ms_err {
                            let t = trial_mse * smooth_block_mse_scale
                                + trial_total_bits_to_use * params.lambda;
                            if t < best_t {
                                best_t = t;
                                best_block[..block_size]
                                    .copy_from_slice(&trial_block[..block_size]);
                                best_match_len = len;
                                best_match_src_window_ofs = src_match_window_ofs;
                                best_match_dst_block_ofs = ofs;
                            }
                        }
                    }
                }
            }
        }

        if best_t < cur_t {
            blocks[orig_block_ofs..orig_block_ofs + block_size]
                .copy_from_slice(&best_block[..block_size]);
            total_modified += 1;

            if (best_match_dst_block_ofs + best_match_len) == block_size {
                prev_match_window_ofs_to_favor_cont =
                    (best_match_src_window_ofs + best_match_len) as isize;
            } else {
                prev_match_window_ofs_to_favor_cont = -1;
            }
        } else {
            prev_match_window_ofs_to_favor_cont = -1;
        }
    }

    total_modified
}

#[allow(clippy::too_many_arguments)]
pub fn reduce_entropy_strided<F>(
    data: &mut [u8],
    sub_offset: usize,
    stride: usize,
    sub_size: usize,
    orig_pixels: &[[[u8; 4]; 16]],
    params: &ReduceEntropyParams,
    scales: Option<&[f32]>,
    mut unpack_fn: F,
) where
    F: FnMut(&[u8], &mut [[u8; 4]; 16]) -> bool,
{
    let num_blocks = data.len() / stride;
    let lookback_blocks = (params.lookback_window_size / stride).max(1);

    let total_color_weight: u32 = params.color_weights.iter().sum();
    let inv_total_weight = 1.0 / total_color_weight as f32;

    for block_idx in 0..num_blocks {
        let cur_offset = block_idx * stride + sub_offset;
        let mut decoded_block = [[0u8; 4]; 16];

        if !unpack_fn(&data[cur_offset..cur_offset + sub_size], &mut decoded_block) {
            continue;
        }

        let cur_mse = compute_block_mse(
            &orig_pixels[block_idx],
            &decoded_block,
            params.color_weights,
            inv_total_weight,
        );
        if cur_mse == 0.0 {
            continue;
        }

        let smooth_scale = scales.map_or(1.0, |s| {
            if s[block_idx] > 0.0 {
                s[block_idx]
            } else {
                1.0
            }
        });
        let cur_bits = LITERAL_BITS * sub_size as f32;
        let mut best_t = cur_mse * smooth_scale + cur_bits * params.lambda;

        let mut best_block = [0u8; 16];
        best_block[..sub_size].copy_from_slice(&data[cur_offset..cur_offset + sub_size]);

        let start_lookback = block_idx.saturating_sub(lookback_blocks);
        let thresh_mse = (cur_mse * 1.3).min(cur_mse + 2.0);

        for prev_idx in (start_lookback..block_idx).rev() {
            let prev_offset = prev_idx * stride + sub_offset;
            let dist = ((block_idx - prev_idx) * stride) as u32;

            for len in (3..=sub_size).rev() {
                let match_bits = compute_match_cost_estimate(dist, len as u32) as f32;
                let total_bits = (sub_size - len) as f32 * LITERAL_BITS + match_bits;

                for ofs in 0..=(sub_size - len) {
                    let mut trial = [0u8; 16];
                    trial[..sub_size].copy_from_slice(&data[cur_offset..cur_offset + sub_size]);
                    trial[ofs..ofs + len]
                        .copy_from_slice(&data[prev_offset + ofs..prev_offset + ofs + len]);

                    let mut trial_decoded = [[0u8; 4]; 16];
                    if !unpack_fn(&trial[..sub_size], &mut trial_decoded) {
                        continue;
                    }

                    let trial_mse = compute_block_mse(
                        &orig_pixels[block_idx],
                        &trial_decoded,
                        params.color_weights,
                        inv_total_weight,
                    );
                    if trial_mse < thresh_mse {
                        let t = trial_mse * smooth_scale + total_bits * params.lambda;
                        if t < best_t {
                            best_t = t;
                            best_block[..sub_size].copy_from_slice(&trial[..sub_size]);
                        }
                    }
                }
            }
        }

        data[cur_offset..cur_offset + sub_size].copy_from_slice(&best_block[..sub_size]);
    }
}

pub fn apply_rdo_optimization(
    compressed: &mut [u8],
    rgba: &[u8],
    width: usize,
    height: usize,
    options: &CompressionOptions,
) {
    if options.rdo_lambda <= 0.0 {
        return;
    }

    // Protect complex bitstream formats (BC7, BC6H) and 1-bit alpha BC1a from byte splicing
    if matches!(options.format, Format::Bc7 | Format::Bc6h) || options.is_1bit_alpha {
        return;
    }

    let blocks_x = width.div_ceil(4);
    let blocks_y = height.div_ceil(4);
    let total_blocks = blocks_x * blocks_y;

    let mut orig_blocks = Vec::with_capacity(total_blocks);
    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let mut blk = [[0u8; 4]; 16];
            for py in 0..4 {
                let y = (by * 4 + py).min(height - 1);
                for px in 0..4 {
                    let x = (bx * 4 + px).min(width - 1);
                    let idx = py * 4 + px;
                    let off = (y * width + x) * 4;
                    blk[idx] = [rgba[off], rgba[off + 1], rgba[off + 2], rgba[off + 3]];
                }
            }
            orig_blocks.push(blk);
        }
    }

    let scales = if options.rdo_ultrasmooth {
        let mut sc = compute_block_mse_scales(rgba, width, height, blocks_x, blocks_y);
        let max_scale = options
            .rdo_smooth_block_scale
            .unwrap_or_else(|| match options.format {
                Format::Bc7 => 15.0 + (50.0 - 15.0) * (options.rdo_lambda / 4.0).min(1.0),
                Format::Bc1 => 15.0 + (50.0 - 15.0) * (options.rdo_lambda / 8.0).min(1.0),
                _ => 10.0 + (30.0 - 10.0) * (options.rdo_lambda / 4.0).min(1.0),
            });

        for s in &mut sc {
            if *s > 0.0 {
                *s = (*s * options.rdo_lambda.min(3.0)).max(max_scale);
            }
        }
        Some(sc)
    } else {
        None
    };

    let params = ReduceEntropyParams {
        lambda: options.rdo_lambda,
        lookback_window_size: options.rdo_lookback_window,
        try_two_matches: options.rdo_try_two_matches,
        smooth_block_max_mse_scale: options.rdo_smooth_block_scale.unwrap_or(15.0),
        color_weights: match options.format {
            Format::Bc4 => [1, 0, 0, 0],
            Format::Bc5 => [1, 1, 0, 0],
            _ => [1, 1, 1, 1],
        },
        ..Default::default()
    };

    match options.format {
        Format::Bc1 => {
            reduce_entropy(
                compressed,
                8,
                &orig_blocks,
                &params,
                scales.as_deref(),
                |blk, out| {
                    let mut rgb = [[0u8; 3]; 16];
                    read_color_block_bc1(blk.try_into().unwrap(), &mut rgb);
                    for i in 0..16 {
                        out[i] = [rgb[i][0], rgb[i][1], rgb[i][2], 255];
                    }
                    true
                },
            );
        }
        Format::Bc4 => {
            reduce_entropy(
                compressed,
                8,
                &orig_blocks,
                &params,
                scales.as_deref(),
                |blk, out| {
                    let mut r = [0u8; 16];
                    decompress_bc4(blk.try_into().unwrap(), &mut r);
                    for i in 0..16 {
                        out[i] = [r[i], r[i], r[i], 255];
                    }
                    true
                },
            );
        }
        Format::Bc3 => {
            reduce_entropy_strided(
                compressed,
                0,
                16,
                8,
                &orig_blocks,
                &params,
                None,
                |blk, out| {
                    let mut alpha = [0u8; 16];
                    decompress_alpha_bc3(blk.try_into().unwrap(), &mut alpha);
                    for i in 0..16 {
                        out[i] = [0, 0, 0, alpha[i]];
                    }
                    true
                },
            );
            reduce_entropy_strided(
                compressed,
                8,
                16,
                8,
                &orig_blocks,
                &params,
                scales.as_deref(),
                |blk, out| {
                    let mut rgb = [[0u8; 3]; 16];
                    read_color_block_bc1(blk.try_into().unwrap(), &mut rgb);
                    for i in 0..16 {
                        out[i] = [rgb[i][0], rgb[i][1], rgb[i][2], 255];
                    }
                    true
                },
            );
        }
        Format::Bc5 => {
            // CryEngine BC5: Block 0 (offset 0..8) is Y (Green)
            reduce_entropy_strided(
                compressed,
                0,
                16,
                8,
                &orig_blocks,
                &params,
                None,
                |blk, out| {
                    let mut g = [0u8; 16];
                    decompress_bc4(blk.try_into().unwrap(), &mut g);
                    for i in 0..16 {
                        out[i] = [0, g[i], 0, 255];
                    }
                    true
                },
            );
            // CryEngine BC5: Block 1 (offset 8..16) is X (Red)
            reduce_entropy_strided(
                compressed,
                8,
                16,
                8,
                &orig_blocks,
                &params,
                None,
                |blk, out| {
                    let mut r = [0u8; 16];
                    decompress_bc4(blk.try_into().unwrap(), &mut r);
                    for i in 0..16 {
                        out[i] = [r[i], 0, 0, 255];
                    }
                    true
                },
            );
        }
        Format::Ctx1 => {
            reduce_entropy(
                compressed,
                8,
                &orig_blocks,
                &params,
                scales.as_deref(),
                |blk, out| {
                    decompress_ctx1_block(blk.try_into().unwrap(), out);
                    true
                },
            );
        }
        _ => {}
    }
}
