use crate::color_types::ColorRGBAf;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct YCbCr {
    pub cr: f32,
    pub y: f32,
    pub cb: f32,
    pub a: f32,
}

impl YCbCr {
    pub fn from_rgbaf(rgb: ColorRGBAf) -> Self {
        let y_tmp = 0.299000 * rgb.r + 0.587000 * rgb.g + 0.114000 * rgb.b;
        let cb_tmp = -0.168736 * rgb.r - 0.331264 * rgb.g + 0.500000 * rgb.b;
        let cr_tmp = 0.500000 * rgb.r - 0.418688 * rgb.g - 0.081312 * rgb.b;

        Self {
            cr: cr_tmp * 0.5 + 0.5,
            y: y_tmp,
            cb: cb_tmp * 0.5 + 0.5,
            a: rgb.a,
        }
    }
}
