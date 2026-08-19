use cry_core::math::{AABB, Matrix34, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum PhysGeomType {
    #[default]
    None = 0,
    Default = 1,
    NoCollide = 2,
    Obb = 3,
    Obstruct = 4,
    DefaultProxy = 5,
}

#[derive(Debug, Clone, Default)]
pub struct MeshSubset {
    pub mat_id: i32,
    pub physicalize_type: PhysGeomType,
    pub first_index: u32,
    pub num_indices: u32,
    pub first_vertex: u32,
    pub num_vertices: u32,
}

#[derive(Debug, Clone, Default)]
pub struct CMesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<[f32; 2]>,
    pub colors_0: Vec<[u8; 4]>,
    pub colors_1: Vec<[u8; 4]>,
    pub topology_ids: Vec<u32>,
    pub indices: Vec<u32>,
    pub subsets: Vec<MeshSubset>,
    pub bbox: AABB,
}

impl CMesh {
    pub fn new() -> Self {
        let mut mesh = Self::default();
        mesh.bbox.reset();
        mesh
    }

    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    pub fn face_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn copy_from(&mut self, other: &CMesh) {
        self.positions = other.positions.clone();
        self.normals = other.normals.clone();
        self.uvs = other.uvs.clone();
        self.colors_0 = other.colors_0.clone();
        self.colors_1 = other.colors_1.clone();
        self.topology_ids = other.topology_ids.clone();
        self.indices = other.indices.clone();
        self.subsets = other.subsets.clone();
        self.bbox = other.bbox;
    }

    pub fn append_streams_from(&mut self, other: &CMesh) -> Result<(), String> {
        let old_vcount = self.vertex_count() as u32;
        self.positions.extend_from_slice(&other.positions);
        self.normals.extend_from_slice(&other.normals);
        self.uvs.extend_from_slice(&other.uvs);
        self.topology_ids.extend_from_slice(&other.topology_ids);

        if !self.colors_0.is_empty() || !other.colors_0.is_empty() {
            if self.colors_0.len() < old_vcount as usize {
                self.colors_0
                    .resize(old_vcount as usize, [255, 255, 255, 255]);
            }
            if other.colors_0.is_empty() {
                self.colors_0
                    .resize(self.positions.len(), [255, 255, 255, 255]);
            } else {
                self.colors_0.extend_from_slice(&other.colors_0);
            }
        }

        for &idx in &other.indices {
            self.indices.push(idx + old_vcount);
        }

        for sub in &other.subsets {
            let mut new_sub = sub.clone();
            new_sub.first_index += self.indices.len() as u32;
            new_sub.first_vertex += old_vcount;
            self.subsets.push(new_sub);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct CNodeCGF {
    pub name: String,
    pub mesh: CMesh,
    pub world_tm: Matrix34,
    pub local_tm: Matrix34,
    pub is_identity_matrix: bool,
    pub is_physics_proxy: bool,
    pub properties: String,
}
