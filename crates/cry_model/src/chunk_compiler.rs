use super::chunk_file::{CChunkFile, ChunkDesc, ChunkType};
use byteorder::{ByteOrder, LittleEndian};
use cry_core::CgfUtil;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ChunkCompiler {
    pub target_version: u32,
}

impl Default for ChunkCompiler {
    fn default() -> Self {
        Self::new(0x0746)
    }
}

impl ChunkCompiler {
    pub fn new(target_version: u32) -> Self {
        Self { target_version }
    }

    pub fn process(&self, source_path: &Path, output_path: &Path) -> Result<Vec<PathBuf>, String> {
        let raw_data = fs::read(source_path)
            .map_err(|e| format!("Failed to read chunk file {:?}: {}", source_path, e))?;

        if raw_data.len() < 16 {
            return Err("Invalid chunk file: buffer too short".to_string());
        }

        let num_chunks = LittleEndian::read_u32(&raw_data[12..16]) as usize;
        let mut chunk_file = CChunkFile::new();

        let chunk_table_offset = 16;
        for i in 0..num_chunks {
            let entry_offset = chunk_table_offset + i * 24;
            if entry_offset + 24 > raw_data.len() {
                break;
            }

            let raw_chunk_type = LittleEndian::read_u32(&raw_data[entry_offset..entry_offset + 4]);
            let file_offset =
                LittleEndian::read_u32(&raw_data[entry_offset + 8..entry_offset + 12]) as usize;
            let size =
                LittleEndian::read_u32(&raw_data[entry_offset + 16..entry_offset + 20]) as usize;

            if file_offset + size <= raw_data.len() {
                let data = raw_data[file_offset..file_offset + size].to_vec();
                chunk_file.chunks.push(ChunkDesc {
                    chunk_type: ChunkType::from_u32(raw_chunk_type),
                    version: self.target_version,
                    id: i as u32,
                    data,
                });
            }
        }

        let bytes = chunk_file.build_bytes().map_err(|e| e.to_string())?;
        CgfUtil::write_temp_rename(output_path, &bytes).map_err(|e| e.to_string())?;

        Ok(vec![output_path.to_path_buf()])
    }
}
