use super::tables::BC7_MODE_INFO;
pub use crate::math::bitstream::BitStream128;
use crate::math::vector::Vec4;
use crate::tables::bc7_masks::{
    ANCHOR_INDEX_2_SUBSET_1, ANCHOR_INDEX_3_SUBSET_1, ANCHOR_INDEX_3_SUBSET_2, get_subset_2,
    get_subset_3,
};

#[inline(always)]
pub fn expand_quantized(val: u32, bits: u32, p_bit: Option<u32>) -> u8 {
    let (c, total_bits) = match p_bit {
        Some(p) => ((val << 1) | (p & 1), bits + 1),
        None => (val, bits),
    };
    let shift = 8 - total_bits;
    ((c << shift) | (c >> (2 * total_bits - 8))) as u8
}

#[inline(always)]
pub fn quantize_endpoint(
    color: Vec4,
    color_bits: u32,
    alpha_bits: u32,
    p_bit: Option<u32>,
) -> ([u8; 4], Vec4) {
    let clamp = color.clamp(0.0, 1.0);
    let max_c = ((1 << color_bits) - 1) as f32;

    let r_q = (clamp.x * max_c).round() as u32;
    let g_q = (clamp.y * max_c).round() as u32;
    let b_q = (clamp.z * max_c).round() as u32;

    let r_unq = expand_quantized(r_q, color_bits, p_bit);
    let g_unq = expand_quantized(g_q, color_bits, p_bit);
    let b_unq = expand_quantized(b_q, color_bits, p_bit);

    let (a_unq, a_norm) = if alpha_bits > 0 {
        let max_a = ((1 << alpha_bits) - 1) as f32;
        let a_q = (clamp.w * max_a).round() as u32;
        let unq = expand_quantized(a_q, alpha_bits, p_bit);
        (unq, unq as f32 / 255.0)
    } else {
        (255, 1.0)
    };

    (
        [r_unq, g_unq, b_unq, a_unq],
        Vec4::new(
            r_unq as f32 / 255.0,
            g_unq as f32 / 255.0,
            b_unq as f32 / 255.0,
            a_norm,
        ),
    )
}

