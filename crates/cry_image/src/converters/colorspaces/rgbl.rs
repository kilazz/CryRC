use crate::color_types::ColorRGBAf;

pub struct Rgbl;

impl Rgbl {
    #[inline(always)]
    pub fn get_luminance_f32(r: f32, g: f32, b: f32) -> f32 {
        r * 0.30 + g * 0.59 + b * 0.11
    }

    pub fn populate_luminance_in_alpha(pixels: &mut [ColorRGBAf]) {
        for p in pixels.iter_mut() {
            p.a = Self::get_luminance_f32(p.r, p.g, p.b).clamp(0.0, 1.0);
        }
    }
}
