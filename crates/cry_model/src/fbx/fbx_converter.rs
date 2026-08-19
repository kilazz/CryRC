use super::import_request::ImportRequest;
use super::mesh_utils::TransformHelpers;
use super::scene::IScene;
use crate::anim_saver::{ExportFlags, SaverAnim};
use crate::chunk_file::{CChunkFile, ChunkType};
use byteorder::{LittleEndian, WriteBytesExt};
use cry_core::CgfUtil;
use cry_core::math::Vec3;
use cry_mesh::{CMesh, MeshCompiler, MeshSubset, PhysGeomType};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneExportType {
    StaticMesh,
    SkinMesh,
    Skeleton,
    Animation,
}

pub struct FbxConverter;

impl FbxConverter {
    pub fn convert_scene(
        scene: &dyn IScene,
        request: &ImportRequest,
        output_path: &Path,
    ) -> Result<(), String> {
        let scale = TransformHelpers::compute_scene_scale(scene, request);
        let _axes_tm = TransformHelpers::compute_axis_transform(
            scene.get_forward_up_axes(),
            &request.forward_up_axes,
        );

        let mut cmesh = CMesh::new();
        for i in 0..scene.get_mesh_count() {
            if let Some(mesh) = scene.get_mesh(i) {
                for p in &mesh.positions {
                    cmesh
                        .positions
                        .push(Vec3::new(p.x * scale, p.y * scale, p.z * scale));
                }
                cmesh.normals.extend_from_slice(&mesh.normals);
                cmesh.uvs.extend_from_slice(&mesh.uvs);
                cmesh.indices.extend_from_slice(&mesh.indices);
            }
        }

        if cmesh.subsets.is_empty() {
            cmesh.subsets.push(MeshSubset {
                mat_id: 0,
                physicalize_type: PhysGeomType::None,
                first_index: 0,
                num_indices: cmesh.indices.len() as u32,
                first_vertex: 0,
                num_vertices: cmesh.positions.len() as u32,
            });
        }

        MeshCompiler::compile(&mut cmesh, true)?;

        let mut chunk_file = CChunkFile::new();
        SaverAnim::save_export_flags(&mut chunk_file, &ExportFlags::default());

        let mut mesh_data = Vec::new();
        mesh_data
            .write_u32::<LittleEndian>(cmesh.positions.len() as u32)
            .unwrap();
        mesh_data
            .write_u32::<LittleEndian>(cmesh.indices.len() as u32)
            .unwrap();
        mesh_data
            .write_u32::<LittleEndian>(cmesh.subsets.len() as u32)
            .unwrap();

        for p in &cmesh.positions {
            mesh_data.write_f32::<LittleEndian>(p.x).unwrap();
            mesh_data.write_f32::<LittleEndian>(p.y).unwrap();
            mesh_data.write_f32::<LittleEndian>(p.z).unwrap();
        }

        for &idx in &cmesh.indices {
            mesh_data.write_u16::<LittleEndian>(idx as u16).unwrap();
        }

        for sub in &cmesh.subsets {
            mesh_data
                .write_u32::<LittleEndian>(sub.first_index)
                .unwrap();
            mesh_data
                .write_u32::<LittleEndian>(sub.num_indices)
                .unwrap();
            mesh_data
                .write_u32::<LittleEndian>(sub.first_vertex)
                .unwrap();
            mesh_data
                .write_u32::<LittleEndian>(sub.num_vertices)
                .unwrap();
            mesh_data.write_i32::<LittleEndian>(sub.mat_id).unwrap();
        }

        chunk_file.add_chunk(ChunkType::Mesh, 0x0800, mesh_data);

        let bytes = chunk_file.build_bytes().map_err(|e| e.to_string())?;
        CgfUtil::write_temp_rename(output_path, &bytes).map_err(|e| e.to_string())?;
        Ok(())
    }
}
