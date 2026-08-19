//! Direct3D 11 BC7 and BC6H partition masks and anchor index tables.

/// 64 partition masks for 2-subset configurations (1 bit per pixel).
pub const PARTITION_MASKS_2: [u16; 64] = [
    0xCCCC, 0x8888, 0xEEEE, 0xECC8, 0xC880, 0xFEEC, 0xFEC8, 0xEC80, 0xC800, 0xFFEC, 0xFE80, 0xE800,
    0xFFE8, 0xFF00, 0xFFF0, 0xF000, 0xF710, 0x008E, 0x7100, 0x08CE, 0x008C, 0x7310, 0x3100, 0x8CCE,
    0x088C, 0x3110, 0x6666, 0x366C, 0x17E8, 0x0FF0, 0x718E, 0x399C, 0xAAAA, 0xF0F0, 0x5A5A, 0x33CC,
    0x3C3C, 0x55AA, 0x9696, 0xA55A, 0x73CE, 0x13C8, 0x324C, 0x3BDC, 0x6996, 0xC33C, 0x9966, 0x0660,
    0x0272, 0x04E4, 0x4E40, 0x2720, 0xC936, 0x936C, 0x39C6, 0x639C, 0x9336, 0x9CC6, 0x817E, 0xE718,
    0xCCF0, 0x0FCC, 0x7744, 0xEE22,
];

/// 64 partition masks for 3-subset configurations (2 bits per pixel packed: bits 0..15 = bit0, bits 16..31 = bit1).
pub const PARTITION_MASKS_3: [u32; 64] = [
    0xF60008CC, 0x73008CC8, 0x3310CC80, 0x00CEEC00, 0xCC003300, 0xCC0000CC, 0x00CCFF00, 0x3300CCCC,
    0xF0000F00, 0xF0000FF0, 0xFF0000F0, 0x88884444, 0x88886666, 0xCCCC2222, 0xEC80136C, 0x7310008C,
    0xC80036C8, 0x310008CE, 0xCCC03330, 0x0CCCF000, 0xEE0000EE, 0x77008888, 0xCC0022C0, 0x33004430,
    0x00CC0C22, 0xFC880344, 0x06606996, 0x66009960, 0xC88C0330, 0xF9000066, 0x0CC0C22C, 0x73108C00,
    0xEC801300, 0x08CEC400, 0xEC80004C, 0x44442222, 0x0F0000F0, 0x49242492, 0x42942942, 0x0C30C30C,
    0x03C0C03C, 0xFF0000AA, 0x5500AA00, 0xCCCC3030, 0x0C0CC0C0, 0x66669090, 0x0FF0A00A, 0x5550AAA0,
    0xF0000AAA, 0x0E0EE0E0, 0x88887070, 0x99906660, 0xE00E0EE0, 0x88880770, 0xF0000666, 0x99006600,
    0xFF000066, 0xC00C0CC0, 0xCCCC0330, 0x90006000, 0x08088080, 0xEEEE1010, 0xFFF0000A, 0x731008CE,
];

/// Anchor index for subset 1 in 2-subset partitions (first pixel in raster order belonging to subset 1).
pub const ANCHOR_INDEX_2_SUBSET_1: [usize; 64] = {
    let mut table = [0usize; 64];
    let mut i = 0;
    while i < 64 {
        table[i] = PARTITION_MASKS_2[i].trailing_zeros() as usize;
        i += 1;
    }
    table
};

/// Anchor index for subset 1 in 3-subset partitions.
pub const ANCHOR_INDEX_3_SUBSET_1: [usize; 64] = {
    let mut table = [0usize; 64];
    let mut p = 0;
    while p < 64 {
        let mut i = 0;
        while i < 16 {
            let mask = PARTITION_MASKS_3[p];
            let bit0 = (mask >> i) & 1;
            let bit1 = (mask >> (16 + i)) & 1;
            let subset = ((bit1 << 1) | bit0) as usize;
            if subset == 1 {
                table[p] = i;
                break;
            }
            i += 1;
        }
        p += 1;
    }
    table
};

/// Anchor index for subset 2 in 3-subset partitions.
pub const ANCHOR_INDEX_3_SUBSET_2: [usize; 64] = {
    let mut table = [0usize; 64];
    let mut p = 0;
    while p < 64 {
        let mut i = 0;
        while i < 16 {
            let mask = PARTITION_MASKS_3[p];
            let bit0 = (mask >> i) & 1;
            let bit1 = (mask >> (16 + i)) & 1;
            let subset = ((bit1 << 1) | bit0) as usize;
            if subset == 2 {
                table[p] = i;
                break;
            }
            i += 1;
        }
        p += 1;
    }
    table
};

/// Returns the subset index (0..=1) for a given pixel in a 2-subset partition.
#[inline(always)]
pub const fn get_subset_2(partition: usize, pixel_idx: usize) -> usize {
    ((PARTITION_MASKS_2[partition] >> pixel_idx) & 1) as usize
}

/// Returns the subset index (0..=2) for a given pixel in a 3-subset partition.
#[inline(always)]
pub const fn get_subset_3(partition: usize, pixel_idx: usize) -> usize {
    let mask = PARTITION_MASKS_3[partition];
    let bit0 = (mask >> pixel_idx) & 1;
    let bit1 = (mask >> (16 + pixel_idx)) & 1;
    ((bit1 << 1) | bit0) as usize
}
