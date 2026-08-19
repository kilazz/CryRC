use super::chunk_file::{ChunkDesc, ChunkType};
use super::skin_saver::{BoneBoxData, CSkinningInfo, CryBoneDescData, IntSkinFace, IntSkinVertex};
use byteorder::{ByteOrder, LittleEndian};
use cry_core::math::{AABB, Matrix34, Vec3};
use cry_mesh::{CMesh, CNodeCGF, MeshSubset, PhysGeomType};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct CContentCGF {
    pub filename: String,
    pub nodes: Vec<CNodeCGF>,
    pub materials: Vec<String>,
    pub skinning_info: Option<CSkinningInfo>,
}

pub struct CgfLoader;

impl CgfLoader {
    pub fn load_cgf(path: &Path) -> Result<CContentCGF, String> {
        let raw_data =
            fs::read(path).map_err(|e| format!("Failed to read CGF file {:?}: {}", path, e))?;
        if raw_data.len() < 16 {
            return Err("Invalid CGF file: header too short".to_string());
        }

        if &raw_data[0..6] != b"CryTek" && &raw_data[0..4] != b"CrCh" {
            return Err("Invalid CGF file signature".to_string());
        }

        let num_chunks = LittleEndian::read_u32(&raw_data[12..16]) as usize;
        let mut chunks = Vec::with_capacity(num_chunks);
        let chunk_table_offset = 16;

        for i in 0..num_chunks {
            let entry_offset = chunk_table_offset + i * 24;
            if entry_offset + 24 > raw_data.len() {
                break;
            }

            let chunk_type_raw = LittleEndian::read_u32(&raw_data[entry_offset..entry_offset + 4]);
            let version = LittleEndian::read_u32(&raw_data[entry_offset + 4..entry_offset + 8]);
            let file_offset =
                LittleEndian::read_u32(&raw_data[entry_offset + 8..entry_offset + 12]) as usize;
            let chunk_id = LittleEndian::read_u32(&raw_data[entry_offset + 12..entry_offset + 16]);
            let size =
                LittleEndian::read_u32(&raw_data[entry_offset + 16..entry_offset + 20]) as usize;

            if file_offset + size <= raw_data.len() {
                chunks.push(ChunkDesc {
                    chunk_type: ChunkType::from_u32(chunk_type_raw),
                    version,
                    id: chunk_id,
                    data: raw_data[file_offset..file_offset + size].to_vec(),
                });
            }
        }

        let mut content = CContentCGF {
            filename: path.to_string_lossy().to_string(),
            nodes: Vec::new(),
            materials: Vec::new(),
            skinning_info: None,
        };

        let mut mesh_map = std::collections::HashMap::new();
        for chunk in &chunks {
            if chunk.chunk_type == ChunkType::Mesh
                && let Ok(mesh) = Self::parse_mesh_chunk(&chunk.data, chunk.version)
            {
                mesh_map.insert(chunk.id, mesh);
            }
        }

        for chunk in &chunks {
            if chunk.chunk_type == ChunkType::Node
                && let Ok((mut node, mesh_id)) = Self::parse_node_chunk(&chunk.data, chunk.version)
            {
                if let Some(mesh_id) = mesh_id
                    && let Some(mesh) = mesh_map.remove(&mesh_id)
                {
                    node.mesh = mesh;
                }
                content.nodes.push(node);
            }
        }

        let mut skinning = CSkinningInfo::default();
        for chunk in &chunks {
            match chunk.chunk_type {
                ChunkType::BoneNameList => {
                    if let Ok(names) = Self::parse_bone_name_list(&chunk.data) {
                        for name in names {
                            skinning.bones.push(CryBoneDescData {
                                controller_id: crc32fast::hash(
                                    name.to_ascii_lowercase().as_bytes(),
                                ),
                                bone_name: name,
                                default_b2w: Matrix34::IDENTITY,
                                default_w2b: Matrix34::IDENTITY,
                                parent_offset: 0,
                                num_children: 0,
                            });
                        }
                    }
                }
                ChunkType::BoneInitialPos => {
                    Self::parse_bone_initial_pos(&chunk.data, &mut skinning.bones);
                }
                ChunkType::CompiledBones => {
                    if let Ok(compiled_bones) = Self::parse_compiled_bones(&chunk.data) {
                        skinning.bones = compiled_bones;
                    }
                }
                ChunkType::CompiledIntSkinVertices => {
                    if let Ok(int_verts) = Self::parse_int_skin_vertices(&chunk.data) {
                        skinning.int_vertices = int_verts;
                    }
                }
                ChunkType::CompiledIntFaces => {
                    if let Ok(int_faces) = Self::parse_int_faces(&chunk.data) {
                        skinning.int_faces = int_faces;
                    }
                }
                ChunkType::CompiledExt2IntMap => {
                    if let Ok(ext2int) = Self::parse_ext2int_map(&chunk.data) {
                        skinning.ext2int_map = ext2int;
                    }
                }
                ChunkType::BonesBoxes => {
                    if let Ok(bbox) = Self::parse_bone_box(&chunk.data) {
                        skinning.bone_boxes.push(bbox);
                    }
                }
                _ => {}
            }
        }

        if !skinning.is_empty() || !skinning.int_vertices.is_empty() {
            content.skinning_info = Some(skinning);
        }

        if content.nodes.is_empty() && !mesh_map.is_empty() {
            for (_, mesh) in mesh_map {
                content.nodes.push(CNodeCGF {
                    name: path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    mesh,
                    world_tm: Matrix34::IDENTITY,
                    local_tm: Matrix34::IDENTITY,
                    is_identity_matrix: true,
                    is_physics_proxy: false,
                    properties: String::new(),
                });
            }
        }

        Ok(content)
    }

