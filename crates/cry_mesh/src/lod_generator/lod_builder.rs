use super::types::LODSequenceOutput;
use crate::mesh::{CMesh, MeshSubset, PhysGeomType};

pub struct LODMeshBuilder;

impl LODMeshBuilder {
    pub fn build_lod_mesh(sequence: &LODSequenceOutput, percentage: f32) -> CMesh {
        let num_moves =
            ((sequence.move_list.len() as f32) * (1.0 - percentage / 100.0) + 0.5) as usize;
        let num_moves = num_moves.min(sequence.move_list.len());

        let mut remap: Vec<u32> = (0..sequence.positions.len() as u32).collect();
        for i in 0..num_moves {
            let m = &sequence.move_list[i];
            remap[m.from as usize] = m.to;
        }

        for i in 0..remap.len() {
            let mut curr = i as u32;
            while remap[curr as usize] != curr {
                curr = remap[curr as usize];
            }
            remap[i] = curr;
        }

        let mut new_indices = Vec::new();
        for chunk in sequence.indices.chunks(3) {
            if chunk.len() == 3 {
                let id0 = remap[chunk[0] as usize];
                let id1 = remap[chunk[1] as usize];
                let id2 = remap[chunk[2] as usize];

                if id0 != id1 && id0 != id2 && id1 != id2 {
                    new_indices.push(id0);
                    new_indices.push(id1);
                    new_indices.push(id2);
                }
            }
        }

        let mut pos_remap = vec![u32::MAX; sequence.positions.len()];
        let mut out_positions = Vec::new();
        let mut final_indices = Vec::with_capacity(new_indices.len());

        for idx in new_indices {
            if pos_remap[idx as usize] == u32::MAX {
                pos_remap[idx as usize] = out_positions.len() as u32;
                out_positions.push(sequence.positions[idx as usize]);
            }
            final_indices.push(pos_remap[idx as usize]);
        }

        let mut out_mesh = CMesh::new();
        out_mesh.positions = out_positions;
        out_mesh.indices = final_indices;

        out_mesh.subsets.push(MeshSubset {
            mat_id: 0,
            physicalize_type: PhysGeomType::None,
            first_index: 0,
            num_indices: out_mesh.indices.len() as u32,
            first_vertex: 0,
            num_vertices: out_mesh.positions.len() as u32,
        });

        out_mesh
    }
}
