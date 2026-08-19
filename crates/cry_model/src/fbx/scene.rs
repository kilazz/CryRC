use super::mesh_utils::Mesh;
use cry_core::math::{Matrix34, Quat, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeType {
    Mesh,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct NodeAttribute {
    pub attr_type: AttributeType,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct SceneNode {
    pub name: String,
    pub world_transform: Matrix34,
    pub geometry_offset: Matrix34,
    pub attributes: Vec<NodeAttribute>,
    pub parent: i32,
    pub children: Vec<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct SceneTrs {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

#[derive(Debug, Clone)]
pub struct SceneAnimation {
    pub name: String,
    pub start_frame: i32,
    pub end_frame: i32,
}

pub trait IScene: Send + Sync {
    fn get_forward_up_axes(&self) -> &str;
    fn get_unit_size_in_centimeters(&self) -> f64;
    fn get_node_count(&self) -> usize;
    fn get_node(&self, idx: usize) -> Option<&SceneNode>;
    fn get_mesh_count(&self) -> usize;
    fn get_mesh(&self, idx: usize) -> Option<&Mesh>;
    fn evaluate_node_local_transform(&self, node_idx: usize, frame_idx: i32) -> SceneTrs;
}
