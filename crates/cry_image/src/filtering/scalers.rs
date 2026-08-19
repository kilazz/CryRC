use crate::math::vector::Vec4;

pub struct Scalers;

impl Scalers {
    pub fn upscale_pow2_twice_horizontally(
        src: &[Vec4],
        width: usize,
        height: usize,
    ) -> (Vec<Vec4>, usize, usize) {
        let new_w = width * 2;
        let mut dst = vec![Vec4::splat(0.0); new_w * height];

        for y in 0..height {
            for x in 0..width {
                let p = src[y * width + x];
                dst[y * new_w + (x * 2)] = p;
                dst[y * new_w + (x * 2 + 1)] = p;
            }
        }
        (dst, new_w, height)
    }
}
