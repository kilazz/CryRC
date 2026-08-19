use super::anim_saver::{ExportFlags, SaverAnim};
use super::chunk_file::{CChunkFile, ChunkType};
use super::skin_saver::{CryBoneDescData, SkinSaver};
use byteorder::{LittleEndian, WriteBytesExt};
use cry_core::math::{Quat, Vec3};

#[derive(Debug, Clone, Copy, Default)]
pub struct CryKeyPQS {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

pub struct SaverCAF;

impl SaverCAF {
    pub fn save_controller_0833(
        chunk_file: &mut CChunkFile,
        frames: &[CryKeyPQS],
        controller_id: u32,
    ) {
        let mut data = Vec::new();
        data.write_u32::<LittleEndian>(frames.len() as u32).unwrap();
        data.write_u32::<LittleEndian>(controller_id).unwrap();

        for f in frames {
            data.write_f32::<LittleEndian>(f.position.x).unwrap();
            data.write_f32::<LittleEndian>(f.position.y).unwrap();
            data.write_f32::<LittleEndian>(f.position.z).unwrap();

            data.write_f32::<LittleEndian>(f.rotation.v[0]).unwrap();
            data.write_f32::<LittleEndian>(f.rotation.v[1]).unwrap();
            data.write_f32::<LittleEndian>(f.rotation.v[2]).unwrap();
            data.write_f32::<LittleEndian>(f.rotation.w).unwrap();

            data.write_f32::<LittleEndian>(f.scale.x).unwrap();
            data.write_f32::<LittleEndian>(f.scale.y).unwrap();
            data.write_f32::<LittleEndian>(f.scale.z).unwrap();
        }

        chunk_file.add_chunk(ChunkType::Controller, 0x0833, data);
    }

    pub fn build_uncompressed_caf(
        bones: &[CryBoneDescData],
        controllers: &[(u32, Vec<CryKeyPQS>)],
        start_frame: i32,
        end_frame: i32,
    ) -> Vec<u8> {
        let mut chunk_file = CChunkFile::new();
        SaverAnim::save_export_flags(&mut chunk_file, &ExportFlags::default());
        SaverAnim::save_timing(&mut chunk_file, start_frame, end_frame);

        for (ctrl_id, frames) in controllers {
            Self::save_controller_0833(&mut chunk_file, frames, *ctrl_id);
        }

        SkinSaver::save_bone_names(&mut chunk_file, bones);
        chunk_file.build_bytes().unwrap()
    }
}
