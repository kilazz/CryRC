use crate::tables::srgb::{linear_to_srgb, srgb_to_linear};

pub fn premultiply_alpha(rgba: &mut [u8], is_srgb: bool) {
    if is_srgb {
        for px in rgba.chunks_exact_mut(4) {
            let a = px[3] as f32 / 255.0;
            if a == 0.0 {
                px[0] = 0;
                px[1] = 0;
                px[2] = 0;
            } else if a < 1.0 {
                px[0] = linear_to_srgb(srgb_to_linear(px[0]) * a);
                px[1] = linear_to_srgb(srgb_to_linear(px[1]) * a);
                px[2] = linear_to_srgb(srgb_to_linear(px[2]) * a);
            }
        }
    } else {
        for px in rgba.chunks_exact_mut(4) {
            let a = px[3] as u32;
            if a == 0 {
                px[0] = 0;
                px[1] = 0;
                px[2] = 0;
            } else if a < 255 {
                px[0] = ((px[0] as u32 * a + 127) / 255) as u8;
                px[1] = ((px[1] as u32 * a + 127) / 255) as u8;
                px[2] = ((px[2] as u32 * a + 127) / 255) as u8;
            }
        }
    }
}

pub fn demultiply_alpha(rgba: &mut [u8], is_srgb: bool) {
    if is_srgb {
        for px in rgba.chunks_exact_mut(4) {
            let a = px[3] as f32 / 255.0;
            if a > 0.0 && a < 1.0 {
                let inv_a = 1.0 / a;
                px[0] = linear_to_srgb((srgb_to_linear(px[0]) * inv_a).clamp(0.0, 1.0));
                px[1] = linear_to_srgb((srgb_to_linear(px[1]) * inv_a).clamp(0.0, 1.0));
                px[2] = linear_to_srgb((srgb_to_linear(px[2]) * inv_a).clamp(0.0, 1.0));
            }
        }
    } else {
        for px in rgba.chunks_exact_mut(4) {
            let a = px[3] as u32;
            if a > 0 && a < 255 {
                px[0] = ((px[0] as u32 * 255 + (a / 2)) / a).min(255) as u8;
                px[1] = ((px[1] as u32 * 255 + (a / 2)) / a).min(255) as u8;
                px[2] = ((px[2] as u32 * 255 + (a / 2)) / a).min(255) as u8;
            }
        }
    }
}
