use super::controller::Controller;
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::{self, Write};

#[derive(Debug, Clone, Default)]
pub struct FootPlantVectors {
    pub l_heel_start: f32,
    pub l_heel_end: f32,
    pub l_toe_start: f32,
    pub l_toe_end: f32,
    pub r_heel_start: f32,
    pub r_heel_end: f32,
    pub r_toe_start: f32,
    pub r_toe_end: f32,
}

#[derive(Debug, Clone)]
pub struct GlobalAnimationHeaderCAF {
    pub file_path: String,
    pub file_path_crc32: u32,
    pub dba_path: String,
    pub flags: u32,
    pub speed: f32,
    pub distance: f32,
    pub slope: f32,
    pub turn_speed: f32,
    pub asset_turn: f32,
    pub controllers: Vec<Controller>,
    pub foot_plants: FootPlantVectors,
}

impl GlobalAnimationHeaderCAF {
    pub fn new(path: &str) -> Self {
        Self {
            file_path: path.to_string(),
            file_path_crc32: crc32fast::hash(path.to_ascii_lowercase().as_bytes()),
            dba_path: String::new(),
            flags: 0,
            speed: 0.0,
            distance: 0.0,
            slope: 0.0,
            turn_speed: 0.0,
            asset_turn: 0.0,
            controllers: Vec::new(),
            foot_plants: FootPlantVectors::default(),
        }
    }

    pub fn write_motion_parameters_to_stream<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_u32::<LittleEndian>(self.flags)?;
        w.write_f32::<LittleEndian>(self.speed)?;
        w.write_f32::<LittleEndian>(self.distance)?;
        w.write_f32::<LittleEndian>(self.slope)?;
        w.write_f32::<LittleEndian>(self.asset_turn)?;
        w.write_f32::<LittleEndian>(self.turn_speed)?;
        w.write_f32::<LittleEndian>(self.foot_plants.l_heel_start)?;
        w.write_f32::<LittleEndian>(self.foot_plants.l_heel_end)?;
        w.write_f32::<LittleEndian>(self.foot_plants.l_toe_start)?;
        w.write_f32::<LittleEndian>(self.foot_plants.l_toe_end)?;
        w.write_f32::<LittleEndian>(self.foot_plants.r_heel_start)?;
        w.write_f32::<LittleEndian>(self.foot_plants.r_heel_end)?;
        w.write_f32::<LittleEndian>(self.foot_plants.r_toe_start)?;
        w.write_f32::<LittleEndian>(self.foot_plants.r_toe_end)?;
        Ok(())
    }
}
