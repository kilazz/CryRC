use crate::math::vector::Vec3;
use std::f32::consts::PI;

pub struct ImportanceSampling;

impl ImportanceSampling {
    pub fn hammersley_sequence(sample_idx: usize, sample_count: usize) -> (f32, f32) {
        let mut bits = sample_idx as u32;
        bits = bits.rotate_right(16);
        bits = ((bits & 0x55555555) << 1) | ((bits & 0xAAAAAAAA) >> 1);
        bits = ((bits & 0x33333333) << 2) | ((bits & 0xCCCCCCCC) >> 2);
        bits = ((bits & 0x0F0F0F0F) << 4) | ((bits & 0xF0F0F0F0) >> 4);
        bits = ((bits & 0x00FF00FF) << 8) | ((bits & 0xFF00FF00) >> 8);
        (
            sample_idx as f32 / sample_count as f32,
            bits as f32 * 2.328_306_4e-10,
        )
    }

    pub fn importance_sample_ggx(xi: (f32, f32), roughness: f32, normal: Vec3) -> Vec3 {
        let alpha = roughness * roughness;
        let phi = 2.0 * PI * xi.0;
        let cos_theta = ((1.0 - xi.1) / (1.0 + (alpha * alpha - 1.0) * xi.1))
            .sqrt()
            .clamp(0.0, 1.0);
        let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();

        let h_tangent = Vec3::new(sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta);
        let up = if normal.z.abs() < 0.999 {
            Vec3::new(0.0, 0.0, 1.0)
        } else {
            Vec3::new(1.0, 0.0, 0.0)
        };

        let mut tangent_x = Vec3::new(
            up.y * normal.z - up.z * normal.y,
            up.z * normal.x - up.x * normal.z,
            up.x * normal.y - up.y * normal.x,
        );
        let len_tx = tangent_x.length().max(1e-6);
        tangent_x *= 1.0 / len_tx;

        let tangent_y = Vec3::new(
            normal.y * tangent_x.z - normal.z * tangent_x.y,
            normal.z * tangent_x.x - normal.x * tangent_x.z,
            normal.x * tangent_x.y - normal.y * tangent_x.x,
        );

        Vec3::new(
            tangent_x.x * h_tangent.x + tangent_y.x * h_tangent.y + normal.x * h_tangent.z,
            tangent_x.y * h_tangent.x + tangent_y.y * h_tangent.y + normal.y * h_tangent.z,
            tangent_x.z * h_tangent.x + tangent_y.z * h_tangent.y + normal.z * h_tangent.z,
        )
    }
}
