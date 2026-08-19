use super::mesh::CMesh;
use super::mikktspace::{MikkTSpaceGenerator, MikkTSpaceMesh};
use super::vertex_cache::ForsythOptimizer;
use cry_core::math::Vec3;

struct CMeshMikkAdapter<'a> {
    mesh: &'a mut CMesh,
    tangents: Vec<Vec3>,
    bitangents: Vec<Vec3>,
}

impl<'a> MikkTSpaceMesh for CMeshMikkAdapter<'a> {
    fn get_num_faces(&self) -> usize {
        self.mesh.indices.len() / 3
    }
    fn get_num_vertices_of_face(&self, _face: usize) -> usize {
        3
    }
    fn get_position(&self, face: usize, vert: usize) -> Vec3 {
        self.mesh.positions[self.mesh.indices[face * 3 + vert] as usize]
    }
    fn get_normal(&self, face: usize, vert: usize) -> Vec3 {
        self.mesh.normals[self.mesh.indices[face * 3 + vert] as usize]
    }
    fn get_tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
        self.mesh.uvs[self.mesh.indices[face * 3 + vert] as usize]
    }
    fn set_tspace(
        &mut self,
        tangent: Vec3,
        bitangent: Vec3,
        _mag_s: f32,
        _mag_t: f32,
        _is_orientation_preserving: bool,
        face: usize,
        vert: usize,
    ) {
        let idx = self.mesh.indices[face * 3 + vert] as usize;
        self.tangents[idx] = tangent;
        self.bitangents[idx] = bitangent;
    }
}

pub struct MeshCompiler;

impl MeshCompiler {
    pub fn compile(mesh: &mut CMesh, optimize_vertex_cache: bool) -> Result<(), String> {
        if mesh.positions.is_empty() || mesh.indices.is_empty() {
            return Ok(());
        }

        let v_count = mesh.positions.len();
        let mut adapter = CMeshMikkAdapter {
            tangents: vec![Vec3::new(1.0, 0.0, 0.0); v_count],
            bitangents: vec![Vec3::new(0.0, 1.0, 0.0); v_count],
            mesh,
        };

        if !MikkTSpaceGenerator::gen_tang_space_default(&mut adapter) {
            return Err("MikkTSpace tangent calculation failed.".to_string());
        }

        if optimize_vertex_cache {
            adapter.mesh.indices = ForsythOptimizer::optimize_indices(
                &adapter.mesh.indices,
                adapter.mesh.positions.len(),
            );
        }

        adapter.mesh.bbox.reset();
        for p in &adapter.mesh.positions {
            adapter.mesh.bbox.add_point(*p);
        }

        Ok(())
    }
}
