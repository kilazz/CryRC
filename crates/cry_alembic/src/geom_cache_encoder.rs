use super::geom_cache_file::*;
use super::geom_cache_predictors::*;
use super::geom_cache_writer::GeomCacheWriter;
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::{self, Write};

pub struct MeshRawFrame {
    pub positions: Vec<Position>,
    pub texcoords: Vec<Texcoords>,
    pub qtangents: Vec<QTangent>,
    pub colors: Vec<Color>,
}

pub struct GeomCacheEncoder<'a> {
    writer: &'a mut GeomCacheWriter,
    #[allow(dead_code)]
    use_b_frames: bool,
    #[allow(dead_code)]
    index_frame_distance: u32,
}

impl<'a> GeomCacheEncoder<'a> {
    pub fn new(
        writer: &'a mut GeomCacheWriter,
        use_b_frames: bool,
        index_frame_distance: u32,
    ) -> Self {
        Self {
            writer,
            use_b_frames,
            index_frame_distance: index_frame_distance.min(MAX_IFRAME_DISTANCE),
        }
    }

    pub fn encode_iframe(
        &mut self,
        frame_time: f32,
        mesh: &MeshRawFrame,
        predictor_data: &[u16],
    ) -> io::Result<()> {
        let mut buffer = Vec::new();
        buffer.write_u32::<LittleEndian>(0)?;
        buffer.write_all(&[0u8; 12])?;

        let predicted_positions = if !predictor_data.is_empty() {
            parallelogram_predict_positions(&mesh.positions, predictor_data)
        } else {
            mesh.positions.clone()
        };

        for p in &predicted_positions {
            buffer.write_u16::<LittleEndian>(p.x)?;
            buffer.write_u16::<LittleEndian>(p.y)?;
            buffer.write_u16::<LittleEndian>(p.z)?;
        }
        while buffer.len() % 16 != 0 {
            buffer.push(0);
        }

        for uv in &mesh.texcoords {
            buffer.write_u16::<LittleEndian>(uv.u)?;
            buffer.write_u16::<LittleEndian>(uv.v)?;
        }
        while buffer.len() % 16 != 0 {
            buffer.push(0);
        }

        for q in &mesh.qtangents {
            for &c in q {
                buffer.write_i16::<LittleEndian>(c)?;
            }
        }
        while buffer.len() % 16 != 0 {
            buffer.push(0);
        }

        for &col in &mesh.colors {
            buffer.write_u8(col)?;
        }
        while buffer.len() % 16 != 0 {
            buffer.push(0);
        }

        let (offset, size) = self.writer.write_block(&buffer, true)?;
        self.writer
            .add_frame_info(FrameType::IFrame, frame_time, offset, size);
        Ok(())
    }

    pub fn encode_bframe(
        &mut self,
        frame_time: f32,
        current_mesh: &MeshRawFrame,
        floor_mesh: &MeshRawFrame,
        ceil_mesh: &MeshRawFrame,
    ) -> io::Result<()> {
        let mut buffer = Vec::new();

        let (pos_ctrl, predicted_positions) = optimize_temporal_predictor(
            &current_mesh.positions,
            &floor_mesh.positions,
            &ceil_mesh.positions,
        );

        buffer.write_u32::<LittleEndian>(0)?;
        pos_ctrl.write(&mut buffer)?;
        buffer.write_all(&[0u8; 8])?;

        for p in &predicted_positions {
            buffer.write_u16::<LittleEndian>(p.x)?;
            buffer.write_u16::<LittleEndian>(p.y)?;
            buffer.write_u16::<LittleEndian>(p.z)?;
        }
        while buffer.len() % 16 != 0 {
            buffer.push(0);
        }

        let (offset, size) = self.writer.write_block(&buffer, true)?;
        self.writer
            .add_frame_info(FrameType::BFrame, frame_time, offset, size);
        Ok(())
    }
}
