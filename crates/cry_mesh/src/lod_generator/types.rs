use cry_core::math::Vec3;

#[derive(Debug, Clone)]
pub struct Vertex {
    pub pos: Vec3,
    pub polys: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct Poly {
    pub v: [u32; 3],
    pub moves: Vec<u32>,
}

#[derive(Debug, Clone, Copy)]
pub struct Move {
    pub from: u32,
    pub to: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct TakenMove {
    pub from: u32,
    pub to: u32,
    pub error: f32,
}

#[derive(Debug, Clone)]
pub struct ZSpanData {
    pub height: f32,
    pub move_id: u32,
    pub poly_id: u32,
}

#[derive(Debug, Clone, Default)]
pub struct ZSpan {
    pub error: f32,
    pub reference: f32,
    pub data: Vec<ZSpanData>,
}

#[derive(Debug, Clone)]
pub struct LODSequenceOutput {
    pub positions: Vec<Vec3>,
    pub indices: Vec<u32>,
    pub move_list: Vec<TakenMove>,
}

#[derive(Debug, Clone)]
pub struct LODGenParams {
    pub view_resolution: f32,
    pub views_around: usize,
    pub view_elevations: usize,
    pub silhouette_weight: f32,
    pub vertex_welding_distance: f32,
    pub check_topology: bool,
    pub object_has_base: bool,
}

impl Default for LODGenParams {
    fn default() -> Self {
        Self {
            view_resolution: 25.0,
            views_around: 12,
            view_elevations: 3,
            silhouette_weight: 5.0,
            vertex_welding_distance: 0.001,
            check_topology: true,
            object_has_base: false,
        }
    }
}
