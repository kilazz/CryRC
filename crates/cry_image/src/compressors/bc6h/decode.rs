use super::encode::{EndpointsRaw, get_mode_precisions_1subset, get_mode_precisions_2subsets};
use super::quant::{sign_extend, unquantize_bc6h};
use super::tables::{
    Field, Field::*, MODE_1_LAYOUT, MODE_2_LAYOUT, MODE_3_LAYOUT, MODE_4_LAYOUT, MODE_5_LAYOUT,
    MODE_6_LAYOUT, MODE_7_LAYOUT, MODE_8_LAYOUT, MODE_9_LAYOUT, MODE_10_LAYOUT, MODE_11_LAYOUT,
    MODE_12_LAYOUT, MODE_13_LAYOUT, MODE_14_LAYOUT,
};
use crate::math::bitstream::BitStream128;
use crate::tables::WEIGHTS_F32;
use crate::tables::bc7_masks::{ANCHOR_INDEX_2_SUBSET_1, get_subset_2};

pub fn decompress_bc6h_block(block: &[u8; 16], is_signed: bool, out_rgb: &mut [[f32; 3]; 16]) {
    let bs = BitStream128::from_bytes(block);
    let m2 = bs.read_bits(0, 2);
    let m5 = bs.read_bits(0, 5);

    let (mode_idx, layout): (usize, &[Field]) = match m2 {
        0b00 => (1, &MODE_1_LAYOUT),
        0b01 => (2, &MODE_2_LAYOUT),
        _ => match m5 {
            0b00010 => (3, &MODE_3_LAYOUT),
            0b00110 => (4, &MODE_4_LAYOUT),
            0b01010 => (5, &MODE_5_LAYOUT),
            0b01110 => (6, &MODE_6_LAYOUT),
            0b10010 => (7, &MODE_7_LAYOUT),
            0b10110 => (8, &MODE_8_LAYOUT),
            0b11010 => (9, &MODE_9_LAYOUT),
            0b11110 => (10, &MODE_10_LAYOUT),
            0b00011 => (11, &MODE_11_LAYOUT),
            0b00111 => (12, &MODE_12_LAYOUT),
            0b01011 => (13, &MODE_13_LAYOUT),
            0b01111 => (14, &MODE_14_LAYOUT),
            _ => {
                out_rgb.fill([0.0, 0.0, 0.0]);
                return;
            }
        },
    };

    let mut raw = EndpointsRaw::default();
    for (i, &field) in layout.iter().enumerate() {
        let bit = bs.read_bits(i as u32, 1);
        match field {
            M(_) => {}
            D(b) => raw.partition |= bit << b,
            RS1(b) => raw.rs1 |= bit << b,
            GS1(b) => raw.gs1 |= bit << b,
            BS1(b) => raw.bs1 |= bit << b,
            RE1(b) => raw.re1 |= bit << b,
            GE1(b) => raw.ge1 |= bit << b,
            BE1(b) => raw.be1 |= bit << b,
            RS2(b) => raw.rs2 |= bit << b,
            GS2(b) => raw.gs2 |= bit << b,
            BS2(b) => raw.bs2 |= bit << b,
            RE2(b) => raw.re2 |= bit << b,
            GE2(b) => raw.ge2 |= bit << b,
            BE2(b) => raw.be2 |= bit << b,
        }
    }

    let is_two_subsets = mode_idx <= 10;
    let index_start_bit = if is_two_subsets { 82 } else { 65 };
    let mut ep = [[[0.0f32; 3]; 2]; 2];

    if is_two_subsets {
        let (base_bits, delta_bits) = get_mode_precisions_2subsets(mode_idx);

        let s0_r = if is_signed {
            sign_extend(raw.rs1, base_bits[0])
        } else {
            raw.rs1 as i32
        };
        let s0_g = if is_signed {
            sign_extend(raw.gs1, base_bits[1])
        } else {
            raw.gs1 as i32
        };
        let s0_b = if is_signed {
            sign_extend(raw.bs1, base_bits[2])
        } else {
            raw.bs1 as i32
        };

        let (e0_r, e0_g, e0_b) = if mode_idx == 10 {
            if is_signed {
                (
                    sign_extend(raw.re1, 6),
                    sign_extend(raw.ge1, 6),
                    sign_extend(raw.be1, 6),
                )
            } else {
                (raw.re1 as i32, raw.ge1 as i32, raw.be1 as i32)
            }
        } else {
            (
                s0_r + sign_extend(raw.re1, delta_bits[0]),
                s0_g + sign_extend(raw.ge1, delta_bits[1]),
                s0_b + sign_extend(raw.be1, delta_bits[2]),
            )
        };

        let (s1_r, s1_g, s1_b) = if mode_idx == 10 {
            if is_signed {
                (
                    sign_extend(raw.rs2, 6),
                    sign_extend(raw.gs2, 6),
                    sign_extend(raw.bs2, 6),
                )
            } else {
                (raw.rs2 as i32, raw.gs2 as i32, raw.bs2 as i32)
            }
        } else {
            (
                s0_r + sign_extend(raw.rs2, delta_bits[0]),
                s0_g + sign_extend(raw.gs2, delta_bits[1]),
                s0_b + sign_extend(raw.bs2, delta_bits[2]),
            )
        };

        let (e1_r, e1_g, e1_b) = if mode_idx == 10 {
            if is_signed {
                (
                    sign_extend(raw.re2, 6),
                    sign_extend(raw.ge2, 6),
                    sign_extend(raw.be2, 6),
                )
            } else {
                (raw.re2 as i32, raw.ge2 as i32, raw.be2 as i32)
            }
        } else {
            (
                s0_r + sign_extend(raw.re2, delta_bits[0]),
                s0_g + sign_extend(raw.ge2, delta_bits[1]),
                s0_b + sign_extend(raw.be2, delta_bits[2]),
            )
        };

        ep[0][0] = [
            unquantize_bc6h(s0_r, base_bits[0], is_signed),
            unquantize_bc6h(s0_g, base_bits[1], is_signed),
            unquantize_bc6h(s0_b, base_bits[2], is_signed),
        ];
        ep[0][1] = [
            unquantize_bc6h(e0_r, base_bits[0], is_signed),
            unquantize_bc6h(e0_g, base_bits[1], is_signed),
            unquantize_bc6h(e0_b, base_bits[2], is_signed),
        ];
        ep[1][0] = [
            unquantize_bc6h(s1_r, base_bits[0], is_signed),
            unquantize_bc6h(s1_g, base_bits[1], is_signed),
            unquantize_bc6h(s1_b, base_bits[2], is_signed),
        ];
        ep[1][1] = [
            unquantize_bc6h(e1_r, base_bits[0], is_signed),
            unquantize_bc6h(e1_g, base_bits[1], is_signed),
            unquantize_bc6h(e1_b, base_bits[2], is_signed),
        ];

        let partition = (raw.partition as usize).min(31);
        let anchor1 = ANCHOR_INDEX_2_SUBSET_1[partition];
        let mut pos = index_start_bit;

        for (i, slot) in out_rgb.iter_mut().enumerate() {
            let subset = get_subset_2(partition, i);
            let is_anchor = i == 0 || i == anchor1;
            let bits = if is_anchor { 2 } else { 3 };
            let idx = bs.read_bits(pos, bits) as usize;
            pos += bits;

            let w = WEIGHTS_F32[3][idx];
            let c0 = ep[subset][0];
            let c1 = ep[subset][1];
            *slot = [
                c0[0] * (1.0 - w) + c1[0] * w,
                c0[1] * (1.0 - w) + c1[1] * w,
                c0[2] * (1.0 - w) + c1[2] * w,
            ];
        }
    } else {
        let (base_bits, delta_bits) = get_mode_precisions_1subset(mode_idx);
        let s0_r = if is_signed {
            sign_extend(raw.rs1, base_bits)
        } else {
            raw.rs1 as i32
        };
        let s0_g = if is_signed {
            sign_extend(raw.gs1, base_bits)
        } else {
            raw.gs1 as i32
        };
        let s0_b = if is_signed {
            sign_extend(raw.bs1, base_bits)
        } else {
            raw.bs1 as i32
        };

        let (e0_r, e0_g, e0_b) = if mode_idx == 11 {
            if is_signed {
                (
                    sign_extend(raw.re1, 10),
                    sign_extend(raw.ge1, 10),
                    sign_extend(raw.be1, 10),
                )
            } else {
                (raw.re1 as i32, raw.ge1 as i32, raw.be1 as i32)
            }
        } else {
            (
                s0_r + sign_extend(raw.re1, delta_bits),
                s0_g + sign_extend(raw.ge1, delta_bits),
                s0_b + sign_extend(raw.be1, delta_bits),
            )
        };

        ep[0][0] = [
            unquantize_bc6h(s0_r, base_bits, is_signed),
            unquantize_bc6h(s0_g, base_bits, is_signed),
            unquantize_bc6h(s0_b, base_bits, is_signed),
        ];
        ep[0][1] = [
            unquantize_bc6h(e0_r, base_bits, is_signed),
            unquantize_bc6h(e0_g, base_bits, is_signed),
            unquantize_bc6h(e0_b, base_bits, is_signed),
        ];

        let mut pos = index_start_bit;
        for (i, slot) in out_rgb.iter_mut().enumerate() {
            let bits = if i == 0 { 3 } else { 4 };
            let idx = bs.read_bits(pos, bits) as usize;
            pos += bits;

            let w = WEIGHTS_F32[4][idx];
            let c0 = ep[0][0];
            let c1 = ep[0][1];
            *slot = [
                c0[0] * (1.0 - w) + c1[0] * w,
                c0[1] * (1.0 - w) + c1[1] * w,
                c0[2] * (1.0 - w) + c1[2] * w,
            ];
        }
    }
}
