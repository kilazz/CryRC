use crate::math::vector::Vec4;
use std::f32::consts::PI;

pub struct Probes;

impl Probes {
    pub fn convert_probe_to_cubemap_strip(
        src: &[Vec4],
        width: usize,
        height: usize,
    ) -> (Vec<Vec4>, usize, usize) {
        if width == height * 2 {
            let dst_square = (height / 2).next_power_of_two();
            let dst_w = dst_square * 6;
            let dst_h = dst_square;
            let mut out = vec![Vec4::splat(0.0); dst_w * dst_h];
            let app = 1.0 / dst_h as f32;

            for y in 0..dst_h {
                for x in 0..dst_w {
                    let side = x / dst_h;
                    let local_x = x % dst_h;
                    let cy = (y as f32 * 2.0 - dst_h as f32 + 1.0) * app;
                    let cp = (local_x as f32 * 2.0 - dst_h as f32 + 1.0) * app;

                    let mut fvec = match side {
                        0 => crate::math::vector::Vec3::new(1.0, cy, cp),
                        1 => crate::math::vector::Vec3::new(-1.0, cy, -cp),
                        2 => crate::math::vector::Vec3::new(cp, -1.0, -cy),
                        3 => crate::math::vector::Vec3::new(cp, 1.0, cy),
                        4 => crate::math::vector::Vec3::new(cp, cy, -1.0),
                        _ => crate::math::vector::Vec3::new(-cp, cy, 1.0),
                    };
                    std::mem::swap(&mut fvec.y, &mut fvec.z);
                    fvec = fvec.normalize();

                    let longitude = width as f32 * (0.5 + fvec.x.atan2(-fvec.z) / (2.0 * PI));
                    let latitude = height as f32 * (1.0 - (fvec.y.clamp(-1.0, 1.0)).acos() / PI);
                    let sx = (longitude.floor() as usize).min(width - 1);
                    let sy = (latitude.floor() as usize).min(height - 1);

                    out[y * dst_w + x] = src[sy * width + sx];
                }
            }
            (out, dst_w, dst_h)
        } else {
            (src.to_vec(), width, height)
        }
    }
}
