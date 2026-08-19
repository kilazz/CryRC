use super::anim_saver::{ExportFlags, SaverAnim};
use super::cgf_loader::CgfLoader;
use super::chunk_file::{CChunkFile, ChunkType};
use super::skin_saver::{BoneBoxData, CryBoneDescData, IntSkinFace, IntSkinVertex, SkinSaver};
use byteorder::{LittleEndian, WriteBytesExt};
use cry_core::CgfUtil;
use cry_core::math::{AABB, Vec3};
use cry_mesh::{CMesh, MeshCompiler, MeshSubset, PhysGeomType};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct VClothLink {
    pub i1: usize,
    pub i2: usize,
    pub len_sqr: f32,
}

#[derive(Debug, Clone, Default)]
pub struct VClothPreProcess {
    pub vertices: Vec<Vec3>,
    pub indices: Vec<u32>,
    pub attached: Vec<bool>,
    pub links: Vec<VClothLink>,
}

impl VClothPreProcess {
    pub fn process_cloth(vertices: &[Vec3], indices: &[u32], attached: &[bool]) -> Self {
        let mut cloth = Self {
            vertices: vertices.to_vec(),
            indices: indices.to_vec(),
            attached: attached.to_vec(),
            links: Vec::new(),
        };

        for chunk in indices.chunks_exact(3) {
            let pairs = [
                (chunk[0], chunk[1]),
                (chunk[1], chunk[2]),
                (chunk[2], chunk[0]),
            ];
            for (a, b) in pairs {
                let (i1, i2) = (a as usize, b as usize);
                if i1 < vertices.len() && i2 < vertices.len() {
                    let p1 = vertices[i1];
                    let p2 = vertices[i2];
                    let len_sqr =
                        (p1.x - p2.x).powi(2) + (p1.y - p2.y).powi(2) + (p1.z - p2.z).powi(2);
                    if !cloth
                        .links
                        .iter()
                        .any(|l| (l.i1 == i1 && l.i2 == i2) || (l.i1 == i2 && l.i2 == i1))
                    {
                        cloth.links.push(VClothLink { i1, i2, len_sqr });
                    }
                }
            }
        }
        cloth
    }
}

pub struct CharacterCompiler;

impl Default for CharacterCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl CharacterCompiler {
    pub fn new() -> Self {
        Self
    }

    pub fn process(&self, source_path: &Path, output_path: &Path) -> Result<Vec<PathBuf>, String> {
        let ext = source_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        match ext.as_str() {
            "cdf" => self.process_cdf_merging(source_path, output_path),
            "chr" => self.process_chr(source_path, output_path),
            "skin" => self.process_skin(source_path, output_path),
            _ => Err(format!("Unsupported character extension: .{}", ext)),
        }
    }

    fn process_chr(&self, source_path: &Path, output_path: &Path) -> Result<Vec<PathBuf>, String> {
        let content = CgfLoader::load_cgf(source_path)
            .map_err(|e| format!("Failed to parse source CHR: {}", e))?;
        let bones = if let Some(ref skinning) = content.skinning_info {
            skinning.bones.clone()
        } else {
            content
                .nodes
                .iter()
                .enumerate()
                .map(|(idx, node)| CryBoneDescData {
                    bone_name: node.name.clone(),
                    default_b2w: node.world_tm,
                    default_w2b: node.world_tm,
                    parent_offset: -(idx as i32),
                    controller_id: crc32fast::hash(node.name.to_ascii_lowercase().as_bytes()),
                    num_children: 0,
                })
                .collect()
        };

        if bones.is_empty() {
            return Err(format!("No bones found in skeleton {:?}", source_path));
        }

        let mut chunk_file = CChunkFile::new();
        SaverAnim::save_export_flags(&mut chunk_file, &ExportFlags::default());
        SkinSaver::save_bone_names(&mut chunk_file, &bones);
        SkinSaver::save_bone_initial_matrices(&mut chunk_file, &bones, 100.0);
        SkinSaver::save_compiled_bones(&mut chunk_file, &bones);

        let bytes = chunk_file.build_bytes().map_err(|e| e.to_string())?;
        CgfUtil::write_temp_rename(output_path, &bytes).map_err(|e| e.to_string())?;
        Ok(vec![output_path.to_path_buf()])
    }

