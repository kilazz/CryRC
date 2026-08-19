#[inline(always)]
pub fn build_codebook_8(a0: u8, a1: u8) -> [u8; 8] {
    let a0 = a0 as i32;
    let a1 = a1 as i32;
    [
        a0 as u8,
        a1 as u8,
        ((6 * a0 + a1) / 7) as u8,
        ((5 * a0 + 2 * a1) / 7) as u8,
        ((4 * a0 + 3 * a1) / 7) as u8,
        ((3 * a0 + 4 * a1) / 7) as u8,
        ((2 * a0 + 5 * a1) / 7) as u8,
        ((a0 + 6 * a1) / 7) as u8,
    ]
}

#[inline(always)]
pub fn build_codebook_6(a0: u8, a1: u8) -> [u8; 8] {
    let a0 = a0 as i32;
    let a1 = a1 as i32;
    [
        a0 as u8,
        a1 as u8,
        ((4 * a0 + a1) / 5) as u8,
        ((3 * a0 + 2 * a1) / 5) as u8,
        ((2 * a0 + 3 * a1) / 5) as u8,
        ((a0 + 4 * a1) / 5) as u8,
        0,
        255,
    ]
}

pub fn fit_codes(alphas: &[u8; 16], mask: u16, codes: &[u8; 8], indices: &mut [u8; 16]) -> u32 {
    let mut total_error = 0u32;
    for (i, &alpha) in alphas.iter().enumerate() {
        if (mask & (1 << i)) == 0 {
            indices[i] = 0;
            continue;
        }

        let val = alpha as i32;
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

pub fn write_alpha_block_bc3(alpha0: u8, alpha1: u8, indices: &[u8; 16], block: &mut [u8; 8]) {
    block[0] = alpha0;
    block[1] = alpha1;

    let mut val0: u32 = 0;
    for (j, &idx) in indices[..8].iter().enumerate() {
        val0 |= ((idx & 0x07) as u32) << (3 * j);
    }
    block[2] = (val0 & 0xFF) as u8;
    block[3] = ((val0 >> 8) & 0xFF) as u8;
    block[4] = ((val0 >> 16) & 0xFF) as u8;

    let mut val1: u32 = 0;
    for (j, &idx) in indices[8..].iter().enumerate() {
        val1 |= ((idx & 0x07) as u32) << (3 * j);
    }
    block[5] = (val1 & 0xFF) as u8;
    block[6] = ((val1 >> 8) & 0xFF) as u8;
    block[7] = ((val1 >> 16) & 0xFF) as u8;
}

pub fn read_alpha_block_bc3(block: &[u8; 8]) -> (u8, u8, [u8; 16]) {
    let alpha0 = block[0];
    let alpha1 = block[1];
    let mut indices = [0u8; 16];

    let val0 = (block[2] as u32) | ((block[3] as u32) << 8) | ((block[4] as u32) << 16);
    let val1 = (block[5] as u32) | ((block[6] as u32) << 8) | ((block[7] as u32) << 16);

    for j in 0..8 {
        indices[j] = ((val0 >> (3 * j)) & 0x07) as u8;
        indices[8 + j] = ((val1 >> (3 * j)) & 0x07) as u8;
    }

    (alpha0, alpha1, indices)
}

pub fn compress_alpha_bc3(alphas: &[u8; 16], mask: u16, _flags: u32, block: &mut [u8; 8]) {
    let mut min7 = 255u8;
    let mut max7 = 0u8;
    let mut min5 = 255u8;
    let mut max5 = 0u8;

    for (i, &val) in alphas.iter().enumerate() {
        if (mask & (1 << i)) == 0 {
            continue;
        }
        if val < min7 {
            min7 = val;
        }
        if val > max7 {
            max7 = val;
        }
        if val != 0 && val < min5 {
            min5 = val;
        }
        if val != 255 && val > max5 {
            max5 = val;
        }
    }

    if min5 > max5 {
        min5 = max5;
    }

    let codes7 = build_codebook_8(max7, min7);
    let mut indices7 = [0u8; 16];
    let err7 = fit_codes(alphas, mask, &codes7, &mut indices7);

    let codes5 = build_codebook_6(min5, max5);
    let mut indices5 = [0u8; 16];
    let err5 = fit_codes(alphas, mask, &codes5, &mut indices5);

    if err7 <= err5 {
        write_alpha_block_bc3(max7, min7, &indices7, block);
    } else {
        write_alpha_block_bc3(min5, max5, &indices5, block);
    }
}

pub fn decompress_alpha_bc3(block: &[u8; 8], out: &mut [u8; 16]) {
    let (alpha0, alpha1, indices) = read_alpha_block_bc3(block);
    let codes = if alpha0 > alpha1 {
        build_codebook_8(alpha0, alpha1)
    } else {
        build_codebook_6(alpha0, alpha1)
    };

    for i in 0..16 {
        out[i] = codes[indices[i] as usize];
    }
}
