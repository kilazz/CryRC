use crate::mesh::{CMesh, MeshSubset, PhysGeomType};
use cry_core::math::{Vec2, Vec3};

#[derive(Debug, Clone)]
pub struct UVSquare {
    pub w: usize,
    pub h: usize,
    pub x: usize,
    pub y: usize,
    pub start_poly: usize,
    pub end_poly: usize,
    pub mx: f32,
    pub my: f32,
}

pub struct AutoUV;

impl AutoUV {
    pub fn pack_squares(squares: &mut [UVSquare], max_dim: usize) -> (usize, usize) {
        squares.sort_by_key(|b| std::cmp::Reverse(b.w * b.h));

        let mut cur_x = 0usize;
        let mut cur_y = 0usize;
        let mut row_h = 0usize;

        for sq in squares.iter_mut() {
            if cur_x + sq.w > max_dim {
                cur_x = 0;
                cur_y += row_h;
                row_h = 0;
            }

            sq.x = cur_x;
            sq.y = cur_y;

            cur_x += sq.w;
            row_h = row_h.max(sq.h);
        }

        (max_dim, (cur_y + row_h).max(max_dim))
    }

    pub fn create_unwrapped_mesh(positions: &[Vec3], indices: &[u32], uvs: &[Vec2]) -> CMesh {
        let mut mesh = CMesh::new();
        mesh.positions = positions.to_vec();
        mesh.indices = indices.to_vec();
        mesh.uvs = uvs.iter().map(|uv| [uv.x, uv.y]).collect();
        mesh.normals = vec![Vec3::new(0.0, 0.0, 1.0); positions.len()];

        mesh.subsets.push(MeshSubset {
            mat_id: 0,
            physicalize_type: PhysGeomType::None,
            first_index: 0,
            num_indices: indices.len() as u32,
            first_vertex: 0,
            num_vertices: positions.len() as u32,
        });

        mesh
    }
}