    fn parse_node_chunk(data: &[u8], _version: u32) -> Result<(CNodeCGF, Option<u32>), String> {
        if data.len() < 144 {
            return Err("Node chunk data too small".to_string());
        }

        let name_bytes = &data[0..64];
        let name_len = name_bytes.iter().position(|&b| b == 0).unwrap_or(64);
        let name = String::from_utf8_lossy(&name_bytes[..name_len]).to_string();

        let object_id = LittleEndian::read_i32(&data[64..68]);
        let mut tm = [0.0f32; 16];
        for i in 0..16 {
            tm[i] = LittleEndian::read_f32(&data[80 + i * 4..84 + i * 4]);
        }

        let mut local_tm = Matrix34::IDENTITY;
        local_tm.m[0] = [tm[0], tm[4], tm[8], tm[12] * 0.01];
        local_tm.m[1] = [tm[1], tm[5], tm[9], tm[13] * 0.01];
        local_tm.m[2] = [tm[2], tm[6], tm[10], tm[14] * 0.01];

        let mut properties = String::new();
        let prop_len = if data.len() >= 160 {
            LittleEndian::read_u32(&data[156..160]) as usize
        } else {
            0
        };

        if prop_len > 0 && data.len() >= 160 + prop_len {
            properties = String::from_utf8_lossy(&data[160..160 + prop_len]).to_string();
        }

        let is_identity = local_tm.is_identity();
        let is_physics_proxy = name.to_ascii_lowercase().contains("phys");

        let node = CNodeCGF {
            name,
            mesh: CMesh::new(),
            world_tm: local_tm,
            local_tm,
            is_identity_matrix: is_identity,
            is_physics_proxy,
            properties,
        };

        let mesh_id = if object_id >= 0 {
            Some(object_id as u32)
        } else {
            None
        };
        Ok((node, mesh_id))
    }

    fn parse_mesh_chunk(data: &[u8], _version: u32) -> Result<CMesh, String> {
        if data.len() < 12 {
            return Err("Mesh chunk data too small".to_string());
        }

        let num_vertices = LittleEndian::read_u32(&data[0..4]) as usize;
        let num_indices = LittleEndian::read_u32(&data[4..8]) as usize;
        let num_subsets = LittleEndian::read_u32(&data[8..12]) as usize;

        let mut mesh = CMesh::new();
        let mut offset = 12;

        let pos_bytes = num_vertices * 12;
        if offset + pos_bytes > data.len() {
            return Err("Mesh chunk vertex buffer out of bounds".to_string());
        }

        for i in 0..num_vertices {
            let p_off = offset + i * 12;
            let x = LittleEndian::read_f32(&data[p_off..p_off + 4]);
            let y = LittleEndian::read_f32(&data[p_off + 4..p_off + 8]);
            let z = LittleEndian::read_f32(&data[p_off + 8..p_off + 12]);
            mesh.positions.push(Vec3::new(x, y, z));
        }
        offset += pos_bytes;

        let is_u16 =
            (data.len() - offset) >= (num_indices * 2) && (data.len() - offset) < (num_indices * 4);
        if is_u16 {
            for i in 0..num_indices {
                mesh.indices.push(LittleEndian::read_u16(
                    &data[offset + i * 2..offset + (i + 1) * 2],
                ) as u32);
            }
            offset += num_indices * 2;
        } else {
            for i in 0..num_indices {
                mesh.indices.push(LittleEndian::read_u32(
                    &data[offset + i * 4..offset + (i + 1) * 4],
                ));
            }
            offset += num_indices * 4;
        }

        for _ in 0..num_subsets {
            if offset + 20 <= data.len() {
                let first_index = LittleEndian::read_u32(&data[offset..offset + 4]);
                let num_indices = LittleEndian::read_u32(&data[offset + 4..offset + 8]);
                let first_vertex = LittleEndian::read_u32(&data[offset + 8..offset + 12]);
                let num_vertices = LittleEndian::read_u32(&data[offset + 12..offset + 16]);
                let mat_id = LittleEndian::read_i32(&data[offset + 16..offset + 20]);

                mesh.subsets.push(MeshSubset {
                    mat_id,
                    physicalize_type: PhysGeomType::None,
                    first_index,
                    num_indices,
                    first_vertex,
                    num_vertices,
                });
                offset += 20;
            }
        }

        mesh.normals
            .resize(mesh.positions.len(), Vec3::new(0.0, 0.0, 1.0));
        mesh.uvs.resize(mesh.positions.len(), [0.0, 0.0]);
        mesh.bbox.reset();
        for p in &mesh.positions {
            mesh.bbox.add_point(*p);
        }

        Ok(mesh)
    }

