use crate::color_types::{ColorRGBA8, ColorRGBAf};

pub struct Rgbk;

impl Rgbk {
    pub fn compress_squared(color: ColorRGBAf, rgbk_max_value: f32) -> ColorRGBA8 {
        let inv_max = 1.0 / rgbk_max_value;
        let r = (color.r * inv_max).clamp(0.0, 1.0);
        let g = (color.g * inv_max).clamp(0.0, 1.0);
        let b = (color.b * inv_max).clamp(0.0, 1.0);

        let mx = r.max(g).max(b);
        let mut k = (mx.sqrt() * 255.0).clamp(1.0, 255.0) as i32;

        while (k as f32 / 255.0).powi(2) < mx && k < 255 {
            k += 1;
        }

        let k_factor = (k as f32 / 255.0).powi(2);
        let inv_k = if k_factor > 0.0 { 1.0 / k_factor } else { 1.0 };

        ColorRGBA8 {
            b: ((b * inv_k).clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
            g: ((g * inv_k).clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
            r: ((r * inv_k).clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
            a: k as u8,
        }
    }
}
