use cry_core::math::Vec3;

#[derive(Debug, Clone, Copy, Default)]
pub struct Tangent {
    pub tangent: Vec3,
    pub bitangent: Vec3,
    pub handedness: f32,
}

pub struct TangentSpaceCalculation;

impl TangentSpaceCalculation {
    pub fn calculate_tangents(
        positions: &[Vec3],
        normals: &[Vec3],
        uvs: &[[f32; 2]],
        indices: &[u32],
    ) -> Result<Vec<Tangent>, String> {
        let vertex_count = positions.len();
        let num_triangles = indices.len() / 3;

        let mut tan1 = vec![Vec3::ZERO; vertex_count];
        let mut tan2 = vec![Vec3::ZERO; vertex_count];
        let mut tangents = vec![Tangent::default(); vertex_count];

        for i in 0..num_triangles {
            let i1 = indices[i * 3] as usize;
            let i2 = indices[i * 3 + 1] as usize;
            let i3 = indices[i * 3 + 2] as usize;

            let v1 = positions[i1];
            let v2 = positions[i2];
            let v3 = positions[i3];

            let w1 = uvs[i1];
            let w2 = uvs[i2];
            let w3 = uvs[i3];

            let x1 = v2.x - v1.x;
            let x2 = v3.x - v1.x;
            let y1 = v2.y - v1.y;
            let y2 = v3.y - v1.y;
            let z1 = v2.z - v1.z;
            let z2 = v3.z - v1.z;

            let s1 = w2[0] - w1[0];
            let s2 = w3[0] - w1[0];
            let t1 = w2[1] - w1[1];
            let t2 = w3[1] - w1[1];

            let r = s1 * t2 - s2 * t1;
            let inv_r = if r.abs() > 1e-6 { 1.0 / r } else { 1.0 };

            let sdir = Vec3::new(
                (t2 * x1 - t1 * x2) * inv_r,
                (t2 * y1 - t1 * y2) * inv_r,
                (t2 * z1 - t1 * z2) * inv_r,
            );
            let tdir = Vec3::new(
                (s1 * x2 - s2 * x1) * inv_r,
                (s1 * y2 - s2 * y1) * inv_r,
                (s1 * z2 - s2 * z1) * inv_r,
            );

            tan1[i1] = Vec3::new(
                tan1[i1].x + sdir.x,
                tan1[i1].y + sdir.y,
                tan1[i1].z + sdir.z,
            );
            tan1[i2] = Vec3::new(
                tan1[i2].x + sdir.x,
                tan1[i2].y + sdir.y,
                tan1[i2].z + sdir.z,
            );
            tan1[i3] = Vec3::new(
                tan1[i3].x + sdir.x,
                tan1[i3].y + sdir.y,
                tan1[i3].z + sdir.z,
            );

            tan2[i1] = Vec3::new(
                tan2[i1].x + tdir.x,
                tan2[i1].y + tdir.y,
                tan2[i1].z + tdir.z,
            );
            tan2[i2] = Vec3::new(
                tan2[i2].x + tdir.x,
                tan2[i2].y + tdir.y,
                tan2[i2].z + tdir.z,
            );
            tan2[i3] = Vec3::new(
                tan2[i3].x + tdir.x,
                tan2[i3].y + tdir.y,
                tan2[i3].z + tdir.z,
            );
        }

        for a in 0..vertex_count {
            let n = normals[a];
            let t = tan1[a];

            let n_dot_t = n.dot(t);
            let tangent = Vec3::new(
                t.x - n.x * n_dot_t,
                t.y - n.y * n_dot_t,
                t.z - n.z * n_dot_t,
            )
            .normalized();

            let cross_n_t = n.cross(t);
            let handedness = if cross_n_t.dot(tan2[a]) < 0.0 {
                -1.0
            } else {
                1.0
            };
            let cross_tangent = n.cross(tangent);
            let bitangent = Vec3::new(
                cross_tangent.x * handedness,
                cross_tangent.y * handedness,
                cross_tangent.z * handedness,
            );

            tangents[a] = Tangent {
                tangent,
                bitangent,
                handedness,
            };
        }

        Ok(tangents)
    }
}
