use crate::math::vector::Vec3;

pub const FACE_2D_MAPPING: [[[f32; 3]; 3]; 6] = [
    [[0.0, 0.0, -1.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]],
    [[0.0, 0.0, 1.0], [0.0, -1.0, 0.0], [-1.0, 0.0, 0.0]],
    [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]],
    [[1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, -1.0, 0.0]],
    [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]],
    [[-1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]],
];

pub struct CubeMapTopology;

impl CubeMapTopology {
    pub fn texel_coord_to_vect(face_idx: usize, u: f32, v: f32, size: usize) -> Vec3 {
        let nvc_u = (2.0 * (u + 0.5) / size as f32) - 1.0;
        let nvc_v = (2.0 * (v + 0.5) / size as f32) - 1.0;

        let u_dir = FACE_2D_MAPPING[face_idx][0];
        let v_dir = FACE_2D_MAPPING[face_idx][1];
        let face_axis = FACE_2D_MAPPING[face_idx][2];

        let dir = Vec3::new(
            u_dir[0] * nvc_u + v_dir[0] * nvc_v + face_axis[0],
            u_dir[1] * nvc_u + v_dir[1] * nvc_v + face_axis[1],
            u_dir[2] * nvc_u + v_dir[2] * nvc_v + face_axis[2],
        );
        dir.normalize()
    }

    pub fn vect_to_texel_coord(dir: Vec3, size: usize) -> (usize, usize, usize) {
        let abs_x = dir.x.abs();
        let abs_y = dir.y.abs();
        let abs_z = dir.z.abs();

        let (face_idx, max_axis) = if abs_x >= abs_y && abs_x >= abs_z {
            (if dir.x >= 0.0 { 0 } else { 1 }, abs_x)
        } else if abs_y >= abs_x && abs_y >= abs_z {
            (if dir.y >= 0.0 { 2 } else { 3 }, abs_y)
        } else {
            (if dir.z >= 0.0 { 4 } else { 5 }, abs_z)
        };

        let on_face = Vec3::new(dir.x / max_axis, dir.y / max_axis, dir.z / max_axis);
        let u_dir = FACE_2D_MAPPING[face_idx][0];
        let v_dir = FACE_2D_MAPPING[face_idx][1];

        let nvc_u = on_face.x * u_dir[0] + on_face.y * u_dir[1] + on_face.z * u_dir[2];
        let nvc_v = on_face.x * v_dir[0] + on_face.y * v_dir[1] + on_face.z * v_dir[2];

        let u = ((size as f32 * 0.5 * (nvc_u + 1.0)).floor() as usize).min(size - 1);
        let v = ((size as f32 * 0.5 * (nvc_v + 1.0)).floor() as usize).min(size - 1);
        (face_idx, u, v)
    }
}
