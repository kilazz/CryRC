use byteorder::{LittleEndian, WriteBytesExt};
use std::io::{self, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ChunkType {
    Any = 0,
    Mesh = 0x1000,
    Helper = 0x1001,
    VertAnim = 0x1002,
    BoneAnim = 0x1003,
    GeomNameList = 0x1004,
    BoneNameList = 0x1005,
    MeshMorphTarget = 0x1006,
    TimedStats = 0x1007,
    ExportFlags = 0x1008,
    DataStream = 0x1009,
    MeshPhysicsData = 0x100A,
    Controller = 0x100B,
    Node = 0x100C,
    Timing = 0x1011,
    SpeedInfo = 0x1012,
    FootPlantInfo = 0x1013,
    Bones = 0x1014,
    BoneInitialPos = 0x1015,
    CompiledBones = 0x1016,
    CompiledPhysicalBones = 0x1017,
    CompiledMorphTargets = 0x1018,
    CompiledPhysicalProxies = 0x1019,
    CompiledIntFaces = 0x101A,
    CompiledIntSkinVertices = 0x101B,
    CompiledExt2IntMap = 0x101C,
    BonesBoxes = 0x101D,
    GlobalAnimationHeaderCAF = 0x1020,
    GlobalAnimationHeaderAIM = 0x1021,
}

impl ChunkType {
    #[inline]
    pub fn from_u32(val: u32) -> Self {
        match val {
            0x1000 => ChunkType::Mesh,
            0x1001 => ChunkType::Helper,
            0x1002 => ChunkType::VertAnim,
            0x1003 => ChunkType::BoneAnim,
            0x1004 => ChunkType::GeomNameList,
            0x1005 => ChunkType::BoneNameList,
            0x1006 => ChunkType::MeshMorphTarget,
            0x1007 => ChunkType::TimedStats,
            0x1008 => ChunkType::ExportFlags,
            0x1009 => ChunkType::DataStream,
            0x100A | 0x100E => ChunkType::MeshPhysicsData,
            0x100B => ChunkType::Controller,
            0x100C => ChunkType::Node,
            0x1011 => ChunkType::Timing,
            0x1012 => ChunkType::SpeedInfo,
            0x1013 => ChunkType::FootPlantInfo,
            0x1014 => ChunkType::Bones,
            0x1015 => ChunkType::BoneInitialPos,
            0x1016 => ChunkType::CompiledBones,
            0x1017 => ChunkType::CompiledPhysicalBones,
            0x1018 => ChunkType::CompiledMorphTargets,
            0x1019 => ChunkType::CompiledPhysicalProxies,
            0x101A => ChunkType::CompiledIntFaces,
            0x101B => ChunkType::CompiledIntSkinVertices,
            0x101C => ChunkType::CompiledExt2IntMap,
            0x101D => ChunkType::BonesBoxes,
            0x1020 => ChunkType::GlobalAnimationHeaderCAF,
            0x1021 => ChunkType::GlobalAnimationHeaderAIM,
            _ => ChunkType::Any,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChunkDesc {
    pub chunk_type: ChunkType,
    pub version: u32,
    pub id: u32,
    pub data: Vec<u8>,
}

#[derive(Default)]
pub struct CChunkFile {
    pub chunks: Vec<ChunkDesc>,
}

impl CChunkFile {
    pub fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    pub fn add_chunk(&mut self, chunk_type: ChunkType, version: u32, data: Vec<u8>) -> u32 {
        let id = self.chunks.len() as u32;
        self.chunks.push(ChunkDesc {
            chunk_type,
            version,
            id,
            data,
        });
        id
    }

    pub fn build_bytes(&self) -> io::Result<Vec<u8>> {
        let mut buffer = Vec::new();

        buffer.write_all(b"CryTek\x00\x00")?;
        buffer.write_u32::<LittleEndian>(0x0745)?;
        buffer.write_u32::<LittleEndian>(self.chunks.len() as u32)?;

        let header_size = buffer.len();
        let chunk_table_size = self.chunks.len() * 24;

        let mut current_data_offset = (header_size + chunk_table_size) as u32;
        let mut chunk_offsets = Vec::new();

        for chunk in &self.chunks {
            chunk_offsets.push(current_data_offset);
            current_data_offset += chunk.data.len() as u32;
            let pad = (4 - (chunk.data.len() % 4)) % 4;
            current_data_offset += pad as u32;
        }

        for (i, chunk) in self.chunks.iter().enumerate() {
            buffer.write_u32::<LittleEndian>(chunk.chunk_type as u32)?;
            buffer.write_u32::<LittleEndian>(chunk.version)?;
            buffer.write_u32::<LittleEndian>(chunk_offsets[i])?;
            buffer.write_u32::<LittleEndian>(chunk.id)?;
            buffer.write_u32::<LittleEndian>(chunk.data.len() as u32)?;
            buffer.write_u32::<LittleEndian>(0)?;
        }

        for chunk in &self.chunks {
            buffer.write_all(&chunk.data)?;
            let pad = (4 - (chunk.data.len() % 4)) % 4;
            buffer.resize(buffer.len() + pad, 0);
        }

        Ok(buffer)
    }
}
