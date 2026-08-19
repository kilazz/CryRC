use super::anim_saver::{ExportFlags, SaverAnim};
use super::cgf_loader::CgfLoader;
use super::chunk_file::{CChunkFile, ChunkType};
use byteorder::{LittleEndian, WriteBytesExt};
use cry_core::CgfUtil;
use cry_mesh::{AutoGenerator, AutoLodSettings, CMesh, MeshCompiler, PhysGeomType};
use std::path::{Path, PathBuf};

pub const MAX_STATOBJ_LODS_NUM: usize = 6;
pub const CGF_NODE_NAME_LOD_PREFIX: &str = "$lod";

#[derive(Debug, Clone)]
pub struct StatCGFCompilerConfig {
    pub vertex_pos_f16: bool,
    pub vertex_idx_u16: bool,
    pub use_qtangents: bool,
    pub split_lods: bool,
    pub strip_mesh_data: bool,
    pub merge_all_nodes: bool,
}

impl Default for StatCGFCompilerConfig {
    fn default() -> Self {
        Self {
            vertex_pos_f16: false,
            vertex_idx_u16: true,
            use_qtangents: true,
            split_lods: false,
            strip_mesh_data: false,
            merge_all_nodes: true,
        }
    }
}

pub struct StatCGFCompiler {
    pub config: StatCGFCompilerConfig,
}

impl StatCGFCompiler {
    pub fn new(config: StatCGFCompilerConfig) -> Self {
        Self { config }
    }

    pub fn process(&self, source_path: &Path, output_path: &Path) -> Result<Vec<PathBuf>, String> {
        let cgf_content = CgfLoader::load_cgf(source_path)
            .map_err(|e| format!("Failed to parse CGF {:?}: {}", source_path, e))?;

        if cgf_content.nodes.is_empty() {
            return Err(format!("CGF {:?} contains no mesh nodes", source_path));
        }

        let mut lod0_nodes = Vec::new();
        let mut lod_nodes_by_level: Vec<Vec<cry_mesh::CNodeCGF>> =
            vec![Vec::new(); MAX_STATOBJ_LODS_NUM];

        for node in cgf_content.nodes {
            let lower_name = node.name.to_ascii_lowercase();
            if lower_name.starts_with(CGF_NODE_NAME_LOD_PREFIX) {
                let prefix_len = CGF_NODE_NAME_LOD_PREFIX.len();
                let lvl_char = lower_name.chars().nth(prefix_len).unwrap_or('1');
                let lod_idx = lvl_char.to_digit(10).unwrap_or(1) as usize;

                if lod_idx > 0 && lod_idx < MAX_STATOBJ_LODS_NUM {
                    lod_nodes_by_level[lod_idx].push(node);
                } else {
                    lod_nodes_by_level[1].push(node);
                }
            } else {
                lod0_nodes.push(node);
            }
        }

        let has_authored_lods = lod_nodes_by_level.iter().skip(1).any(|l| !l.is_empty());
        if !has_authored_lods && self.config.split_lods {
            let auto_gen = AutoGenerator::new(AutoLodSettings::default());
            auto_gen.generate_lods_for_nodes(&mut lod0_nodes);

            let mut remaining_lod0 = Vec::new();
            for node in lod0_nodes {
                let lower_name = node.name.to_ascii_lowercase();
                if lower_name.starts_with(CGF_NODE_NAME_LOD_PREFIX) {
                    let lvl_char = lower_name
                        .chars()
                        .nth(CGF_NODE_NAME_LOD_PREFIX.len())
                        .unwrap_or('1');
                    let lod_idx = lvl_char.to_digit(10).unwrap_or(1) as usize;
                    if lod_idx < MAX_STATOBJ_LODS_NUM {
                        lod_nodes_by_level[lod_idx].push(node);
                    }
                } else {
                    remaining_lod0.push(node);
                }
            }
            lod0_nodes = remaining_lod0;
        }

        let mut output_files = Vec::new();

        let mut lod0_mesh = CMesh::new();
        if self.config.merge_all_nodes {
            for node in &lod0_nodes {
                let old_vcount = lod0_mesh.vertex_count();
                if old_vcount == 0 {
                    lod0_mesh.copy_from(&node.mesh);
                } else {
                    lod0_mesh.append_streams_from(&node.mesh)?;
                }
                if !node.is_identity_matrix {
                    for j in old_vcount..lod0_mesh.vertex_count() {
                        lod0_mesh.positions[j] =
                            node.world_tm.transform_point(&lod0_mesh.positions[j]);
                        lod0_mesh.normals[j] = node.world_tm.rotate_vector(&lod0_mesh.normals[j]);
                    }
                }
            }
        } else if let Some(first_node) = lod0_nodes.first() {
            lod0_mesh.copy_from(&first_node.mesh);
        }

        if lod0_mesh.subsets.is_empty() {
            lod0_mesh.subsets.push(cry_mesh::MeshSubset {
                mat_id: 0,
                physicalize_type: PhysGeomType::None,
                first_index: 0,
                num_indices: lod0_mesh.indices.len() as u32,
                first_vertex: 0,
                num_vertices: lod0_mesh.positions.len() as u32,
            });
        }

        MeshCompiler::compile(&mut lod0_mesh, true)?;

        let mut lod0_chunk_file = CChunkFile::new();
        let export_flags = ExportFlags {
            merge_all_nodes: self.config.merge_all_nodes,
            use_custom_normals: true,
            want_f32_vertices: !self.config.vertex_pos_f16,
            ..Default::default()
        };
        SaverAnim::save_export_flags(&mut lod0_chunk_file, &export_flags);
        self.save_mesh_chunks(&mut lod0_chunk_file, &lod0_mesh)?;
        self.save_node_chunks(&mut lod0_chunk_file, output_path, &lod0_mesh)?;

        let lod0_bytes = lod0_chunk_file.build_bytes().map_err(|e| e.to_string())?;
        CgfUtil::write_temp_rename(output_path, &lod0_bytes).map_err(|e| e.to_string())?;
        output_files.push(output_path.to_path_buf());

        Ok(output_files)
    }