    fn process_skin(&self, source_path: &Path, output_path: &Path) -> Result<Vec<PathBuf>, String> {
        let content = CgfLoader::load_cgf(source_path)
            .map_err(|e| format!("Failed to parse source SKIN: {}", e))?;
        if content.nodes.is_empty() {
            return Err(format!("Skin {:?} contains no mesh nodes", source_path));
        }

        let mut merged_mesh = CMesh::new();
        for node in &content.nodes {
            let old_vcount = merged_mesh.vertex_count();
            if old_vcount == 0 {
                merged_mesh.copy_from(&node.mesh);
            } else {
                merged_mesh.append_streams_from(&node.mesh)?;
            }
        }
        if merged_mesh.subsets.is_empty() {
            merged_mesh.subsets.push(MeshSubset {
                mat_id: 0,
                physicalize_type: PhysGeomType::None,
                first_index: 0,
                num_indices: merged_mesh.indices.len() as u32,
                first_vertex: 0,
                num_vertices: merged_mesh.positions.len() as u32,
            });
        }
        MeshCompiler::compile(&mut merged_mesh, true)?;

        let (int_verts, ext2int, int_faces, bone_boxes) =
            if let Some(ref skinning) = content.skinning_info {
                (
                    skinning.int_vertices.clone(),
                    skinning.ext2int_map.clone(),
                    skinning.int_faces.clone(),
                    skinning.bone_boxes.clone(),
                )
            } else {
                Self::build_skinning_buffers(&merged_mesh)
            };

        let mut chunk_file = CChunkFile::new();
        let export_flags = ExportFlags {
            eight_weights_per_vertex: true,
            ..Default::default()
        };
        SaverAnim::save_export_flags(&mut chunk_file, &export_flags);

        self.save_mesh_chunks(&mut chunk_file, &merged_mesh)?;
        self.save_node_chunks(&mut chunk_file, output_path)?;

        if let Some(ref skinning) = content.skinning_info
            && !skinning.bones.is_empty()
        {
            SkinSaver::save_bone_names(&mut chunk_file, &skinning.bones);
            SkinSaver::save_bone_initial_matrices(&mut chunk_file, &skinning.bones, 100.0);
            SkinSaver::save_compiled_bones(&mut chunk_file, &skinning.bones);
        }

        SkinSaver::save_compiled_int_skin_vertices(&mut chunk_file, &int_verts);
        SkinSaver::save_compiled_int_faces(&mut chunk_file, &int_faces);
        SkinSaver::save_compiled_ext2int_map(&mut chunk_file, &ext2int);
        SkinSaver::save_compiled_bone_boxes(&mut chunk_file, &bone_boxes);

        let bytes = chunk_file.build_bytes().map_err(|e| e.to_string())?;
        CgfUtil::write_temp_rename(output_path, &bytes).map_err(|e| e.to_string())?;
        Ok(vec![output_path.to_path_buf()])
    }