    fn parse_bone_name_list(data: &[u8]) -> Result<Vec<String>, String> {
        if data.len() < 4 {
            return Ok(Vec::new());
        }
        let num_bones = LittleEndian::read_u32(&data[0..4]) as usize;
        let mut names = Vec::with_capacity(num_bones);
        let mut cur = 4;

        while cur < data.len() && names.len() < num_bones {
            let slice = &data[cur..];
            if let Some(null_pos) = slice.iter().position(|&b| b == 0) {
                if null_pos == 0 {
                    break;
                }
                names.push(String::from_utf8_lossy(&slice[..null_pos]).to_string());
                cur += null_pos + 1;
            } else {
                break;
            }
        }
        Ok(names)
    }

    fn parse_bone_initial_pos(data: &[u8], bones: &mut [CryBoneDescData]) {
        let bone_count = data.len() / 48;
        for (i, bone) in bones.iter_mut().enumerate().take(bone_count) {
            let off = i * 48;
            let mut b2w = Matrix34::IDENTITY;

            b2w.m[0][0] = LittleEndian::read_f32(&data[off..off + 4]);
            b2w.m[1][0] = LittleEndian::read_f32(&data[off + 4..off + 8]);
            b2w.m[2][0] = LittleEndian::read_f32(&data[off + 8..off + 12]);

            b2w.m[0][1] = LittleEndian::read_f32(&data[off + 12..off + 16]);
            b2w.m[1][1] = LittleEndian::read_f32(&data[off + 16..off + 20]);
            b2w.m[2][1] = LittleEndian::read_f32(&data[off + 20..off + 24]);

            b2w.m[0][2] = LittleEndian::read_f32(&data[off + 24..off + 28]);
            b2w.m[1][2] = LittleEndian::read_f32(&data[off + 28..off + 32]);
            b2w.m[2][2] = LittleEndian::read_f32(&data[off + 32..off + 36]);

            b2w.m[0][3] = LittleEndian::read_f32(&data[off + 36..off + 40]) * 0.01;
            b2w.m[1][3] = LittleEndian::read_f32(&data[off + 40..off + 44]) * 0.01;
            b2w.m[2][3] = LittleEndian::read_f32(&data[off + 44..off + 48]) * 0.01;

            bone.default_b2w = b2w;
        }
    }

    fn parse_compiled_bones(data: &[u8]) -> Result<Vec<CryBoneDescData>, String> {
        if data.len() < 4 {
            return Ok(Vec::new());
        }
        let num_bones = LittleEndian::read_u32(&data[0..4]) as usize;
        let mut bones = Vec::with_capacity(num_bones);
        let mut cur = 4;
        let entry_size = 168;

        for _ in 0..num_bones {
            if cur + entry_size > data.len() {
                break;
            }
            let ctrl_id = LittleEndian::read_u32(&data[cur..cur + 4]);
            let name_bytes = &data[cur + 4..cur + 68];
            let name_len = name_bytes.iter().position(|&b| b == 0).unwrap_or(64);
            let name = String::from_utf8_lossy(&name_bytes[..name_len]).to_string();

            let parent_offset = LittleEndian::read_i32(&data[cur + 68..cur + 72]);
            let mut b2w = Matrix34::IDENTITY;
            let mut w2b = Matrix34::IDENTITY;

            let mut m_off = cur + 72;
            for row in 0..3 {
                for col in 0..4 {
                    b2w.m[row][col] = LittleEndian::read_f32(&data[m_off..m_off + 4]);
                    m_off += 4;
                }
            }
            for row in 0..3 {
                for col in 0..4 {
                    w2b.m[row][col] = LittleEndian::read_f32(&data[m_off..m_off + 4]);
                    m_off += 4;
                }
            }

            bones.push(CryBoneDescData {
                bone_name: name,
                controller_id: ctrl_id,
                parent_offset,
                default_b2w: b2w,
                default_w2b: w2b,
                num_children: 0,
            });

            cur += entry_size;
        }

        Ok(bones)
    }

