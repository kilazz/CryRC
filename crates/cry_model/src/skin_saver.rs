use super::chunk_file::{CChunkFile, ChunkType};
use byteorder::{LittleEndian, WriteBytesExt};
use cry_core::math::{AABB, Matrix34, Vec3};

#[derive(Debug, Clone, Default)]
pub struct CryBoneDescData {
    pub bone_name: String,
    pub default_b2w: Matrix34,
    pub default_w2b: Matrix34,
    pub parent_offset: i32,
    pub controller_id: u32,
    pub num_children: i32,
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct IntSkinVertex {
    pub pos: Vec3,
    pub normal: Vec3,
    pub bone_ids: [u8; 4],
    pub weights: [u8; 4],
}

#[derive(Debug, Clone, Copy, Default)]
pub struct IntSkinFace {
    pub i0: u32,
    pub i1: u32,
    pub i2: u32,
    pub mat_id: u32,
}

#[derive(Debug, Clone, Default)]
pub struct BoneBoxData {
    pub bone_id: i32,
    pub aabb: AABB,
    pub vertex_indices: Vec<u16>,
}

#[derive(Debug, Clone, Default)]
pub struct CSkinningInfo {
    pub bones: Vec<CryBoneDescData>,
    pub int_vertices: Vec<IntSkinVertex>,
    pub int_faces: Vec<IntSkinFace>,
    pub ext2int_map: Vec<u16>,
    pub bone_boxes: Vec<BoneBoxData>,
}

impl CSkinningInfo {
    pub fn is_empty(&self) -> bool {
        self.bones.is_empty()
    }
}

pub struct SkinSaver;

impl SkinSaver {
    pub fn save_bone_names(chunk_file: &mut CChunkFile, bones: &[CryBoneDescData]) {
        let mut data = Vec::new();
        data.write_u32::<LittleEndian>(bones.len() as u32).unwrap();

        for b in bones {
            data.extend_from_slice(b.bone_name.as_bytes());
            data.push(0);
        }
        data.push(0);

        chunk_file.add_chunk(ChunkType::BoneNameList, 0x0745, data);
    }

    pub fn save_bone_initial_matrices(
        chunk_file: &mut CChunkFile,
        bones: &[CryBoneDescData],
        unit_size_in_centimeters: f32,
    ) {
        let mut data = Vec::with_capacity(bones.len() * 48);

        for b in bones {
            for col in 0..3 {
                for row in 0..3 {
                    data.write_f32::<LittleEndian>(b.default_b2w.m[row][col])
                        .unwrap();
                }
            }
            for row in 0..3 {
                let trans = b.default_b2w.m[row][3] * unit_size_in_centimeters;
                data.write_f32::<LittleEndian>(trans).unwrap();
            }
        }

        chunk_file.add_chunk(ChunkType::BoneInitialPos, 0x0825, data);
    }

    pub fn save_compiled_bones(chunk_file: &mut CChunkFile, bones: &[CryBoneDescData]) {
        let mut data = Vec::new();
        data.write_u32::<LittleEndian>(bones.len() as u32).unwrap();

        for b in bones {
            data.write_u32::<LittleEndian>(b.controller_id).unwrap();

            let mut name_buf = [0u8; 64];
            let bytes = b.bone_name.as_bytes();
            let len = bytes.len().min(63);
            name_buf[..len].copy_from_slice(&bytes[..len]);
            data.extend_from_slice(&name_buf);

            data.write_i32::<LittleEndian>(b.parent_offset).unwrap();

            for row in 0..3 {
                for col in 0..4 {
                    data.write_f32::<LittleEndian>(b.default_b2w.m[row][col])
                        .unwrap();
                }
            }
            for row in 0..3 {
                for col in 0..4 {
                    data.write_f32::<LittleEndian>(b.default_w2b.m[row][col])
                        .unwrap();
                }
            }
        }

        chunk_file.add_chunk(ChunkType::CompiledBones, 0x0800, data);
    }

    pub fn save_compiled_int_skin_vertices(
        chunk_file: &mut CChunkFile,
        vertices: &[IntSkinVertex],
    ) {
        let mut data = Vec::with_capacity(4 + vertices.len() * 32);
        data.write_u32::<LittleEndian>(vertices.len() as u32)
            .unwrap();

        for v in vertices {
            data.write_f32::<LittleEndian>(v.pos.x).unwrap();
            data.write_f32::<LittleEndian>(v.pos.y).unwrap();
            data.write_f32::<LittleEndian>(v.pos.z).unwrap();

            data.write_f32::<LittleEndian>(v.normal.x).unwrap();
            data.write_f32::<LittleEndian>(v.normal.y).unwrap();
            data.write_f32::<LittleEndian>(v.normal.z).unwrap();

            data.extend_from_slice(&v.bone_ids);
            data.extend_from_slice(&v.weights);
        }

        chunk_file.add_chunk(ChunkType::CompiledIntSkinVertices, 0x0800, data);
    }

    pub fn save_compiled_int_faces(chunk_file: &mut CChunkFile, faces: &[IntSkinFace]) {
        let mut data = Vec::with_capacity(4 + faces.len() * 8);
        data.write_u32::<LittleEndian>(faces.len() as u32).unwrap();

        for f in faces {
            data.write_u16::<LittleEndian>(f.i0 as u16).unwrap();
            data.write_u16::<LittleEndian>(f.i1 as u16).unwrap();
            data.write_u16::<LittleEndian>(f.i2 as u16).unwrap();
            data.write_u16::<LittleEndian>(f.mat_id as u16).unwrap();
        }

        chunk_file.add_chunk(ChunkType::CompiledIntFaces, 0x0800, data);
    }

    pub fn save_compiled_ext2int_map(chunk_file: &mut CChunkFile, ext2int: &[u16]) {
        let mut data = Vec::with_capacity(4 + ext2int.len() * 2);
        data.write_u32::<LittleEndian>(ext2int.len() as u32)
            .unwrap();

        for &idx in ext2int {
            data.write_u16::<LittleEndian>(idx).unwrap();
        }

        chunk_file.add_chunk(ChunkType::CompiledExt2IntMap, 0x0800, data);
    }

    pub fn save_compiled_bone_boxes(chunk_file: &mut CChunkFile, bone_boxes: &[BoneBoxData]) {
        for bbox in bone_boxes {
            let mut data = Vec::new();
            data.write_i32::<LittleEndian>(bbox.bone_id).unwrap();

            data.write_f32::<LittleEndian>(bbox.aabb.min.x).unwrap();
            data.write_f32::<LittleEndian>(bbox.aabb.min.y).unwrap();
            data.write_f32::<LittleEndian>(bbox.aabb.min.z).unwrap();

            data.write_f32::<LittleEndian>(bbox.aabb.max.x).unwrap();
            data.write_f32::<LittleEndian>(bbox.aabb.max.y).unwrap();
            data.write_f32::<LittleEndian>(bbox.aabb.max.z).unwrap();

            data.write_u32::<LittleEndian>(bbox.vertex_indices.len() as u32)
                .unwrap();
            for &v_idx in &bbox.vertex_indices {
                data.write_u16::<LittleEndian>(v_idx).unwrap();
            }

            chunk_file.add_chunk(ChunkType::BonesBoxes, 0x0800, data);
        }
    }
}
