pub fn write_alpha_block_bc2(alphas: &[u8; 16], mask: u16, block: &mut [u8; 8]) {
    for (i, byte) in block.iter_mut().enumerate() {
        let p0 = 2 * i;
        let p1 = 2 * i + 1;

        let a0 = if (mask & (1 << p0)) != 0 {
            alphas[p0]
        } else {
            0
        };
        let a1 = if (mask & (1 << p1)) != 0 {
            alphas[p1]
        } else {
            0
        };

        let q0 = ((a0 as u32 * 15 + 128) / 255) as u8;
        let q1 = ((a1 as u32 * 15 + 128) / 255) as u8;

        *byte = (q0 & 0x0F) | ((q1 & 0x0F) << 4);
    }
}

pub fn read_alpha_block_bc2(block: &[u8; 8], out: &mut [u8; 16]) {
    for (i, &byte) in block.iter().enumerate() {
        let q0 = byte & 0x0F;
        let q1 = byte >> 4;
        out[2 * i] = q0 * 0x11;
        out[2 * i + 1] = q1 * 0x11;
    }
}

#[inline(always)]
pub fn compress_alpha_bc2(alphas: &[u8; 16], mask: u16, block: &mut [u8; 8]) {
    write_alpha_block_bc2(alphas, mask, block);
}

#[inline(always)]
pub fn decompress_alpha_bc2(block: &[u8; 8], out: &mut [u8; 16]) {
    read_alpha_block_bc2(block, out);
}
