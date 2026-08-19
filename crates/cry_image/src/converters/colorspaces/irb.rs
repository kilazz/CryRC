use crate::color_types::ColorRGBAf;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Irb {
    pub i: f32,
    pub r: f32,
    pub b: f32,
    pub a: f32,
}

impl Irb {
    pub fn from_rgbaf(rgb: ColorRGBAf) -> Self {
        let i_tmp = rgb.g + rgb.r.max(rgb.b);
        let r_tmp = if i_tmp != 0.0 { rgb.r / i_tmp } else { 0.0 };
        let b_tmp = if i_tmp != 0.0 { rgb.b / i_tmp } else { 0.0 };

        Self {
            i: (i_tmp * 0.5).clamp(0.0, 1.0),
            r: r_tmp.clamp(0.0, 1.0),
            b: b_tmp.clamp(0.0, 1.0),
            a: rgb.a,
        }
    }
}
