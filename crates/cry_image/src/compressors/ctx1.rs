use crate::math::normal::complement_z;
use crate::math::pca::{
    compute_weighted_covariance3, estimate_principle_component, get_principle_projection_vec3,
};
use crate::math::vector::Vec3;

#[inline(always)]
pub fn float_to_88(color: &Vec3) -> u16 {
    let r = (color.x * 255.0).round().clamp(0.0, 255.0) as u16;
    let g = (color.y * 255.0).round().clamp(0.0, 255.0) as u16;
    (r << 8) | g
}

#[inline(always)]
pub fn unpack_88(val: u16) -> [u8; 2] {
    [((val >> 8) & 0xFF) as u8, (val & 0xFF) as u8]
}

pub fn write_bitone_block_4(start: &Vec3, end: &Vec3, indices: &[u8; 16], block: &mut [u8; 8]) {
    let mut a = float_to_88(start);
    let mut b = float_to_88(end);
    let mut remapped = *indices;

    if a < b {
        std::mem::swap(&mut a, &mut b);
        for idx in &mut remapped {
            *idx ^= 1;
        }
    } else if a == b {
        remapped.fill(0);
    }

    block[0] = (a & 0xFF) as u8;
    block[1] = (a >> 8) as u8;
    block[2] = (b & 0xFF) as u8;
    block[3] = (b >> 8) as u8;

    for i in 0..4 {
        let offset = i * 4;
        block[4 + i] = (remapped[offset] & 0x03)
            | ((remapped[offset + 1] & 0x03) << 2)
            | ((remapped[offset + 2] & 0x03) << 4)
            | ((remapped[offset + 3] & 0x03) << 6);
    }
}

pub fn decompress_bitones_ctx1(block: &[u8; 8], out_rgba: &mut [[u8; 4]; 16]) {
    let a_val = (block[0] as u16) | ((block[1] as u16) << 8);
    let b_val = (block[2] as u16) | ((block[3] as u16) << 8);

    let [r0, g0] = unpack_88(a_val);
    let [r1, g1] = unpack_88(b_val);

    let c0 = [r0 as f32, g0 as f32];
    let c1 = [r1 as f32, g1 as f32];

    let mut palette = [[0u8; 4]; 4];
    palette[0] = [r0, g0, 0, 255];
    palette[1] = [r1, g1, 0, 255];
    palette[2] = [
        ((2.0 * c0[0] + c1[0]) / 3.0).round() as u8,
        ((2.0 * c0[1] + c1[1]) / 3.0).round() as u8,
        0,
        255,
    ];
    palette[3] = [
        ((c0[0] + 2.0 * c1[0]) / 3.0).round() as u8,
        ((c0[1] + 2.0 * c1[1]) / 3.0).round() as u8,
        0,
        255,
    ];

    for i in 0..4 {
        let byte = block[4 + i];
        for j in 0..4 {
            let idx = (byte >> (j * 2)) & 0x03;
            out_rgba[i * 4 + j] = palette[idx as usize];
        }
    }
}

pub fn decompress_normals_ctx1(block: &[u8; 8], out_xyzd: &mut [[u8; 4]; 16]) {
    let a_val = (block[0] as u16) | ((block[1] as u16) << 8);
    let b_val = (block[2] as u16) | ((block[3] as u16) << 8);

    let [r0, g0] = unpack_88(a_val);
    let [r1, g1] = unpack_88(b_val);

    let c0 = [r0 as f32, g0 as f32];
    let c1 = [r1 as f32, g1 as f32];

    let mut palette_rg = [[0.0f32; 2]; 4];
    palette_rg[0] = c0;
    palette_rg[1] = c1;
    palette_rg[2] = [(2.0 * c0[0] + c1[0]) / 3.0, (2.0 * c0[1] + c1[1]) / 3.0];
    palette_rg[3] = [(c0[0] + 2.0 * c1[0]) / 3.0, (c0[1] + 2.0 * c1[1]) / 3.0];

    for i in 0..4 {
        let byte = block[4 + i];
        for j in 0..4 {
            let idx = (byte >> (j * 2)) & 0x03;
            let rg = palette_rg[idx as usize];

            let x = (rg[0] / 255.0) * 2.0 - 1.0;
            let y = (rg[1] / 255.0) * 2.0 - 1.0;
            let normal = complement_z(Vec3::new(x, y, 0.0));

            out_xyzd[i * 4 + j] = [
                ((normal.x * 0.5 + 0.5) * 255.0).round() as u8,
                ((normal.y * 0.5 + 0.5) * 255.0).round() as u8,
                ((normal.z * 0.5 + 0.5) * 255.0).round() as u8,
                255,
            ];
        }
    }
}

pub fn compress_ctx1_block(rgba: &[[u8; 4]; 16], mask: u16, _flags: u32, out_block: &mut [u8; 8]) {
    let mut count = 0;
    let mut points = [Vec3::splat(0.0); 16];

    for (i, p) in rgba.iter().enumerate() {
        if (mask & (1 << i)) == 0 {
            continue;
        }
        points[count] = Vec3::new(p[0] as f32 / 255.0, p[1] as f32 / 255.0, 0.0);
        count += 1;
    }

    if count <= 1 {
        let p = if count == 1 {
            points[0]
        } else {
            Vec3::splat(0.0)
        };
        write_bitone_block_4(&p, &p, &[0; 16], out_block);
        return;
    }

    let weights = [1.0f32; 16];
    let (cov, centroid) = compute_weighted_covariance3(&points[..count], &weights[..count]);
    let principle = estimate_principle_component(&cov);
    let (s_proj, e_proj) = get_principle_projection_vec3(&principle, &centroid, &points[..count]);

    let codes = [
        s_proj,
        e_proj,
        s_proj * (2.0 / 3.0) + e_proj * (1.0 / 3.0),
        s_proj * (1.0 / 3.0) + e_proj * (2.0 / 3.0),
    ];

    let mut indices = [0u8; 16];
    for (i, p) in rgba.iter().enumerate() {
        if (mask & (1 << i)) == 0 {
            continue;
        }
        let pt = Vec3::new(p[0] as f32 / 255.0, p[1] as f32 / 255.0, 0.0);
        let mut min_d = f32::MAX;
        let mut best_idx = 0u8;
        for (j, code) in codes.iter().enumerate() {
            let dist = (*code - pt).length_squared();
            if dist < min_d {
                min_d = dist;
                best_idx = j as u8;
            }
        }
        indices[i] = best_idx;
    }

    write_bitone_block_4(&s_proj, &e_proj, &indices, out_block);
}

#[inline(always)]
pub fn decompress_ctx1_block(block: &[u8; 8], out_rgba: &mut [[u8; 4]; 16]) {
    decompress_bitones_ctx1(block, out_rgba);
}

#[inline(always)]
pub fn decompress_ctx1_normals_block(block: &[u8; 8], out_xyzd: &mut [[u8; 4]; 16]) {
    decompress_normals_ctx1(block, out_xyzd);
}
