// Copyright 2001-2026 Crytek GmbH / Crytek Group. All rights reserved.
// BC4 / 3Dc+ Single-Channel Block Compressor (Unsigned & Signed SNORM)

use crate::compressors::bc3::{
    compress_alpha_bc3, decompress_alpha_bc3, read_alpha_block_bc3, write_alpha_block_bc3,
};

#[inline(always)]
pub fn build_codebook_8_signed(a0: i8, a1: i8) -> [i8; 8] {
    let a0 = a0 as i32;
    let a1 = a1 as i32;
    [
        a0 as i8,
        a1 as i8,
        ((6 * a0 + a1) / 7) as i8,
        ((5 * a0 + 2 * a1) / 7) as i8,
        ((4 * a0 + 3 * a1) / 7) as i8,
        ((3 * a0 + 4 * a1) / 7) as i8,
        ((2 * a0 + 5 * a1) / 7) as i8,
        ((a0 + 6 * a1) / 7) as i8,
    ]
}

#[inline(always)]
pub fn build_codebook_6_signed(a0: i8, a1: i8) -> [i8; 8] {
    let a0 = a0 as i32;
    let a1 = a1 as i32;
    [
        a0 as i8,
        a1 as i8,
        ((4 * a0 + a1) / 5) as i8,
        ((3 * a0 + 2 * a1) / 5) as i8,
        ((2 * a0 + 3 * a1) / 5) as i8,
        ((a0 + 4 * a1) / 5) as i8,
        -127,
        127,
    ]
}

#[inline(always)]
pub fn build_codebook_8_u16(a0: u16, a1: u16) -> [u16; 8] {
    let a0 = a0 as u32;
    let a1 = a1 as u32;
    [
        a0 as u16,
        a1 as u16,
        ((6 * a0 + a1) / 7) as u16,
        ((5 * a0 + 2 * a1) / 7) as u16,
        ((4 * a0 + 3 * a1) / 7) as u16,
        ((3 * a0 + 4 * a1) / 7) as u16,
        ((2 * a0 + 5 * a1) / 7) as u16,
        ((a0 + 6 * a1) / 7) as u16,
    ]
}

#[inline(always)]
pub fn build_codebook_8_i16(a0: i16, a1: i16) -> [i16; 8] {
    let a0 = a0 as i32;
    let a1 = a1 as i32;
    [
        a0 as i16,
        a1 as i16,
        ((6 * a0 + a1) / 7) as i16,
        ((5 * a0 + 2 * a1) / 7) as i16,
        ((4 * a0 + 3 * a1) / 7) as i16,
        ((3 * a0 + 4 * a1) / 7) as i16,
        ((2 * a0 + 5 * a1) / 7) as i16,
        ((a0 + 6 * a1) / 7) as i16,
    ]
}

pub fn fit_codes_signed(
    values: &[i8; 16],
    mask: u16,
    codes: &[i8; 8],
    indices: &mut [u8; 16],
) -> u32 {
    let mut total_error = 0u32;
    for (i, &value) in values.iter().enumerate() {
        if (mask & (1 << i)) == 0 {
            indices[i] = 0;
            continue;
        }

        let val = value as i32;
        let mut min_dist = i32::MAX;
        let mut best_idx = 0u8;

        for (j, &code) in codes.iter().enumerate() {
            let diff = val - (code as i32);
            let dist = diff * diff;
            if dist < min_dist {
                min_dist = dist;
                best_idx = j as u8;
            }
        }

        indices[i] = best_idx;
        total_error += min_dist as u32;
    }
    total_error
}

#[inline(always)]
pub fn compress_bc4(data: &[u8; 16], mask: u16, flags: u32, block: &mut [u8; 8]) {
    compress_alpha_bc3(data, mask, flags, block);
}

pub fn compress_bc4_signed(data: &[i8; 16], mask: u16, _flags: u32, block: &mut [u8; 8]) {
    let mut min7 = 127i8;
    let mut max7 = -127i8;
    let mut min5 = 127i8;
    let mut max5 = -127i8;

    for (i, &raw_val) in data.iter().enumerate() {
        if (mask & (1 << i)) == 0 {
            continue;
        }
        let val = raw_val.clamp(-127, 127);
        min7 = min7.min(val);
        max7 = max7.max(val);
        if val != -127 && val != 127 {
            min5 = min5.min(val);
            max5 = max5.max(val);
        }
    }

    if min7 > max7 {
        block.fill(0);
        return;
    }

    if min5 > max5 {
        min5 = min7;
        max5 = max7;
    }

    // Flat block handling (e.g. 0 for neutral normal components)
    if max7 == min7 {
        let indices = [0u8; 16];
        write_alpha_block_bc3(max7 as u8, max7 as u8, &indices, block);
        return;
    }

    // 8-step signed mode (max7 > min7)
    let codes7 = build_codebook_8_signed(max7, min7);
    let mut indices7 = [0u8; 16];
    let err7 = fit_codes_signed(data, mask, &codes7, &mut indices7);

    // 6-step signed mode (min5 <= max5)
    let codes5 = build_codebook_6_signed(min5, max5);
    let mut indices5 = [0u8; 16];
    let err5 = fit_codes_signed(data, mask, &codes5, &mut indices5);

    if err7 <= err5 {
        write_alpha_block_bc3(max7 as u8, min7 as u8, &indices7, block);
    } else {
        write_alpha_block_bc3(min5 as u8, max5 as u8, &indices5, block);
    }
}

