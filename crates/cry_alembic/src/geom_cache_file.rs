use byteorder::{LittleEndian, WriteBytesExt};
use std::io::{self, Write};

pub const CAX_FILE_SIGNATURE: u32 = 0x43415843; // 'CAXC'
pub const CAX_FILE_VERSION: u32 = 6;
pub const MAX_IFRAME_DISTANCE: u32 = 60;
pub const MESH_PREDICTOR_LOOK_BACK_MAX_DIST: u32 = 0xFFFE;
pub const TANGENT_QUAT_PRECISION: u32 = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BlockCompressionFormat {
    None = 0,
    Deflate = 1,
    Lz4Hc = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    IFrame = 0,
    BFrame = 1,
}

pub const FILE_HEADER_FLAG_PLAYBACK_FROM_MEMORY: u32 = 1 << 0;
pub const FILE_HEADER_FLAG_32BIT_INDICES: u32 = 1 << 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Position {
    pub x: u16,
    pub y: u16,
    pub z: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Texcoords {
    pub u: u16,
    pub v: u16,
}

pub type QTangent = [i16; 4];
pub type Color = u8;

#[derive(Debug, Clone, Default)]
pub struct SHeader {
    pub signature: u32,
    pub version: u32,
    pub total_uncompressed_animation_size: u64,
    pub num_frames: u32,
    pub flags: u32,
    pub block_compression_format: u8,
    pub aabb_min: [f32; 3],
    pub aabb_max: [f32; 3],
}

impl SHeader {
    pub fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_u32::<LittleEndian>(self.signature)?;
        w.write_u32::<LittleEndian>(self.version)?;
        w.write_u64::<LittleEndian>(self.total_uncompressed_animation_size)?;
        w.write_u32::<LittleEndian>(self.num_frames)?;
        w.write_u32::<LittleEndian>(self.flags)?;
        w.write_u8(self.block_compression_format)?;
        w.write_all(&[0u8; 3])?;
        for &min in &self.aabb_min {
            w.write_f32::<LittleEndian>(min)?;
        }
        for &max in &self.aabb_max {
            w.write_f32::<LittleEndian>(max)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct SFrameInfo {
    pub frame_type: u8,
    pub frame_time: f32,
    pub frame_offset: u64,
    pub frame_size: u32,
}

impl SFrameInfo {
    pub fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_u8(self.frame_type)?;
        w.write_all(&[0u8; 3])?;
        w.write_f32::<LittleEndian>(self.frame_time)?;
        w.write_u64::<LittleEndian>(self.frame_offset)?;
        w.write_u32::<LittleEndian>(self.frame_size)?;
        w.write_all(&[0u8; 4])?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct STemporalPredictorControl {
    pub acceleration: u8,
    pub index_frame_lerp_factor: u8,
    pub combine_factor: u8,
}

impl STemporalPredictorControl {
    pub fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_u8(self.acceleration)?;
        w.write_u8(self.index_frame_lerp_factor)?;
        w.write_u8(self.combine_factor)?;
        w.write_u8(0)?;
        Ok(())
    }
}
