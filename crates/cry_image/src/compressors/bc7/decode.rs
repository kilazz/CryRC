use super::encode::{BitStream128, expand_quantized};
use super::tables::BC7_MODE_INFO;
use crate::tables::WEIGHTS_F32;
use crate::tables::bc7_masks::{
    ANCHOR_INDEX_2_SUBSET_1, ANCHOR_INDEX_3_SUBSET_1, ANCHOR_INDEX_3_SUBSET_2, get_subset_2,
    get_subset_3,
};

#[inline(always)]
pub fn interpolate_color(c0: [u8; 4], c1: [u8; 4], index: usize, index_bits: usize) -> [u8; 4] {
    let w = WEIGHTS_F32[index_bits][index];
    let inv_w = 1.0 - w;
    [
        (c0[0] as f32 * inv_w + c1[0] as f32 * w).round() as u8,
        (c0[1] as f32 * inv_w + c1[1] as f32 * w).round() as u8,
        (c0[2] as f32 * inv_w + c1[2] as f32 * w).round() as u8,
        (c0[3] as f32 * inv_w + c1[3] as f32 * w).round() as u8,
    ]
}

#[inline(always)]
pub fn apply_rotation(pixel: &mut [u8; 4], rotation: usize) {
    match rotation {
        1 => pixel.swap(0, 3),
        2 => pixel.swap(1, 3),
        3 => pixel.swap(2, 3),
        _ => {}
    }
}

