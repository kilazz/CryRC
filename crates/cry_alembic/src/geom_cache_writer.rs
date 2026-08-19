use super::geom_cache_block_compressor::{IGeomCacheBlockCompressor, create_block_compressor};
use super::geom_cache_file::*;
use byteorder::{LittleEndian, WriteBytesExt};
use cry_core::math::AABB;
use std::fs::File;
use std::io::{self, Seek, SeekFrom, Write};
use std::path::Path;

pub struct GeomCacheWriter {
    file: File,
    compressor: Box<dyn IGeomCacheBlockCompressor>,
    compression_format: BlockCompressionFormat,
    header: SHeader,
    frame_infos: Vec<SFrameInfo>,
    total_uncompressed: u64,
    frame_infos_offset: u64,
}

impl GeomCacheWriter {
    pub fn new(
        path: &Path,
        num_frames: u32,
        format: BlockCompressionFormat,
        playback_from_memory: bool,
        use_32bit_indices: bool,
    ) -> io::Result<Self> {
        let mut file = File::create(path)?;
        let mut flags = 0u32;
        if playback_from_memory {
            flags |= FILE_HEADER_FLAG_PLAYBACK_FROM_MEMORY;
        }
        if use_32bit_indices {
            flags |= FILE_HEADER_FLAG_32BIT_INDICES;
        }

        let header = SHeader {
            signature: 0,
            version: CAX_FILE_VERSION,
            num_frames,
            block_compression_format: format as u8,
            flags,
            ..Default::default()
        };

        header.write(&mut file)?;
        let frame_infos_offset = file.stream_position()?;
        file.write_all(&vec![0u8; num_frames as usize * 24])?;
        let compressor = create_block_compressor(format);

        Ok(Self {
            file,
            compressor,
            compression_format: format,
            header,
            frame_infos: Vec::with_capacity(num_frames as usize),
            total_uncompressed: 0,
            frame_infos_offset,
        })
    }

    pub fn write_block(&mut self, data: &[u8], compress: bool) -> io::Result<(u64, u32)> {
        let offset = self.file.stream_position()?;
        self.total_uncompressed += data.len() as u64;

        if compress && self.compression_format != BlockCompressionFormat::None {
            let compressed = self.compressor.compress(data).map_err(io::Error::other)?;
            self.file.write_u32::<LittleEndian>(data.len() as u32)?;
            self.file
                .write_u32::<LittleEndian>(compressed.len() as u32)?;
            self.file.write_all(&compressed)?;
            Ok((offset, (8 + compressed.len()) as u32))
        } else {
            self.file.write_all(data)?;
            Ok((offset, data.len() as u32))
        }
    }

    pub fn add_frame_info(&mut self, frame_type: FrameType, time: f32, offset: u64, size: u32) {
        self.frame_infos.push(SFrameInfo {
            frame_type: frame_type as u8,
            frame_time: time,
            frame_offset: offset,
            frame_size: size,
        });
    }

    pub fn finish(mut self, aabb: &AABB) -> io::Result<()> {
        self.file.seek(SeekFrom::Start(self.frame_infos_offset))?;
        for info in &self.frame_infos {
            info.write(&mut self.file)?;
        }

        self.header.signature = CAX_FILE_SIGNATURE;
        self.header.aabb_min = [aabb.min.x, aabb.min.y, aabb.min.z];
        self.header.aabb_max = [aabb.max.x, aabb.max.y, aabb.max.z];
        self.header.total_uncompressed_animation_size = self.total_uncompressed;

        self.file.seek(SeekFrom::Start(0))?;
        self.header.write(&mut self.file)?;
        self.file.flush()?;
        Ok(())
    }
}
