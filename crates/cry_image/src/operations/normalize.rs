use crate::color_types::ColorRGBAf;
use cry_core::math::{Vec3, Vec4};

pub struct RangeNormalizer;

impl RangeNormalizer {
    pub fn normalize_image_range(pixels: &mut [ColorRGBAf]) -> (Vec4, Vec4) {
        let mut min_c = Vec4::new(f32::MAX, f32::MAX, f32::MAX, f32::MAX);
        let mut max_c = Vec4::new(-f32::MAX, -f32::MAX, -f32::MAX, -f32::MAX);

        for p in pixels.iter() {
            min_c.x = min_c.x.min(p.r);
            max_c.x = max_c.x.max(p.r);
            min_c.y = min_c.y.min(p.g);
            max_c.y = max_c.y.max(p.g);
            min_c.z = min_c.z.min(p.b);
            max_c.z = max_c.z.max(p.b);
            min_c.w = min_c.w.min(p.a);
            max_c.w = max_c.w.max(p.a);
        }

        let scale_r = (max_c.x - min_c.x).max(1e-5);
        let scale_g = (max_c.y - min_c.y).max(1e-5);
        let scale_b = (max_c.z - min_c.z).max(1e-5);

        for p in pixels.iter_mut() {
            p.r = (p.r - min_c.x) / scale_r;
            p.g = (p.g - min_c.y) / scale_g;
            p.b = (p.b - min_c.z) / scale_b;
        }

        (min_c, max_c)
    }

    pub fn normalize_vectors(pixels: &mut [ColorRGBAf]) {
        for p in pixels.iter_mut() {
            let mut v = Vec3::new(p.r * 2.0 - 1.0, p.g * 2.0 - 1.0, p.b * 2.0 - 1.0);
            let len = v.len();
            if len > 1e-6 {
                let inv_len = 1.0 / len;
                v.x *= inv_len;
                v.y *= inv_len;
                v.z *= inv_len;
            } else {
                v = Vec3::new(0.0, 0.0, 1.0);
            }
            p.r = v.x * 0.5 + 0.5;
            p.g = v.y * 0.5 + 0.5;
            p.b = v.z * 0.5 + 0.5;
        }
    }
}
