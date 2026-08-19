use super::import_request::ImportRequest;
use super::scene::IScene;
use cry_core::math::{Matrix34, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct BoneLink {
    pub bone_id: usize,
    pub weight: f32,
}

#[derive(Debug, Clone, Default)]
pub struct VertexLinks {
    pub links: Vec<BoneLink>,
}

impl VertexLinks {
    pub fn normalize(&mut self, max_bone_links: usize) {
        self.links
            .sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap());
        self.links.truncate(max_bone_links);
        let sum: f32 = self.links.iter().map(|l| l.weight).sum();
        if sum > 1e-6 {
            let inv = 1.0 / sum;
            for l in &mut self.links {
                l.weight *= inv;
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub links: Vec<VertexLinks>,
}

pub struct TransformHelpers;

impl TransformHelpers {
    pub fn compute_axis_transform(source_axes: &str, target_axes: &str) -> Matrix34 {
        let mut m = Matrix34::IDENTITY;
        if source_axes.eq_ignore_ascii_case("+Z+Y") && target_axes.eq_ignore_ascii_case("-Y+Z") {
            m.m[0] = [1.0, 0.0, 0.0, 0.0];
            m.m[1] = [0.0, 0.0, -1.0, 0.0];
            m.m[2] = [0.0, 1.0, 0.0, 0.0];
        }
        m
    }

    pub fn compute_scene_scale(scene: &dyn IScene, request: &ImportRequest) -> f32 {
        let unit_cm = match request.source_unit_size_text.to_ascii_lowercase().as_str() {
            "mm" => 0.1,
            "m" => 100.0,
            "in" => 2.54,
            _ => scene.get_unit_size_in_centimeters(),
        };
        (request.scale as f64 * (unit_cm * 0.01)) as f32
    }
}
