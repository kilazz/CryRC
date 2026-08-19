use super::geom_cache_encoder::{GeomCacheEncoder, MeshRawFrame};
use super::geom_cache_file::*;
use super::geom_cache_predictors::optimize_mesh_for_compression;
use super::geom_cache_tangents::encode_qtangent;
use super::geom_cache_writer::GeomCacheWriter;
use super::ogawa::AlembicOgawaParser;
use cry_core::math::{AABB, Matrix33};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AlembicCompilerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Config parsing error: {0}")]
    Config(String),
    #[error("Alembic parse error: {0}")]
    Alembic(String),
}

#[derive(Debug, Clone)]
pub struct AlembicBuildConfig {
    pub up_axis: String,
    pub mesh_prediction: bool,
    pub use_b_frames: bool,
    pub index_frame_distance: u32,
    pub block_compression: BlockCompressionFormat,
    pub playback_from_memory: bool,
    pub position_precision: f64,
}

impl Default for AlembicBuildConfig {
    fn default() -> Self {
        Self {
            up_axis: "Y".to_string(),
            mesh_prediction: false,
            use_b_frames: false,
            index_frame_distance: 15,
            block_compression: BlockCompressionFormat::Deflate,
            playback_from_memory: false,
            position_precision: 1.0,
        }
    }
}

pub struct AlembicCompiler {
    pub config: AlembicBuildConfig,
}

impl Default for AlembicCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl AlembicCompiler {
    pub fn new() -> Self {
        Self {
            config: AlembicBuildConfig::default(),
        }
    }

    pub fn load_or_create_cbc(&mut self, cbc_path: &Path) -> Result<(), AlembicCompilerError> {
        if cbc_path.exists() {
            let content = fs::read_to_string(cbc_path)?;
            let mut reader = Reader::from_str(&content);
            let mut buf = Vec::new();

            loop {
                match reader.read_event_into(&mut buf) {
                    Ok(Event::Empty(e)) | Ok(Event::Start(e)) => {
                        if e.name().as_ref() == b"CacheBuildConfiguration" {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                                let val = String::from_utf8_lossy(&attr.value).to_string();
                                match key.as_str() {
                                    "UpAxis" => self.config.up_axis = val,
                                    "MeshPrediction" => self.config.mesh_prediction = val == "1",
                                    "UseBFrames" => self.config.use_b_frames = val == "1",
                                    "IndexFrameDistance" => {
                                        self.config.index_frame_distance = val.parse().unwrap_or(15)
                                    }
                                    "BlockCompressionFormat" => match val.as_str() {
                                        "store" => {
                                            self.config.block_compression =
                                                BlockCompressionFormat::None
                                        }
                                        "lz4hc" => {
                                            self.config.block_compression =
                                                BlockCompressionFormat::Lz4Hc
                                        }
                                        _ => {
                                            self.config.block_compression =
                                                BlockCompressionFormat::Deflate
                                        }
                                    },
                                    "PlaybackFromMemory" => {
                                        self.config.playback_from_memory = val == "1"
                                    }
                                    "PositionPrecision" => {
                                        self.config.position_precision = val.parse().unwrap_or(1.0)
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    Ok(Event::Eof) => break,
                    Err(e) => return Err(AlembicCompilerError::Config(e.to_string())),
                    _ => {}
                }
                buf.clear();
            }
        }
        Ok(())
    }

    pub fn compile(
        &mut self,
        source_path: &Path,
        output_path: &Path,
    ) -> Result<(), AlembicCompilerError> {
        let cbc_path = source_path.with_extension("cbc");
        self.load_or_create_cbc(&cbc_path)?;

        let abc_scene = AlembicOgawaParser::load_from_file(source_path)
            .map_err(AlembicCompilerError::Alembic)?;
        if abc_scene.meshes.is_empty() {
            return Err(AlembicCompilerError::Alembic(
                "Alembic archive contains no poly meshes".to_string(),
            ));
        }

        let num_frames = abc_scene.frame_times.len() as u32;
        let mut writer = GeomCacheWriter::new(
            output_path,
            num_frames,
            self.config.block_compression,
            self.config.playback_from_memory,
            false,
        )?;

        let mut aabb = AABB::default();
        aabb.reset();

        let primary_mesh = &abc_scene.meshes[0];
        let primary_sample = &primary_mesh.samples[0];

        for p in &primary_sample.positions {
            aabb.add_point(*p);
        }

        let aabb_size = aabb.get_size();
        let k_mult = 65535.0f32;

        let mut encoder = GeomCacheEncoder::new(
            &mut writer,
            self.config.use_b_frames,
            self.config.index_frame_distance,
        );
        let mut last_raw_frame = None;

        for (frame_idx, &frame_time) in abc_scene.frame_times.iter().enumerate() {
            let sample = if frame_idx < primary_mesh.samples.len() {
                &primary_mesh.samples[frame_idx]
            } else {
                primary_sample
            };

            let mut quantized_positions = Vec::with_capacity(sample.positions.len());
            for p in &sample.positions {
                let norm_x = ((p.x - aabb.min.x) / aabb_size.x).clamp(0.0, 1.0);
                let norm_y = ((p.y - aabb.min.y) / aabb_size.y).clamp(0.0, 1.0);
                let norm_z = ((p.z - aabb.min.z) / aabb_size.z).clamp(0.0, 1.0);

                quantized_positions.push(Position {
                    x: (norm_x * k_mult) as u16,
                    y: (norm_y * k_mult) as u16,
                    z: (norm_z * k_mult) as u16,
                });
            }

            let mut indices_map: HashMap<u16, Vec<u32>> = HashMap::new();
            let mut tri_indices = Vec::new();
            let num_verts = sample.positions.len() as u32;

            if num_verts >= 3 {
                for i in 1..num_verts - 1 {
                    tri_indices.push(0);
                    tri_indices.push(i);
                    tri_indices.push(i + 1);
                }
            }
            indices_map.insert(0, tri_indices);

            let (reordered_positions, predictor_data) = optimize_mesh_for_compression(
                &quantized_positions,
                &mut indices_map,
                self.config.mesh_prediction,
            );

            let texcoords = vec![Texcoords { u: 0, v: 0 }; reordered_positions.len()];
            let default_tangent = encode_qtangent(Matrix33::identity(), false);
            let qtangents = vec![default_tangent; reordered_positions.len()];
            let colors = vec![255u8; reordered_positions.len()];

            let current_raw_frame = MeshRawFrame {
                positions: reordered_positions,
                texcoords,
                qtangents,
                colors,
            };

            let is_iframe = !self.config.use_b_frames
                || frame_idx == 0
                || frame_idx == (num_frames as usize - 1)
                || (frame_idx as u32).is_multiple_of(self.config.index_frame_distance);

            if is_iframe || last_raw_frame.is_none() {
                encoder.encode_iframe(frame_time as f32, &current_raw_frame, &predictor_data)?;
            } else if let Some(ref prev_frame) = last_raw_frame {
                encoder.encode_bframe(
                    frame_time as f32,
                    &current_raw_frame,
                    prev_frame,
                    &current_raw_frame,
                )?;
            }

            last_raw_frame = Some(current_raw_frame);
        }

        writer.finish(&aabb)?;
        Ok(())
    }
}