pub fn compress_bc4_u16(data: &[u16; 16], mask: u16, block: &mut [u8; 8]) {
    let mut min16 = 65535u16;
    let mut max16 = 0u16;

    for (i, &val) in data.iter().enumerate() {
        if (mask & (1 << i)) == 0 {
            continue;
        }
        if val < min16 {
            min16 = val;
        }
        if val > max16 {
            max16 = val;
        }
    }

    let e0_8 = (max16 >> 8) as u8;
    let e1_8 = (min16 >> 8) as u8;
    let e0_16 = (e0_8 as u16) * 257;
    let e1_16 = (e1_8 as u16) * 257;

    let codes = build_codebook_8_u16(e0_16, e1_16);
    let mut indices = [0u8; 16];
    for (i, &item) in data.iter().enumerate() {
        if (mask & (1 << i)) == 0 {
            continue;
        }
        let val = item as i64;
        let mut min_dist = i64::MAX;
        let mut best_idx = 0u8;
        for (j, &code) in codes.iter().enumerate() {
            let diff = val - (code as i64);
            let dist = diff * diff;
            if dist < min_dist {
                min_dist = dist;
                best_idx = j as u8;
            }
        }
        indices[i] = best_idx;
    }

    write_alpha_block_bc3(e0_8, e1_8, &indices, block);
}

pub fn compress_bc4_i16(data: &[i16; 16], mask: u16, block: &mut [u8; 8]) {
    let mut min16 = 32767i16;
    let mut max16 = -32767i16;

    for (i, &raw_val) in data.iter().enumerate() {
        if (mask & (1 << i)) == 0 {
            continue;
        }
        let val = raw_val.clamp(-32767, 32767);
        if val < min16 {
            min16 = val;
        }
        if val > max16 {
            max16 = val;
        }
    }

    let e0_8 = ((max16 as i32 * 127) / 32767).clamp(-127, 127) as i8;
    let e1_8 = ((min16 as i32 * 127) / 32767).clamp(-127, 127) as i8;
    let e0_16 = ((e0_8 as i32 * 32767) / 127) as i16;
    let e1_16 = ((e1_8 as i32 * 32767) / 127) as i16;

    let codes = build_codebook_8_i16(e0_16, e1_16);
    let mut indices = [0u8; 16];
    for (i, &item) in data.iter().enumerate() {
        if (mask & (1 << i)) == 0 {
            continue;
        }
        let val = item as i64;
        let mut min_dist = i64::MAX;
        let mut best_idx = 0u8;
        for (j, &code) in codes.iter().enumerate() {
            let diff = val - (code as i64);
            let dist = diff * diff;
            if dist < min_dist {
                min_dist = dist;
                best_idx = j as u8;
            }
        }
        indices[i] = best_idx;
    }

    write_alpha_block_bc3(e0_8 as u8, e1_8 as u8, &indices, block);
}

#[inline(always)]
pub fn decompress_bc4(block: &[u8; 8], out: &mut [u8; 16]) {
    decompress_alpha_bc3(block, out);
}

pub fn decompress_bc4_signed(block: &[u8; 8], out: &mut [i8; 16]) {
    let (alpha0, alpha1, indices) = read_alpha_block_bc3(block);
    let a0 = alpha0 as i8;
    let a1 = alpha1 as i8;
    let codes = if a0 > a1 {
        build_codebook_8_signed(a0, a1)
    } else {
        build_codebook_6_signed(a0, a1)
    };

    for i in 0..16 {
        out[i] = codes[indices[i] as usize];
    }
}

pub fn decompress_bc4_u16(block: &[u8; 8], out: &mut [u16; 16]) {
    let (alpha0, alpha1, indices) = read_alpha_block_bc3(block);
    let a0 = (alpha0 as u16) * 257;
    let a1 = (alpha1 as u16) * 257;
    let codes = build_codebook_8_u16(a0, a1);

    for i in 0..16 {
        out[i] = codes[indices[i] as usize];
    }
}

pub fn decompress_bc4_i16(block: &[u8; 8], out: &mut [i16; 16]) {
    let (alpha0, alpha1, indices) = read_alpha_block_bc3(block);
    let a0 = ((alpha0 as i8 as i32 * 32767) / 127) as i16;
    let a1 = ((alpha1 as i8 as i32 * 32767) / 127) as i16;
    let codes = build_codebook_8_i16(a0, a1);

    for i in 0..16 {
        out[i] = codes[indices[i] as usize];
    }
}
