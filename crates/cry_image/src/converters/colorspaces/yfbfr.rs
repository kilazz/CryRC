use crate::color_types::ColorRGBAf;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct YFbFr {
    pub fr: f32,
    pub y: f32,
    pub fb: f32,
    pub a: f32,
}

impl YFbFr {
    pub fn from_rgbaf(rgb: ColorRGBAf) -> Self {
        let y_tmp = (5.0 / 16.0) * rgb.r + (3.0 / 8.0) * rgb.g + (5.0 / 16.0) * rgb.b;
        let fb_tmp = -0.5 * rgb.r + 1.0 * rgb.g - 0.5 * rgb.b;
        let fr_tmp = 1.0 * rgb.r - 1.0 * rgb.b;

        Self {
            fr: fr_tmp * 0.5 + 0.5,
            y: y_tmp,
            fb: fb_tmp * 0.5 + 0.5,
            a: rgb.a,
        }
    }
}
