use crate::compressors::bc3::write_alpha_block_bc3;
use crate::compressors::bc4::*;
use crate::math::normal::{DEVIANCE_MAX, complement_z};
use crate::math::vector::Vec3;

pub fn compress_bc5(red: &[u8; 16], green: &[u8; 16], mask: u16, flags: u32, block: &mut [u8; 16]) {
    let (block_r, block_g) = block.split_at_mut(8);
    compress_bc4(red, mask, flags, block_r.try_into().unwrap());
    compress_bc4(green, mask, flags, block_g.try_into().unwrap());
}

pub fn compress_bc5_signed(
    red: &[i8; 16],
    green: &[i8; 16],
    mask: u16,
    flags: u32,
    block: &mut [u8; 16],
) {
    let (block_r, block_g) = block.split_at_mut(8);
    compress_bc4_signed(red, mask, flags, block_r.try_into().unwrap());
    compress_bc4_signed(green, mask, flags, block_g.try_into().unwrap());
}

pub fn compress_bc5_u16(red: &[u16; 16], green: &[u16; 16], mask: u16, block: &mut [u8; 16]) {
    let (block_r, block_g) = block.split_at_mut(8);
    compress_bc4_u16(red, mask, block_r.try_into().unwrap());
    compress_bc4_u16(green, mask, block_g.try_into().unwrap());
}

pub fn compress_bc5_i16(red: &[i16; 16], green: &[i16; 16], mask: u16, block: &mut [u8; 16]) {
    let (block_r, block_g) = block.split_at_mut(8);
    compress_bc4_i16(red, mask, block_r.try_into().unwrap());
    compress_bc4_i16(green, mask, block_g.try_into().unwrap());
}

pub fn compress_bc5_normals(
    red: &[u8; 16],
    green: &[u8; 16],
    mask: u16,
    _flags: u32,
    block_x: &mut [u8; 8],
    block_y: &mut [u8; 8],
) {
    let mut min_x = 255u8;
    let mut max_x = 0u8;
    let mut min_y = 255u8;
    let mut max_y = 0u8;

    for i in 0..16 {
        if (mask & (1 << i)) == 0 {
            continue;
        }
        min_x = min_x.min(red[i]);
        max_x = max_x.max(red[i]);
        min_y = min_y.min(green[i]);
        max_y = max_y.max(green[i]);
    }

    let codes_x = build_codebook_8_u16((max_x as u16) * 257, (min_x as u16) * 257);
    let codes_y = build_codebook_8_u16((max_y as u16) * 257, (min_y as u16) * 257);

    let mut ind_x = [0u8; 16];
    let mut ind_y = [0u8; 16];

    for i in 0..16 {
        if (mask & (1 << i)) == 0 {
            continue;
        }
        let rx = (red[i] as f32 / 255.0) * 2.0 - 1.0;
        let gy = (green[i] as f32 / 255.0) * 2.0 - 1.0;
        let normal = complement_z(Vec3::new(rx, gy, 0.0));

        let mut max_cos = DEVIANCE_MAX;
        let mut best_x = 0u8;
        let mut best_y = 0u8;

        for (jx, &cx) in codes_x.iter().enumerate() {
            let xf = ((cx >> 8) as f32 / 255.0) * 2.0 - 1.0;
            for (jy, &cy) in codes_y.iter().enumerate() {
                let yf = ((cy >> 8) as f32 / 255.0) * 2.0 - 1.0;
                let cand_normal = complement_z(Vec3::new(xf, yf, 0.0));
                let dot = normal.dot(&cand_normal);
                if dot > max_cos {
                    max_cos = dot;
                    best_x = jx as u8;
                    best_y = jy as u8;
                }
            }
        }

        ind_x[i] = best_x;
        ind_y[i] = best_y;
    }

    write_alpha_block_bc3(max_x, min_x, &ind_x, block_x);
    write_alpha_block_bc3(max_y, min_y, &ind_y, block_y);
}

pub fn compress_bc5_normals_signed(
    red: &[i8; 16],
    green: &[i8; 16],
    mask: u16,
    flags: u32,
    block_x: &mut [u8; 8],
    block_y: &mut [u8; 8],
) {
    compress_bc4_signed(red, mask, flags, block_x);
    compress_bc4_signed(green, mask, flags, block_y);
}

pub fn decompress_bc5(block: &[u8; 16], out_red: &mut [u8; 16], out_green: &mut [u8; 16]) {
    let (block_r, block_g) = block.split_at(8);
    decompress_bc4(block_r.try_into().unwrap(), out_red);
    decompress_bc4(block_g.try_into().unwrap(), out_green);
}

pub fn decompress_bc5_signed(block: &[u8; 16], out_red: &mut [i8; 16], out_green: &mut [i8; 16]) {
    let (block_r, block_g) = block.split_at(8);
    decompress_bc4_signed(block_r.try_into().unwrap(), out_red);
    decompress_bc4_signed(block_g.try_into().unwrap(), out_green);
}

pub fn decompress_bc5_normals(block_x: &[u8; 8], block_y: &[u8; 8], out_xyzd: &mut [[u8; 4]; 16]) {
    let mut red = [0u8; 16];
    let mut green = [0u8; 16];
    decompress_bc4(block_x, &mut red);
    decompress_bc4(block_y, &mut green);

    for i in 0..16 {
        let x = (red[i] as f32 / 255.0) * 2.0 - 1.0;
        let y = (green[i] as f32 / 255.0) * 2.0 - 1.0;
        let normal = complement_z(Vec3::new(x, y, 0.0));

        out_xyzd[i] = [
            red[i],
            green[i],
            ((normal.z * 0.5 + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8,
            255,
        ];
    }
}

pub fn decompress_bc5_normals_signed(
    block_x: &[u8; 8],
    block_y: &[u8; 8],
    out_xyzd: &mut [[f32; 4]; 16],
) {
    let mut red = [0i8; 16];
    let mut green = [0i8; 16];
    decompress_bc4_signed(block_x, &mut red);
    decompress_bc4_signed(block_y, &mut green);

    for i in 0..16 {
        let x = (red[i] as f32 / 127.0).clamp(-1.0, 1.0);
        let y = (green[i] as f32 / 127.0).clamp(-1.0, 1.0);
        let normal = complement_z(Vec3::new(x, y, 0.0));
        out_xyzd[i] = [normal.x, normal.y, normal.z, 1.0];
    }
}

pub fn decompress_bc5_u16(block: &[u8; 16], out_red: &mut [u16; 16], out_green: &mut [u16; 16]) {
    let (block_r, block_g) = block.split_at(8);
    decompress_bc4_u16(block_r.try_into().unwrap(), out_red);
    decompress_bc4_u16(block_g.try_into().unwrap(), out_green);
}

pub fn decompress_bc5_i16(block: &[u8; 16], out_red: &mut [i16; 16], out_green: &mut [i16; 16]) {
    let (block_r, block_g) = block.split_at(8);
    decompress_bc4_i16(block_r.try_into().unwrap(), out_red);
    decompress_bc4_i16(block_g.try_into().unwrap(), out_green);
}
