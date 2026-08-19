use super::global_animation_header::GlobalAnimationHeaderCAF;
use super::quat_quantization::SmallTree64BitExtQuat;
use crate::chunk_file::{CChunkFile, ChunkType};
use byteorder::{LittleEndian, WriteBytesExt};
use cry_core::math::Vec3;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;

#[derive(Default)]
pub struct TrackStorage {
    pub animations: Vec<GlobalAnimationHeaderCAF>,
    pub unique_rot_tracks: Vec<Vec<SmallTree64BitExtQuat>>,
    pub unique_pos_tracks: Vec<Vec<Vec3>>,
}

impl TrackStorage {
    pub fn save_database_905(&mut self, output_path: &Path) -> io::Result<()> {
        let mut rot_map: HashMap<Vec<u8>, u32> = HashMap::new();
        let mut pos_map: HashMap<Vec<u8>, u32> = HashMap::new();
        let mut anim_data_bytes = Vec::new();

        for anim in &self.animations {
            anim_data_bytes.write_u16::<LittleEndian>(anim.file_path.len() as u16)?;
            anim_data_bytes.write_all(anim.file_path.as_bytes())?;
            anim.write_motion_parameters_to_stream(&mut anim_data_bytes)?;
            anim_data_bytes.write_u16::<LittleEndian>(anim.controllers.len() as u16)?;

            for ctrl in &anim.controllers {
                let mut rot_raw = Vec::with_capacity(ctrl.rot_keys.len() * 8);
                for k in &ctrl.rot_keys {
                    rot_raw.write_u32::<LittleEndian>(k.m_1)?;
                    rot_raw.write_u32::<LittleEndian>(k.m_2)?;
                }

                let rot_id = if let Some(&id) = rot_map.get(&rot_raw) {
                    id
                } else {
                    let id = self.unique_rot_tracks.len() as u32;
                    rot_map.insert(rot_raw, id);
                    self.unique_rot_tracks.push(ctrl.rot_keys.clone());
                    id
                };

                let mut pos_raw = Vec::with_capacity(ctrl.pos_keys.len() * 12);
                for p in &ctrl.pos_keys {
                    pos_raw.write_f32::<LittleEndian>(p.x)?;
                    pos_raw.write_f32::<LittleEndian>(p.y)?;
                    pos_raw.write_f32::<LittleEndian>(p.z)?;
                }

                let pos_id = if let Some(&id) = pos_map.get(&pos_raw) {
                    id
                } else {
                    let id = self.unique_pos_tracks.len() as u32;
                    pos_map.insert(pos_raw, id);
                    self.unique_pos_tracks.push(ctrl.pos_keys.clone());
                    id
                };

                anim_data_bytes.write_u32::<LittleEndian>(ctrl.controller_id)?;
                anim_data_bytes.write_u32::<LittleEndian>(rot_id)?;
                anim_data_bytes.write_u32::<LittleEndian>(pos_id)?;
            }
        }

        let mut chunk_file = CChunkFile::new();
        let mut payload = Vec::new();
        payload.write_u32::<LittleEndian>(self.animations.len() as u32)?;
        payload.write_u32::<LittleEndian>(self.unique_rot_tracks.len() as u32)?;
        payload.write_u32::<LittleEndian>(self.unique_pos_tracks.len() as u32)?;
        payload.extend_from_slice(&anim_data_bytes);

        chunk_file.add_chunk(ChunkType::Controller, 0x0905, payload);

        if let Some(parent) = output_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let bytes = chunk_file
            .build_bytes()
            .map_err(|e| io::Error::other(e.to_string()))?;
        std::fs::write(output_path, bytes)?;
        Ok(())
    }
}