pub fn decompress_bc7_block(block: &[u8; 16], out_rgba: &mut [[u8; 4]; 16]) {
    let bs = BitStream128::from_bytes(block);
    let mut mode: usize = 0;
    while mode < 8 && bs.read_bits(mode as u32, 1) == 0 {
        mode += 1;
    }
    if mode >= 8 {
        out_rgba.fill([0, 0, 0, 0]);
        return;
    }

    match mode {
        6 => {
            let mut pos = 7;
            let r0 = bs.read_bits(pos, 7);
            pos += 7;
            let r1 = bs.read_bits(pos, 7);
            pos += 7;
            let g0 = bs.read_bits(pos, 7);
            pos += 7;
            let g1 = bs.read_bits(pos, 7);
            pos += 7;
            let b0 = bs.read_bits(pos, 7);
            pos += 7;
            let b1 = bs.read_bits(pos, 7);
            pos += 7;
            let a0 = bs.read_bits(pos, 7);
            pos += 7;
            let a1 = bs.read_bits(pos, 7);
            pos += 7;
            let p0 = bs.read_bits(pos, 1);
            pos += 1;
            let p1 = bs.read_bits(pos, 1);
            pos += 1;

            let c0 = [
                expand_quantized(r0, 7, Some(p0)),
                expand_quantized(g0, 7, Some(p0)),
                expand_quantized(b0, 7, Some(p0)),
                expand_quantized(a0, 7, Some(p0)),
            ];
            let c1 = [
                expand_quantized(r1, 7, Some(p1)),
                expand_quantized(g1, 7, Some(p1)),
                expand_quantized(b1, 7, Some(p1)),
                expand_quantized(a1, 7, Some(p1)),
            ];

            for (i, slot) in out_rgba.iter_mut().enumerate() {
                let bits = if i == 0 { 3 } else { 4 };
                let idx = bs.read_bits(pos, bits) as usize;
                pos += bits;
                *slot = interpolate_color(c0, c1, idx, 4);
            }
        }
        _ => {
            let info = &BC7_MODE_INFO[mode];
            let part = if info.partition_bits > 0 {
                bs.read_bits((mode + 1) as u32, info.partition_bits) as usize
            } else {
                0
            };
            let rot = if info.rotation_bits > 0 {
                bs.read_bits((mode + 1) as u32 + info.partition_bits, info.rotation_bits) as usize
            } else {
                0
            };
            let idx_mode = if info.index_selection_bits > 0 {
                bs.read_bits(
                    (mode + 1) as u32 + info.partition_bits + info.rotation_bits,
                    info.index_selection_bits,
                ) as usize
            } else {
                0
            };

            let mut pos = (mode + 1) as u32
                + info.partition_bits
                + info.rotation_bits
                + info.index_selection_bits;

            let mut r = [0u32; 6];
            let mut g = [0u32; 6];
            let mut b = [0u32; 6];
            let mut a = [255u32; 6];

            for val in r.iter_mut().take(info.num_subsets * 2) {
                *val = bs.read_bits(pos, info.color_bits);
                pos += info.color_bits;
            }
            for val in g.iter_mut().take(info.num_subsets * 2) {
                *val = bs.read_bits(pos, info.color_bits);
                pos += info.color_bits;
            }
            for val in b.iter_mut().take(info.num_subsets * 2) {
                *val = bs.read_bits(pos, info.color_bits);
                pos += info.color_bits;
            }
            if info.alpha_bits > 0 {
                for val in a.iter_mut().take(info.num_subsets * 2) {
                    *val = bs.read_bits(pos, info.alpha_bits);
                    pos += info.alpha_bits;
                }
            }

            let mut p_bits = [0u32; 6];
            let total_pbits =
                info.num_subsets * (info.endpoint_pbits as usize * 2 + info.shared_pbits as usize);
            for p in &mut p_bits[..total_pbits] {
                *p = bs.read_bits(pos, 1);
                pos += 1;
            }

            let mut ep = [[[0u8; 4]; 2]; 3];
            for s in 0..info.num_subsets {
                let (p0, p1) = if info.endpoint_pbits > 0 {
                    (Some(p_bits[s * 2]), Some(p_bits[s * 2 + 1]))
                } else if info.shared_pbits > 0 {
                    (Some(p_bits[s]), Some(p_bits[s]))
                } else {
                    (None, None)
                };

                ep[s][0] = [
                    expand_quantized(r[2 * s], info.color_bits, p0),
                    expand_quantized(g[2 * s], info.color_bits, p0),
                    expand_quantized(b[2 * s], info.color_bits, p0),
                    if info.alpha_bits > 0 {
                        expand_quantized(a[2 * s], info.alpha_bits, p0)
                    } else {
                        255
                    },
                ];
                ep[s][1] = [
                    expand_quantized(r[2 * s + 1], info.color_bits, p1),
                    expand_quantized(g[2 * s + 1], info.color_bits, p1),
                    expand_quantized(b[2 * s + 1], info.color_bits, p1),
                    if info.alpha_bits > 0 {
                        expand_quantized(a[2 * s + 1], info.alpha_bits, p1)
                    } else {
                        255
                    },
                ];
            }

            if mode == 4 || mode == 5 {
                let (cb, ab) = if mode == 4 {
                    if idx_mode == 0 { (2, 3) } else { (3, 2) }
                } else {
                    (2, 2)
                };

                let mut c_indices = [0usize; 16];
                for (i, slot) in c_indices.iter_mut().enumerate() {
                    let bits = if i == 0 { cb - 1 } else { cb };
                    *slot = bs.read_bits(pos, bits as u32) as usize;
                    pos += bits as u32;
                }

                let mut a_indices = [0usize; 16];
                for (i, slot) in a_indices.iter_mut().enumerate() {
                    let bits = if i == 0 { ab - 1 } else { ab };
                    *slot = bs.read_bits(pos, bits as u32) as usize;
                    pos += bits as u32;
                }

                for (i, slot) in out_rgba.iter_mut().enumerate() {
                    let rgb = interpolate_color(ep[0][0], ep[0][1], c_indices[i], cb);
                    let alpha = interpolate_color(ep[0][0], ep[0][1], a_indices[i], ab)[3];
                    let mut px = [rgb[0], rgb[1], rgb[2], alpha];
                    apply_rotation(&mut px, rot);
                    *slot = px;
                }
            } else {
                let ib = info.index_bits as usize;
                let anchors = match info.num_subsets {
                    2 => [0, ANCHOR_INDEX_2_SUBSET_1[part], 0],
                    3 => [
                        0,
                        ANCHOR_INDEX_3_SUBSET_1[part],
                        ANCHOR_INDEX_3_SUBSET_2[part],
                    ],
                    _ => [0, 0, 0],
                };

                for (i, slot) in out_rgba.iter_mut().enumerate() {
                    let subset = if info.num_subsets == 2 {
                        get_subset_2(part, i)
                    } else if info.num_subsets == 3 {
                        get_subset_3(part, i)
                    } else {
                        0
                    };

                    let is_anchor = match info.num_subsets {
                        1 => i == 0,
                        2 => i == anchors[0] || i == anchors[1],
                        3 => i == anchors[0] || i == anchors[1] || i == anchors[2],
                        _ => false,
                    };

                    let count = if is_anchor { ib - 1 } else { ib };
                    let idx = bs.read_bits(pos, count as u32) as usize;
                    pos += count as u32;

                    let mut px = interpolate_color(ep[subset][0], ep[subset][1], idx, ib);
                    apply_rotation(&mut px, rot);
                    *slot = px;
                }
            }
        }
    }
}
