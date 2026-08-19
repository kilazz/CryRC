use super::quant::sign_extend;
use super::tables::{
    Field, Field::*, MODE_1_LAYOUT, MODE_2_LAYOUT, MODE_3_LAYOUT, MODE_4_LAYOUT, MODE_5_LAYOUT,
    MODE_6_LAYOUT, MODE_7_LAYOUT, MODE_8_LAYOUT, MODE_9_LAYOUT, MODE_10_LAYOUT, MODE_11_LAYOUT,
    MODE_12_LAYOUT, MODE_13_LAYOUT, MODE_14_LAYOUT,
};
use crate::math::bitstream::BitStream128;
use crate::tables::bc7_masks::ANCHOR_INDEX_2_SUBSET_1;

#[derive(Default, Clone, Copy)]
pub struct EndpointsRaw {
    pub rs1: u32,
    pub gs1: u32,
    pub bs1: u32,
    pub re1: u32,
    pub ge1: u32,
    pub be1: u32,
    pub rs2: u32,
    pub gs2: u32,
    pub bs2: u32,
    pub re2: u32,
    pub ge2: u32,
    pub be2: u32,
    pub partition: u32,
}

pub fn write_bc6h_block(
    mode: usize,
    partition: usize,
    endpoints_q: &[[[i32; 3]; 2]; 2],
    indices: &[u8; 16],
    is_signed: bool,
    out_block: &mut [u8; 16],
) {
    let mut bs = BitStream128::default();
    let is_two_subsets = mode <= 10;
    let layout: &[Field] = match mode {
        1 => &MODE_1_LAYOUT,
        2 => &MODE_2_LAYOUT,
        3 => &MODE_3_LAYOUT,
        4 => &MODE_4_LAYOUT,
        5 => &MODE_5_LAYOUT,
        6 => &MODE_6_LAYOUT,
        7 => &MODE_7_LAYOUT,
        8 => &MODE_8_LAYOUT,
        9 => &MODE_9_LAYOUT,
        10 => &MODE_10_LAYOUT,
        11 => &MODE_11_LAYOUT,
        12 => &MODE_12_LAYOUT,
        13 => &MODE_13_LAYOUT,
        14 => &MODE_14_LAYOUT,
        _ => unreachable!(),
    };

    let mut raw = EndpointsRaw {
        partition: partition as u32,
        ..Default::default()
    };

    if is_two_subsets {
        let (base_bits, delta_bits) = get_mode_precisions_2subsets(mode);
        raw.rs1 = (endpoints_q[0][0][0] as u32) & ((1 << base_bits[0]) - 1);
        raw.gs1 = (endpoints_q[0][0][1] as u32) & ((1 << base_bits[1]) - 1);
        raw.bs1 = (endpoints_q[0][0][2] as u32) & ((1 << base_bits[2]) - 1);

        if mode == 10 {
            raw.re1 = (endpoints_q[0][1][0] as u32) & 0x3F;
            raw.ge1 = (endpoints_q[0][1][1] as u32) & 0x3F;
            raw.be1 = (endpoints_q[0][1][2] as u32) & 0x3F;
            raw.rs2 = (endpoints_q[1][0][0] as u32) & 0x3F;
            raw.gs2 = (endpoints_q[1][0][1] as u32) & 0x3F;
            raw.bs2 = (endpoints_q[1][0][2] as u32) & 0x3F;
            raw.re2 = (endpoints_q[1][1][0] as u32) & 0x3F;
            raw.ge2 = (endpoints_q[1][1][1] as u32) & 0x3F;
            raw.be2 = (endpoints_q[1][1][2] as u32) & 0x3F;
        } else {
            let base_r = if is_signed {
                sign_extend(raw.rs1, base_bits[0])
            } else {
                raw.rs1 as i32
            };
            let base_g = if is_signed {
                sign_extend(raw.gs1, base_bits[1])
            } else {
                raw.gs1 as i32
            };
            let base_b = if is_signed {
                sign_extend(raw.bs1, base_bits[2])
            } else {
                raw.bs1 as i32
            };

            raw.re1 = (endpoints_q[0][1][0] - base_r) as u32 & ((1 << delta_bits[0]) - 1);
            raw.ge1 = (endpoints_q[0][1][1] - base_g) as u32 & ((1 << delta_bits[1]) - 1);
            raw.be1 = (endpoints_q[0][1][2] - base_b) as u32 & ((1 << delta_bits[2]) - 1);

            raw.rs2 = (endpoints_q[1][0][0] - base_r) as u32 & ((1 << delta_bits[0]) - 1);
            raw.gs2 = (endpoints_q[1][0][1] - base_g) as u32 & ((1 << delta_bits[1]) - 1);
            raw.bs2 = (endpoints_q[1][0][2] - base_b) as u32 & ((1 << delta_bits[2]) - 1);

            raw.re2 = (endpoints_q[1][1][0] - base_r) as u32 & ((1 << delta_bits[0]) - 1);
            raw.ge2 = (endpoints_q[1][1][1] - base_g) as u32 & ((1 << delta_bits[1]) - 1);
            raw.be2 = (endpoints_q[1][1][2] - base_b) as u32 & ((1 << delta_bits[2]) - 1);
        }
    } else {
        let (base_bits, delta_bits) = get_mode_precisions_1subset(mode);
        raw.rs1 = (endpoints_q[0][0][0] as u32) & ((1 << base_bits) - 1);
        raw.gs1 = (endpoints_q[0][0][1] as u32) & ((1 << base_bits) - 1);
        raw.bs1 = (endpoints_q[0][0][2] as u32) & ((1 << base_bits) - 1);

        if mode == 11 {
            raw.re1 = (endpoints_q[0][1][0] as u32) & 0x3FF;
            raw.ge1 = (endpoints_q[0][1][1] as u32) & 0x3FF;
            raw.be1 = (endpoints_q[0][1][2] as u32) & 0x3FF;
        } else {
            let base_r = if is_signed {
                sign_extend(raw.rs1, base_bits)
            } else {
                raw.rs1 as i32
            };
            let base_g = if is_signed {
                sign_extend(raw.gs1, base_bits)
            } else {
                raw.gs1 as i32
            };
            let base_b = if is_signed {
                sign_extend(raw.bs1, base_bits)
            } else {
                raw.bs1 as i32
            };

            raw.re1 = (endpoints_q[0][1][0] - base_r) as u32 & ((1 << delta_bits) - 1);
            raw.ge1 = (endpoints_q[0][1][1] - base_g) as u32 & ((1 << delta_bits) - 1);
            raw.be1 = (endpoints_q[0][1][2] - base_b) as u32 & ((1 << delta_bits) - 1);
        }
    }

    let mode_header = match mode {
        1 => 0b00,
        2 => 0b01,
        3 => 0b00010,
        4 => 0b00110,
        5 => 0b01010,
        6 => 0b01110,
        7 => 0b10010,
        8 => 0b10110,
        9 => 0b11010,
        10 => 0b11110,
        11 => 0b00011,
        12 => 0b00111,
        13 => 0b01011,
        14 => 0b01111,
        _ => unreachable!(),
    };

    for (i, &field) in layout.iter().enumerate() {
        let bit = match field {
            M(b) => (mode_header >> b) & 1,
            D(b) => (raw.partition >> b) & 1,
            RS1(b) => (raw.rs1 >> b) & 1,
            GS1(b) => (raw.gs1 >> b) & 1,
            BS1(b) => (raw.bs1 >> b) & 1,
            RE1(b) => (raw.re1 >> b) & 1,
            GE1(b) => (raw.ge1 >> b) & 1,
            BE1(b) => (raw.be1 >> b) & 1,
            RS2(b) => (raw.rs2 >> b) & 1,
            GS2(b) => (raw.gs2 >> b) & 1,
            BS2(b) => (raw.bs2 >> b) & 1,
            RE2(b) => (raw.re2 >> b) & 1,
            GE2(b) => (raw.ge2 >> b) & 1,
            BE2(b) => (raw.be2 >> b) & 1,
        };
        bs.write_bits(i as u32, 1, bit);
    }

    let mut pos = if is_two_subsets { 82 } else { 65 };
    let ib = if is_two_subsets { 3 } else { 4 };
    let anchor1 = if is_two_subsets {
        ANCHOR_INDEX_2_SUBSET_1[partition]
    } else {
        0
    };

    for (i, &idx) in indices.iter().enumerate() {
        let is_anchor = i == 0 || (is_two_subsets && i == anchor1);
        let count = if is_anchor { ib - 1 } else { ib };
        bs.write_bits(pos, count, idx as u32);
        pos += count;
    }

    *out_block = bs.to_bytes();
}

#[inline(always)]
pub fn get_mode_precisions_2subsets(mode: usize) -> ([u32; 3], [u32; 3]) {
    match mode {
        1 => ([10, 10, 10], [5, 5, 5]),
        2 => ([7, 7, 7], [6, 6, 6]),
        3 => ([11, 11, 11], [5, 4, 4]),
        4 => ([11, 11, 11], [4, 5, 4]),
        5 => ([11, 11, 11], [4, 4, 5]),
        6 => ([9, 9, 9], [5, 5, 5]),
        7 => ([8, 8, 8], [6, 5, 5]),
        8 => ([8, 8, 8], [5, 6, 5]),
        9 => ([8, 8, 8], [5, 5, 6]),
        10 => ([6, 6, 6], [6, 6, 6]),
        _ => unreachable!(),
    }
}

#[inline(always)]
pub fn get_mode_precisions_1subset(mode: usize) -> (u32, u32) {
    match mode {
        11 => (10, 10),
        12 => (11, 9),
        13 => (12, 8),
        14 => (16, 4),
        _ => unreachable!(),
    }
}