    fn parse_int_skin_vertices(data: &[u8]) -> Result<Vec<IntSkinVertex>, String> {
        if data.len() < 4 {
            return Ok(Vec::new());
        }
        let count = LittleEndian::read_u32(&data[0..4]) as usize;
        let mut vertices = Vec::with_capacity(count);
        let mut cur = 4;

        for _ in 0..count {
            if cur + 32 > data.len() {
                break;
            }
            let px = LittleEndian::read_f32(&data[cur..cur + 4]);
            let py = LittleEndian::read_f32(&data[cur + 4..cur + 8]);
            let pz = LittleEndian::read_f32(&data[cur + 8..cur + 12]);
            let nx = LittleEndian::read_f32(&data[cur + 12..cur + 16]);
            let ny = LittleEndian::read_f32(&data[cur + 16..cur + 20]);
            let nz = LittleEndian::read_f32(&data[cur + 20..cur + 24]);

            vertices.push(IntSkinVertex {
                pos: Vec3::new(px, py, pz),
                normal: Vec3::new(nx, ny, nz),
                bone_ids: [
                    data[cur + 24],
                    data[cur + 25],
                    data[cur + 26],
                    data[cur + 27],
                ],
                weights: [
                    data[cur + 28],
                    data[cur + 29],
                    data[cur + 30],
                    data[cur + 31],
                ],
            });
            cur += 32;
        }
        Ok(vertices)
    }

    fn parse_int_faces(data: &[u8]) -> Result<Vec<IntSkinFace>, String> {
        if data.len() < 4 {
            return Ok(Vec::new());
        }
        let count = LittleEndian::read_u32(&data[0..4]) as usize;
        let mut faces = Vec::with_capacity(count);
        let mut cur = 4;

        for _ in 0..count {
            if cur + 8 > data.len() {
                break;
            }
            faces.push(IntSkinFace {
                i0: LittleEndian::read_u16(&data[cur..cur + 2]) as u32,
                i1: LittleEndian::read_u16(&data[cur + 2..cur + 4]) as u32,
                i2: LittleEndian::read_u16(&data[cur + 4..cur + 6]) as u32,
                mat_id: LittleEndian::read_u16(&data[cur + 6..cur + 8]) as u32,
            });
            cur += 8;
        }
        Ok(faces)
    }

    fn parse_ext2int_map(data: &[u8]) -> Result<Vec<u16>, String> {
        if data.len() < 4 {
            return Ok(Vec::new());
        }
        let count = LittleEndian::read_u32(&data[0..4]) as usize;
        let mut map = Vec::with_capacity(count);
        let mut cur = 4;

        for _ in 0..count {
            if cur + 2 > data.len() {
                break;
            }
            map.push(LittleEndian::read_u16(&data[cur..cur + 2]));
            cur += 2;
        }
        Ok(map)
    }

    fn parse_bone_box(data: &[u8]) -> Result<BoneBoxData, String> {
        if data.len() < 32 {
            return Err("Bone box data too short".to_string());
        }

        let bone_id = LittleEndian::read_i32(&data[0..4]);
        let min_x = LittleEndian::read_f32(&data[4..8]);
        let min_y = LittleEndian::read_f32(&data[8..12]);
        let min_z = LittleEndian::read_f32(&data[12..16]);

        let max_x = LittleEndian::read_f32(&data[16..20]);
        let max_y = LittleEndian::read_f32(&data[20..24]);
        let max_z = LittleEndian::read_f32(&data[24..28]);

        let index_count = LittleEndian::read_u32(&data[28..32]) as usize;
        let mut indices = Vec::with_capacity(index_count);
        let mut cur = 32;

        for _ in 0..index_count {
            if cur + 2 > data.len() {
                break;
            }
            indices.push(LittleEndian::read_u16(&data[cur..cur + 2]));
            cur += 2;
        }

        Ok(BoneBoxData {
            bone_id,
            aabb: AABB {
                min: Vec3::new(min_x, min_y, min_z),
                max: Vec3::new(max_x, max_y, max_z),
            },
            vertex_indices: indices,
        })
    }
}