    fn save_mesh_chunks(&self, chunk_file: &mut CChunkFile, mesh: &CMesh) -> Result<(), String> {
        let mut mesh_data = Vec::new();
        mesh_data
            .write_u32::<LittleEndian>(mesh.positions.len() as u32)
            .unwrap();
        mesh_data
            .write_u32::<LittleEndian>(mesh.indices.len() as u32)
            .unwrap();
        mesh_data
            .write_u32::<LittleEndian>(mesh.subsets.len() as u32)
            .unwrap();

        for p in &mesh.positions {
            if self.config.vertex_pos_f16 {
                mesh_data
                    .write_u16::<LittleEndian>(half::f16::from_f32(p.x).to_bits())
                    .unwrap();
                mesh_data
                    .write_u16::<LittleEndian>(half::f16::from_f32(p.y).to_bits())
                    .unwrap();
                mesh_data
                    .write_u16::<LittleEndian>(half::f16::from_f32(p.z).to_bits())
                    .unwrap();
                mesh_data.write_u16::<LittleEndian>(0).unwrap();
            } else {
                mesh_data.write_f32::<LittleEndian>(p.x).unwrap();
                mesh_data.write_f32::<LittleEndian>(p.y).unwrap();
                mesh_data.write_f32::<LittleEndian>(p.z).unwrap();
            }
        }

        for &idx in &mesh.indices {
            if self.config.vertex_idx_u16 && mesh.positions.len() <= 65535 {
                mesh_data.write_u16::<LittleEndian>(idx as u16).unwrap();
            } else {
                mesh_data.write_u32::<LittleEndian>(idx).unwrap();
            }
        }

        for sub in &mesh.subsets {
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
        Ok(())
    }

    fn save_node_chunks(
        &self,
        chunk_file: &mut CChunkFile,
        path: &Path,
        _mesh: &CMesh,
    ) -> Result<(), String> {
        let mut node_data = Vec::new();
        let node_name = path.file_stem().unwrap_or_default().to_string_lossy();

        let mut name_buf = [0u8; 64];
        let bytes = node_name.as_bytes();
        let len = bytes.len().min(63);
        name_buf[..len].copy_from_slice(&bytes[..len]);
        node_data.extend_from_slice(&name_buf);

        node_data.write_i32::<LittleEndian>(0).unwrap();
        node_data.write_i32::<LittleEndian>(-1).unwrap();
        node_data.write_u32::<LittleEndian>(0).unwrap();
        node_data.write_i32::<LittleEndian>(0).unwrap();

        let tm = [
            1.0f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        for &val in &tm {
            node_data.write_f32::<LittleEndian>(val).unwrap();
        }

        node_data.write_i32::<LittleEndian>(-1).unwrap();
        node_data.write_i32::<LittleEndian>(-1).unwrap();
        node_data.write_i32::<LittleEndian>(-1).unwrap();
        node_data.write_u32::<LittleEndian>(0).unwrap();

        chunk_file.add_chunk(ChunkType::Node, 0x0824, node_data);
        Ok(())
    }
}
