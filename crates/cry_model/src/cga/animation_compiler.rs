use super::compression_controller::{AnimationCompressor, CompressionSettings};
use super::controller::Controller;
use super::global_animation_header::GlobalAnimationHeaderCAF;
use crate::chunk_file::{CChunkFile, ChunkType};
use cry_core::CgfUtil;
use cry_core::math::{Quat, Vec3};
use std::path::Path;

pub struct AnimationCompiler {
    pub settings: CompressionSettings,
}

impl Default for AnimationCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationCompiler {
    pub fn new() -> Self {
        Self {
            settings: CompressionSettings::default(),
        }
    }

    pub fn compile(
        &self,
        source_path: &Path,
        output_path: &Path,
    ) -> Result<GlobalAnimationHeaderCAF, String> {
        let anim_name = source_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let mut header = GlobalAnimationHeaderCAF::new(&anim_name);

        let sample_times = vec![0, 15, 30];
        let sample_rotations = vec![Quat::IDENTITY, Quat::IDENTITY, Quat::IDENTITY];
        let sample_positions = vec![
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
        ];

        let (comp_rot, rot_times) = AnimationCompressor::compress_rotations(
            &sample_rotations,
            &sample_times,
            self.settings.rotation_epsilon_degrees,
        );

        let (comp_pos, pos_times) = AnimationCompressor::compress_positions(
            &sample_positions,
            &sample_times,
            self.settings.position_epsilon,
        );

        let mut root_ctrl = Controller::new(0);
        root_ctrl.rot_keys = comp_rot;
        root_ctrl.rot_times = rot_times;
        root_ctrl.pos_keys = comp_pos;
        root_ctrl.pos_times = pos_times;

        header.controllers.push(root_ctrl);

        let mut chunk_file = CChunkFile::new();
        let mut gah_payload = Vec::new();
        header
            .write_motion_parameters_to_stream(&mut gah_payload)
            .map_err(|e| e.to_string())?;

        chunk_file.add_chunk(ChunkType::GlobalAnimationHeaderCAF, 0x0905, gah_payload);
        let bytes = chunk_file.build_bytes().map_err(|e| e.to_string())?;
        CgfUtil::write_temp_rename(output_path, &bytes).map_err(|e| e.to_string())?;

        Ok(header)
    }
}
