use crate::math::vector::Vec4;

pub fn dither_image_rgba(
    pixels: &[u8],
    quantized: &[u8],
    width: usize,
    height: usize,
    out: &mut [u8],
) {
    assert_eq!(pixels.len(), width * height * 4);
    assert_eq!(quantized.len(), width * height * 4);
    assert_eq!(out.len(), width * height * 4);

    if width == 0 || height == 0 {
        return;
    }

    let mut current_err = vec![Vec4::splat(0.0); width + 2];
    let mut next_err = vec![Vec4::splat(0.0); width + 2];

    for y in 0..height {
        next_err.fill(Vec4::splat(0.0));

        for x in 0..width {
            let idx = (y * width + x) * 4;
            let err_idx = x + 1;

            let p_orig = Vec4::new(
                pixels[idx] as f32 + current_err[err_idx].x,
                pixels[idx + 1] as f32 + current_err[err_idx].y,
                pixels[idx + 2] as f32 + current_err[err_idx].z,
                pixels[idx + 3] as f32 + current_err[err_idx].w,
            );

            let p_quant = Vec4::new(
                quantized[idx] as f32,
                quantized[idx + 1] as f32,
                quantized[idx + 2] as f32,
                quantized[idx + 3] as f32,
            );

            out[idx] = p_orig.x.round().clamp(0.0, 255.0) as u8;
            out[idx + 1] = p_orig.y.round().clamp(0.0, 255.0) as u8;
            out[idx + 2] = p_orig.z.round().clamp(0.0, 255.0) as u8;
            out[idx + 3] = p_orig.w.round().clamp(0.0, 255.0) as u8;

            let diff = p_orig - p_quant;

            current_err[err_idx + 1] += diff * (7.0 / 16.0);
            next_err[err_idx - 1] += diff * (3.0 / 16.0);
            next_err[err_idx] += diff * (5.0 / 16.0);
            next_err[err_idx + 1] += diff * (1.0 / 16.0);
        }

        std::mem::swap(&mut current_err, &mut next_err);
    }
}

pub fn dither_block_rgba(
    pixels: &[[u8; 4]; 16],
    quantized: &[[u8; 4]; 16],
    out_dithered: &mut [[u8; 4]; 16],
) {
    let mut error = [Vec4::splat(0.0); 16];

    for y in 0..4 {
        for x in 0..4 {
            let i = y * 4 + x;

            let orig = Vec4::new(
                pixels[i][0] as f32 + error[i].x,
                pixels[i][1] as f32 + error[i].y,
                pixels[i][2] as f32 + error[i].z,
                pixels[i][3] as f32 + error[i].w,
            );

            let q = Vec4::new(
                quantized[i][0] as f32,
                quantized[i][1] as f32,
                quantized[i][2] as f32,
                quantized[i][3] as f32,
            );

            out_dithered[i] = [
                orig.x.round().clamp(0.0, 255.0) as u8,
                orig.y.round().clamp(0.0, 255.0) as u8,
                orig.z.round().clamp(0.0, 255.0) as u8,
                orig.w.round().clamp(0.0, 255.0) as u8,
            ];

            let diff = orig - q;

            if x < 3 {
                error[i + 1] += diff * (7.0 / 16.0);
            }
            if y < 3 {
                if x > 0 {
                    error[i + 3] += diff * (3.0 / 16.0);
                }
                error[i + 4] += diff * (5.0 / 16.0);
                if x < 3 {
                    error[i + 5] += diff * (1.0 / 16.0);
                }
            }
        }
    }
}
