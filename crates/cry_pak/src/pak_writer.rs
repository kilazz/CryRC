use super::zip_file_format::*;
use byteorder::{LittleEndian, WriteBytesExt};
use crc32fast::Hasher;
use flate2::Compression;
use flate2::write::DeflateEncoder;
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Seek, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PakFileInfo {
    pub relative_path: String,
    pub disk_path: PathBuf,
}

pub struct PakWriter;

impl PakWriter {
    pub fn calculate_aligned_header_offset(
        filename: &str,
        current_offset: u64,
        alignment: usize,
    ) -> u64 {
        if current_offset == 0 || alignment <= 1 {
            return current_offset;
        }
        let total_header_size = (30 + filename.len()) as u64;
        let remainder = (current_offset + total_header_size) % (alignment as u64);
        let aligned_data_offset = if remainder != 0 {
            (current_offset + total_header_size) + (alignment as u64 - remainder)
        } else {
            current_offset + total_header_size
        };
        aligned_data_offset - total_header_size
    }

    pub fn create_pak(
        pak_path: &Path,
        files: &[PakFileInfo],
        alignment: usize,
        encrypt: bool,
        encryption_key: Option<&[u32; 4]>,
    ) -> io::Result<()> {
        let temp_pak = pak_path.with_extension("$pak$");
        if let Some(parent) = temp_pak.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = io::BufWriter::new(File::create(&temp_pak)?);
        let mut cdr_entries = Vec::with_capacity(files.len());
        let mut read_buffer = vec![0u8; 64 * 1024];

        for item in files {
            if !item.disk_path.exists() {
                continue;
            }

            let src_file = File::open(&item.disk_path)?;
            let uncompressed_size = src_file.metadata()?.len();
            let mut reader = BufReader::new(src_file);

            let mut hasher = Hasher::new();
            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());

            loop {
                let bytes_read = reader.read(&mut read_buffer)?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&read_buffer[..bytes_read]);
                encoder.write_all(&read_buffer[..bytes_read])?;
            }

            let crc = hasher.finalize();
            let mut compressed_data = encoder.finish()?;

            let method = if encrypt {
                if let Some(key) = encryption_key {
                    encrypt_buffer(&mut compressed_data, key);
                    METHOD_DEFLATE_AND_ENCRYPT
                } else {
                    METHOD_DEFLATE
                }
            } else {
                METHOD_DEFLATE
            };

            let cur_pos = file.stream_position()?;
            let aligned_header_offset =
                Self::calculate_aligned_header_offset(&item.relative_path, cur_pos, alignment);

            if aligned_header_offset > cur_pos {
                let pad = (aligned_header_offset - cur_pos) as usize;
                file.write_all(&vec![0u8; pad])?;
            }

            let local_header_offset = file.stream_position()?;

            file.write_u32::<LittleEndian>(LOCAL_FILE_SIGNATURE)?;
            file.write_u16::<LittleEndian>(20)?;
            file.write_u16::<LittleEndian>(0)?;
            file.write_u16::<LittleEndian>(method)?;
            file.write_u16::<LittleEndian>(0)?;
            file.write_u16::<LittleEndian>(0)?;
            file.write_u32::<LittleEndian>(crc)?;
            file.write_u32::<LittleEndian>(compressed_data.len() as u32)?;
            file.write_u32::<LittleEndian>(uncompressed_size as u32)?;
            file.write_u16::<LittleEndian>(item.relative_path.len() as u16)?;
            file.write_u16::<LittleEndian>(0)?;
            file.write_all(item.relative_path.as_bytes())?;
            file.write_all(&compressed_data)?;

            cdr_entries.push((
                item.relative_path.clone(),
                crc,
                compressed_data.len() as u32,
                uncompressed_size as u32,
                method,
                local_header_offset,
            ));
        }

        let cdr_offset = file.stream_position()?;
        for (name, crc, comp_sz, uncomp_sz, method, offset) in &cdr_entries {
            file.write_u32::<LittleEndian>(CDR_FILE_SIGNATURE)?;
            file.write_u16::<LittleEndian>(20)?;
            file.write_u16::<LittleEndian>(20)?;
            file.write_u16::<LittleEndian>(0)?;
            file.write_u16::<LittleEndian>(*method)?;
            file.write_u16::<LittleEndian>(0)?;
            file.write_u16::<LittleEndian>(0)?;
            file.write_u32::<LittleEndian>(*crc)?;
            file.write_u32::<LittleEndian>(*comp_sz)?;
            file.write_u32::<LittleEndian>(*uncomp_sz)?;
            file.write_u16::<LittleEndian>(name.len() as u16)?;
            file.write_u16::<LittleEndian>(0)?;
            file.write_u16::<LittleEndian>(0)?;
            file.write_u16::<LittleEndian>(0)?;
            file.write_u16::<LittleEndian>(0)?;
            file.write_u32::<LittleEndian>(0)?;
            file.write_u32::<LittleEndian>(*offset as u32)?;
            file.write_all(name.as_bytes())?;
        }

        let cdr_size = file.stream_position()? - cdr_offset;
        file.write_u32::<LittleEndian>(CDR_END_SIGNATURE)?;
        file.write_u16::<LittleEndian>(if encrypt { 1 << 15 } else { 0 })?;
        file.write_u16::<LittleEndian>(0)?;
        file.write_u16::<LittleEndian>(cdr_entries.len() as u16)?;
        file.write_u16::<LittleEndian>(cdr_entries.len() as u16)?;
        file.write_u32::<LittleEndian>(cdr_size as u32)?;
        file.write_u32::<LittleEndian>(cdr_offset as u32)?;
        file.write_u16::<LittleEndian>(0)?;

        file.flush()?;
        drop(file);

        if pak_path.exists() {
            let _ = fs::remove_file(pak_path);
        }
        fs::rename(&temp_pak, pak_path)?;
        Ok(())
    }
}
