use crate::filtering::cubemap_gen::CubeMapTopology;
use crate::math::vector::Vec4;
use std::f32::consts::PI;

pub struct CubemapPipeline;

impl CubemapPipeline {
    pub fn convert_equirectangular_to_cubemap(
        src_pixels: &[Vec4],
        src_w: usize,
        src_h: usize,
        face_size: usize,
    ) -> [Vec<Vec4>; 6] {
        let mut faces: [Vec<Vec4>; 6] =
            core::array::from_fn(|_| vec![Vec4::splat(0.0); face_size * face_size]);

        for (face_idx, face) in faces.iter_mut().enumerate() {
            for y in 0..face_size {
                for x in 0..face_size {
                    let dir = CubeMapTopology::texel_coord_to_vect(
                        face_idx, x as f32, y as f32, face_size,
                    );
                    let longitude = 0.5 + dir.x.atan2(-dir.z) / (2.0 * PI);
                    let latitude = 1.0 - (dir.y.clamp(-1.0, 1.0)).acos() / PI;

                    let sx = ((longitude * src_w as f32).floor() as usize).min(src_w - 1);
                    let sy = ((latitude * src_h as f32).floor() as usize).min(src_h - 1);

                    face[y * face_size + x] = src_pixels[sy * src_w + sx];
                }
            }
        }
        faces
    }
}