    fn process_cdf_merging(
        &self,
        cdf_path: &Path,
        output_path: &Path,
    ) -> Result<Vec<PathBuf>, String> {
        let content =
            fs::read_to_string(cdf_path).map_err(|e| format!("Failed to read CDF: {}", e))?;
        let mut reader = Reader::from_str(&content);
        let mut buf = Vec::new();
        let mut skin_attachments = Vec::new();
        let parent_dir = cdf_path.parent().unwrap_or(Path::new(""));

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    if e.name().as_ref() == b"Attachment" {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"Binding" {
                                let val = String::from_utf8_lossy(&attr.value).to_string();
                                if !val.is_empty() {
                                    let mut attach_path = PathBuf::from(&val);
                                    if !attach_path.exists() {
                                        attach_path = parent_dir.join(&val);
                                    }
                                    if attach_path.exists() {
                                        skin_attachments.push(attach_path);
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(format!("CDF XML parsing error: {}", e)),
                _ => {}
            }
            buf.clear();
        }

        if skin_attachments.is_empty() {
            let target_skin = output_path.with_extension("skin");
            return self.process_skin(cdf_path, &target_skin);
        }

        let mut consolidated_nodes = Vec::new();
        let mut consolidated_bones = Vec::new();

        for attach_file in &skin_attachments {
            if let Ok(attach_cgf) = CgfLoader::load_cgf(attach_file) {
                if let Some(ref skinning) = attach_cgf.skinning_info
                    && consolidated_bones.is_empty()
                {
                    consolidated_bones = skinning.bones.clone();
                }
                consolidated_nodes.extend(attach_cgf.nodes);
            }
        }

        let mut consolidated_mesh = CMesh::new();
        for node in &consolidated_nodes {
            let old_vcount = consolidated_mesh.vertex_count();
            if old_vcount == 0 {
                consolidated_mesh.copy_from(&node.mesh);
            } else {
                consolidated_mesh.append_streams_from(&node.mesh)?;
            }
        }
        if consolidated_mesh.subsets.is_empty() {
            consolidated_mesh.subsets.push(MeshSubset {
                mat_id: 0,
                physicalize_type: PhysGeomType::None,
                first_index: 0,
                num_indices: consolidated_mesh.indices.len() as u32,
                first_vertex: 0,
                num_vertices: consolidated_mesh.positions.len() as u32,
            });
        }
        MeshCompiler::compile(&mut consolidated_mesh, true)?;

        let (int_verts, ext2int, int_faces, bone_boxes) =
            Self::build_skinning_buffers(&consolidated_mesh);
        let target_skin = output_path.with_extension("skin");
        let mut chunk_file = CChunkFile::new();

        SaverAnim::save_export_flags(&mut chunk_file, &ExportFlags::default());
        self.save_mesh_chunks(&mut chunk_file, &consolidated_mesh)?;
        self.save_node_chunks(&mut chunk_file, &target_skin)?;

        if !consolidated_bones.is_empty() {
            SkinSaver::save_bone_names(&mut chunk_file, &consolidated_bones);
            SkinSaver::save_bone_initial_matrices(&mut chunk_file, &consolidated_bones, 100.0);
            SkinSaver::save_compiled_bones(&mut chunk_file, &consolidated_bones);
        }

        SkinSaver::save_compiled_int_skin_vertices(&mut chunk_file, &int_verts);
        SkinSaver::save_compiled_int_faces(&mut chunk_file, &int_faces);
        SkinSaver::save_compiled_ext2int_map(&mut chunk_file, &ext2int);
        SkinSaver::save_compiled_bone_boxes(&mut chunk_file, &bone_boxes);

        let bytes = chunk_file.build_bytes().map_err(|e| e.to_string())?;
        CgfUtil::write_temp_rename(&target_skin, &bytes).map_err(|e| e.to_string())?;

        Ok(vec![target_skin])
    }

    fn build_skinning_buffers(
        mesh: &CMesh,
    ) -> (
        Vec<IntSkinVertex>,
        Vec<u16>,
        Vec<IntSkinFace>,
        Vec<BoneBoxData>,
    ) {
        let v_count = mesh.positions.len();
        let mut int_verts = Vec::with_capacity(v_count);
        let mut ext2int = Vec::with_capacity(v_count);

        for i in 0..v_count {
            let pos = mesh.positions[i];
            let normal = if i < mesh.normals.len() {
                mesh.normals[i]
            } else {
                Vec3::new(0.0, 0.0, 1.0)
            };
            int_verts.push(IntSkinVertex {
                pos,
                normal,
                bone_ids: [0, 0, 0, 0],
                weights: [255, 0, 0, 0],
            });
            ext2int.push(i as u16);
        }

        let mut int_faces = Vec::with_capacity(mesh.indices.len() / 3);
        for chunk in mesh.indices.chunks_exact(3) {
            int_faces.push(IntSkinFace {
                i0: chunk[0],
                i1: chunk[1],
                i2: chunk[2],
                mat_id: 0,
            });
        }

        let mut root_aabb = AABB::default();
        root_aabb.reset();
        for p in &mesh.positions {
            root_aabb.add_point(*p);
        }

        let bone_boxes = vec![BoneBoxData {
            bone_id: 0,
            aabb: root_aabb,
            vertex_indices: (0..v_count as u16).collect(),
        }];

        (int_verts, ext2int, int_faces, bone_boxes)
    }

    fn save_mesh_chunks(&self, chunk_file: &mut CChunkFile, mesh: &CMesh) -> Result<(), String> {
        let mut data = Vec::new();
        data.write_u32::<LittleEndian>(mesh.positions.len() as u32)
            .unwrap();
        data.write_u32::<LittleEndian>(mesh.indices.len() as u32)
            .unwrap();
        data.write_u32::<LittleEndian>(mesh.subsets.len() as u32)
            .unwrap();

        for p in &mesh.positions {
            data.write_f32::<LittleEndian>(p.x).unwrap();
            data.write_f32::<LittleEndian>(p.y).unwrap();
            data.write_f32::<LittleEndian>(p.z).unwrap();
        }

        for &idx in &mesh.indices {
            data.write_u16::<LittleEndian>(idx as u16).unwrap();
        }

        for sub in &mesh.subsets {
            data.write_u32::<LittleEndian>(sub.first_index).unwrap();
            data.write_u32::<LittleEndian>(sub.num_indices).unwrap();
            data.write_u32::<LittleEndian>(sub.first_vertex).unwrap();
            data.write_u32::<LittleEndian>(sub.num_vertices).unwrap();
            data.write_i32::<LittleEndian>(sub.mat_id).unwrap();
        }

        chunk_file.add_chunk(ChunkType::Mesh, 0x0800, data);
        Ok(())
    }

    fn save_node_chunks(&self, chunk_file: &mut CChunkFile, path: &Path) -> Result<(), String> {
        let mut data = Vec::new();
        let node_name = path.file_stem().unwrap_or_default().to_string_lossy();

        let mut name_buf = [0u8; 64];
        let bytes = node_name.as_bytes();
        let len = bytes.len().min(63);
        name_buf[..len].copy_from_slice(&bytes[..len]);
        data.extend_from_slice(&name_buf);

        data.write_i32::<LittleEndian>(0).unwrap();
        data.write_i32::<LittleEndian>(-1).unwrap();
        data.write_u32::<LittleEndian>(0).unwrap();
        data.write_i32::<LittleEndian>(0).unwrap();

        let tm = [
            1.0f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        for &val in &tm {
            data.write_f32::<LittleEndian>(val).unwrap();
        }

        data.write_i32::<LittleEndian>(-1).unwrap();
        data.write_i32::<LittleEndian>(-1).unwrap();
        data.write_i32::<LittleEndian>(-1).unwrap();
        data.write_u32::<LittleEndian>(0).unwrap();

        chunk_file.add_chunk(ChunkType::Node, 0x0824, data);
        Ok(())
    }
}