pub fn fix_bc7_anchor_indices(
    mode: usize,
    partition: usize,
    snapped_start: &mut [[u8; 4]; 3],
    snapped_end: &mut [[u8; 4]; 3],
    p_bits: &mut [u8; 6],
    indices: &mut [u8; 16],
    alpha_indices: &mut [u8; 16],
) {
    let info = &BC7_MODE_INFO[mode];
    let ib = info.index_bits as usize;
    let ccs = 1 << ib;

    let anchors = match info.num_subsets {
        2 => [0, ANCHOR_INDEX_2_SUBSET_1[partition], 0],
        3 => [
            0,
            ANCHOR_INDEX_3_SUBSET_1[partition],
            ANCHOR_INDEX_3_SUBSET_2[partition],
        ],
        _ => [0, 0, 0],
    };

    for s in 0..info.num_subsets {
        let anchor_idx = anchors[s];
        if indices[anchor_idx] >= (ccs / 2) as u8 {
            std::mem::swap(&mut snapped_start[s][0], &mut snapped_end[s][0]);
            std::mem::swap(&mut snapped_start[s][1], &mut snapped_end[s][1]);
            std::mem::swap(&mut snapped_start[s][2], &mut snapped_end[s][2]);
            if mode != 4 && mode != 5 {
                std::mem::swap(&mut snapped_start[s][3], &mut snapped_end[s][3]);
            }
            if info.endpoint_pbits > 0 {
                p_bits.swap(s * 2, s * 2 + 1);
            }

            for (i, idx) in indices.iter_mut().enumerate() {
                let belongs = match info.num_subsets {
                    1 => true,
                    2 => get_subset_2(partition, i) == s,
                    3 => get_subset_3(partition, i) == s,
                    _ => false,
                };
                if belongs {
                    *idx = ((ccs - 1) as u8) - *idx;
                }
            }
        }
    }

    if mode == 4 || mode == 5 {
        let a_bits = if mode == 4 { 3 } else { 2 };
        let a_ccs = 1 << a_bits;
        if alpha_indices[0] >= (a_ccs / 2) as u8 {
            std::mem::swap(&mut snapped_start[0][3], &mut snapped_end[0][3]);
            for a_idx in alpha_indices.iter_mut() {
                *a_idx = ((a_ccs - 1) as u8) - *a_idx;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn write_bc7_block(
    mode: usize,
    partition: usize,
    rotation: usize,
    idx_mode: usize,
    start: &[[u8; 4]; 3],
    end: &[[u8; 4]; 3],
    p_bits: &[u8; 6],
    indices: &[u8; 16],
    alpha_indices: &[u8; 16],
    out_block: &mut [u8; 16],
) {
    let mut bs = BitStream128::default();
    let info = &BC7_MODE_INFO[mode];

    bs.write_bits(0, (mode + 1) as u32, 1 << mode);
    let mut pos = (mode + 1) as u32;

    if info.partition_bits > 0 {
        bs.write_bits(pos, info.partition_bits, partition as u32);
        pos += info.partition_bits;
    }
    if info.rotation_bits > 0 {
        bs.write_bits(pos, info.rotation_bits, rotation as u32);
        pos += info.rotation_bits;
    }
    if info.index_selection_bits > 0 {
        bs.write_bits(pos, info.index_selection_bits, idx_mode as u32);
        pos += info.index_selection_bits;
    }

    let cb = info.color_bits;
    let shift_c = 8 - cb;
    for s in 0..info.num_subsets {
        bs.write_bits(pos, cb, (start[s][0] >> shift_c) as u32);
        pos += cb;
        bs.write_bits(pos, cb, (end[s][0] >> shift_c) as u32);
        pos += cb;
    }
    for s in 0..info.num_subsets {
        bs.write_bits(pos, cb, (start[s][1] >> shift_c) as u32);
        pos += cb;
        bs.write_bits(pos, cb, (end[s][1] >> shift_c) as u32);
        pos += cb;
    }
    for s in 0..info.num_subsets {
        bs.write_bits(pos, cb, (start[s][2] >> shift_c) as u32);
        pos += cb;
        bs.write_bits(pos, cb, (end[s][2] >> shift_c) as u32);
        pos += cb;
    }

    if info.alpha_bits > 0 {
        let ab = info.alpha_bits;
        let shift_a = 8 - ab;
        for s in 0..info.num_subsets {
            bs.write_bits(pos, ab, (start[s][3] >> shift_a) as u32);
            pos += ab;
            bs.write_bits(pos, ab, (end[s][3] >> shift_a) as u32);
            pos += ab;
        }
    }

    let total_pbits =
        info.num_subsets * (info.endpoint_pbits as usize * 2 + info.shared_pbits as usize);
    for &p in &p_bits[..total_pbits] {
        bs.write_bits(pos, 1, p as u32);
        pos += 1;
    }

    if mode == 4 {
        let (color_bits, a_bits) = if idx_mode == 0 { (2, 3) } else { (3, 2) };
        for (i, &idx) in indices.iter().enumerate() {
            let bits = if i == 0 { color_bits - 1 } else { color_bits };
            bs.write_bits(pos, bits, idx as u32);
            pos += bits;
        }
        for (i, &a_idx) in alpha_indices.iter().enumerate() {
            let bits = if i == 0 { a_bits - 1 } else { a_bits };
            bs.write_bits(pos, bits, a_idx as u32);
            pos += bits;
        }
    } else if mode == 5 {
        for (i, &idx) in indices.iter().enumerate() {
            let bits = if i == 0 { 1 } else { 2 };
            bs.write_bits(pos, bits, idx as u32);
            pos += bits;
        }
        for (i, &a_idx) in alpha_indices.iter().enumerate() {
            let bits = if i == 0 { 1 } else { 2 };
            bs.write_bits(pos, bits, a_idx as u32);
            pos += bits;
        }
    } else {
        let ib = info.index_bits;
        let anchors = match info.num_subsets {
            2 => [0, ANCHOR_INDEX_2_SUBSET_1[partition], 0],
            3 => [
                0,
                ANCHOR_INDEX_3_SUBSET_1[partition],
                ANCHOR_INDEX_3_SUBSET_2[partition],
            ],
            _ => [0, 0, 0],
        };

        for (i, &idx) in indices.iter().enumerate() {
            let is_anchor = match info.num_subsets {
                1 => i == 0,
                2 => i == anchors[0] || i == anchors[1],
                3 => i == anchors[0] || i == anchors[1] || i == anchors[2],
                _ => false,
            };
            let count = if is_anchor { ib - 1 } else { ib };
            bs.write_bits(pos, count, idx as u32);
            pos += count;
        }
    }

    *out_block = bs.to_bytes();
}

#[inline(always)]
pub fn encode_bc7_block_mode6(
    low: [u8; 4],
    high: [u8; 4],
    pbits: [u8; 2],
    selectors: &[u8; 16],
    out_block: &mut [u8; 16],
) {
    let mut low_ep = low;
    let mut high_ep = high;
    let mut pb = pbits;
    let mut inv = 0u8;

    if (selectors[0] & 8) != 0 {
        inv = 15;
        std::mem::swap(&mut low_ep, &mut high_ep);
        pb.swap(0, 1);
    }

    let mut l: u64 = 1 << 6;
    l |= (low_ep[0] as u64) << 7;
    l |= (high_ep[0] as u64) << 14;
    l |= (low_ep[1] as u64) << 21;
    l |= (high_ep[1] as u64) << 28;
    l |= (low_ep[2] as u64) << 35;
    l |= (high_ep[2] as u64) << 42;
    l |= (low_ep[3] as u64) << 49;
    l |= (high_ep[3] as u64) << 56;
    l |= (pb[0] as u64) << 63;

    let mut h: u64 = pb[1] as u64;
    h |= ((inv ^ selectors[0]) as u64) << 1;
    h |= ((inv ^ selectors[1]) as u64) << 4;
    h |= ((inv ^ selectors[2]) as u64) << 8;
    h |= ((inv ^ selectors[3]) as u64) << 12;
    h |= ((inv ^ selectors[4]) as u64) << 16;
    h |= ((inv ^ selectors[5]) as u64) << 20;
    h |= ((inv ^ selectors[6]) as u64) << 24;
    h |= ((inv ^ selectors[7]) as u64) << 28;
    h |= ((inv ^ selectors[8]) as u64) << 32;
    h |= ((inv ^ selectors[9]) as u64) << 36;
    h |= ((inv ^ selectors[10]) as u64) << 40;
    h |= ((inv ^ selectors[11]) as u64) << 44;
    h |= ((inv ^ selectors[12]) as u64) << 48;
    h |= ((inv ^ selectors[13]) as u64) << 52;
    h |= ((inv ^ selectors[14]) as u64) << 56;
    h |= ((inv ^ selectors[15]) as u64) << 60;

    out_block[0..8].copy_from_slice(&l.to_le_bytes());
    out_block[8..16].copy_from_slice(&h.to_le_bytes());
}
