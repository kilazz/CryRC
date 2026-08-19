use crate::color_types::ColorRGBAf;
use cry_core::math::{Matrix33, Vec3};

pub struct CombineNormals;

impl CombineNormals {
    pub fn add_normal_map(
        base_pixels: &[ColorRGBAf],
        base_w: usize,
        base_h: usize,
        bump_pixels: &[ColorRGBAf],
        bump_w: usize,
        bump_h: usize,
    ) -> Vec<ColorRGBAf> {
        let mut output = vec![ColorRGBAf::default(); base_w * base_h];
        for y in 0..base_h {
            let by = y % bump_h;
            for x in 0..base_w {
                let bx = x % bump_w;
                let src_p = base_pixels[y * base_w + x];
                let bump_p = bump_pixels[by * bump_w + bx];

                let v_bump = Vec3::new(
                    bump_p.r * 2.0 - 1.0,
                    bump_p.g * 2.0 - 1.0,
                    bump_p.b * 2.0 - 1.0,
                )
                .normalized();
                let mut v_normal = Vec3::new(
                    src_p.r * 2.0 - 1.0,
                    src_p.g * 2.0 - 1.0,
                    src_p.b * 2.0 - 1.0,
                );

                let mut m_transform = Matrix33::identity();
                m_transform.set_rotation_v0_v1(Vec3::new(0.0, 0.0, 1.0), v_bump);
                v_normal = m_transform.transform_vector(v_normal);

                output[y * base_w + x] = ColorRGBAf::new(
                    (v_normal.x * 0.5 + 0.5).clamp(0.0, 1.0),
                    (v_normal.y * 0.5 + 0.5).clamp(0.0, 1.0),
                    (v_normal.z * 0.5 + 0.5).clamp(0.0, 1.0),
                    0.0,
                );
            }
        }
        output
    }
}
