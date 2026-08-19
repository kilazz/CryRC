use crate::color_types::ColorRGBAf;

/// Standard Direct3D/OpenGL DXGI_FORMAT_R9G9B9E5_SHAREDEXP (9 bits R, 9 bits G, 9 bits B, 5 bits shared exponent)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct Rgb9E5 {
    pub raw: u32,
}

impl Rgb9E5 {
    pub const MAX_RGB9E5: f32 = 65408.0;
    pub const EXP_BIAS: i32 = 15;
    pub const MANTISSA_BITS: i32 = 9;

    #[inline(always)]
    pub const fn new(raw: u32) -> Self {
        Self { raw }
    }

    /// Encodes a floating-point RGBA color into 32-bit R9G9B9E5 format.
    pub fn from_rgbaf(color: ColorRGBAf) -> Self {
        let rc = color.r.clamp(0.0, Self::MAX_RGB9E5);
        let gc = color.g.clamp(0.0, Self::MAX_RGB9E5);
        let bc = color.b.clamp(0.0, Self::MAX_RGB9E5);

        let max_c = rc.max(gc).max(bc);
        if max_c <= 1e-6 {
            return Self { raw: 0 };
        }

        let exp_shared = (max_c.log2().floor() as i32 + 1 - (-Self::EXP_BIAS)).clamp(0, 31);
        let scale = 2.0f32.powi(exp_shared - Self::EXP_BIAS - Self::MANTISSA_BITS);
        let inv_scale = if scale > 0.0 { 1.0 / scale } else { 1.0 };

        let rm = ((rc * inv_scale + 0.5).floor() as u32).min(511);
        let gm = ((gc * inv_scale + 0.5).floor() as u32).min(511);
        let bm = ((bc * inv_scale + 0.5).floor() as u32).min(511);

        let raw = rm | (gm << 9) | (bm << 18) | ((exp_shared as u32) << 27);
        Self { raw }
    }

    /// Decodes R9G9B9E5 back to float RGBA color.
    #[inline(always)]
    pub fn to_rgbaf(&self) -> ColorRGBAf {
        let rm = self.raw & 0x1FF;
        let gm = (self.raw >> 9) & 0x1FF;
        let bm = (self.raw >> 18) & 0x1FF;
        let exp = ((self.raw >> 27) & 0x1F) as i32;

        let scale = 2.0f32.powi(exp - Self::EXP_BIAS - Self::MANTISSA_BITS);
        ColorRGBAf::new(rm as f32 * scale, gm as f32 * scale, bm as f32 * scale, 1.0)
    }
}
