use super::photoshop_irb::form_photoshop_data_block;
use byteorder::{LittleEndian, WriteBytesExt};
use flate2::Compression;
use flate2::write::ZlibEncoder;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

pub struct CryTiffWriter;

impl CryTiffWriter {
    pub fn save_crytif_16bit(
        path: &Path,
        width: u32,
        height: u32,
        rgba16_buffer: &[u16],
        compiler_settings: &str,
    ) -> io::Result<()> {
        let expected_samples = (width * height * 4) as usize;
        if rgba16_buffer.len() != expected_samples {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid dimensions or buffer length for 16-bit RGBA image",
            ));
        }

        let file = File::create(path)?;
        let mut writer = io::BufWriter::new(file);

        let mut raw_bytes = Vec::with_capacity(rgba16_buffer.len() * 2);
        for &sample in rgba16_buffer {
            raw_bytes.write_u16::<LittleEndian>(sample)?;
        }

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&raw_bytes)?;
        let compressed_strip = encoder.finish()?;

        let photoshop_data = if !compiler_settings.is_empty() {
            form_photoshop_data_block(compiler_settings)
        } else {
            Vec::new()
        };

        writer.write_all(b"II\x2A\x00")?;
        let ifd_offset = 8u32;
        writer.write_u32::<LittleEndian>(ifd_offset)?;

        let has_ps = !photoshop_data.is_empty();
        let num_entries = if has_ps { 14u16 } else { 13u16 };
        writer.write_u16::<LittleEndian>(num_entries)?;

        let mut extra_data = Vec::new();
        let bits_per_sample_offset =
            8 + 2 + (num_entries as u32 * 12) + 4 + extra_data.len() as u32;
        for _ in 0..4 {
            extra_data.write_u16::<LittleEndian>(16)?;
        }

        let ps_data_offset = 8 + 2 + (num_entries as u32 * 12) + 4 + extra_data.len() as u32;
        extra_data.write_all(&photoshop_data)?;

        let strip_data_offset = 8 + 2 + (num_entries as u32 * 12) + 4 + extra_data.len() as u32;
        extra_data.write_all(&compressed_strip)?;

        let mut write_tag =
            |tag: u16, tag_type: u16, count: u32, val_or_offset: u32| -> io::Result<()> {
                writer.write_u16::<LittleEndian>(tag)?;
                writer.write_u16::<LittleEndian>(tag_type)?;
                writer.write_u32::<LittleEndian>(count)?;
                writer.write_u32::<LittleEndian>(val_or_offset)?;
                Ok(())
            };

        write_tag(256, 4, 1, width)?;
        write_tag(257, 4, 1, height)?;
        write_tag(258, 3, 4, bits_per_sample_offset)?;
        write_tag(259, 3, 1, 8)?;
        write_tag(262, 3, 1, 2)?;
        write_tag(273, 4, 1, strip_data_offset)?;
        write_tag(274, 3, 1, 1)?;
        write_tag(277, 3, 1, 4)?;
        write_tag(278, 4, 1, height)?;
        write_tag(279, 4, 1, compressed_strip.len() as u32)?;
        write_tag(284, 3, 1, 1)?;
        write_tag(317, 3, 1, 1)?;
        write_tag(339, 3, 1, 1)?;

        if has_ps {
            write_tag(34377, 1, photoshop_data.len() as u32, ps_data_offset)?;
        }

        writer.write_u32::<LittleEndian>(0)?;
        writer.write_all(&extra_data)?;
        Ok(())
    }
}
