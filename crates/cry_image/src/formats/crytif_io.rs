use super::iptc_header::{FIELD_SPECIAL_INSTRUCTIONS, IptcHeader};
use byteorder::{BigEndian, ByteOrder, LittleEndian, WriteBytesExt};
use flate2::Compression;
use flate2::write::ZlibEncoder;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

pub struct CryTifIO;

impl CryTifIO {
    pub fn read_special_instructions(path: &Path) -> Result<String, String> {
        let data = std::fs::read(path).map_err(|e| format!("Failed to read TIFF: {}", e))?;
        if data.len() < 8 {
            return Err("TIFF file too small".to_string());
        }

        if let Some(pos) = data.windows(4).position(|w| w == b"8BIM") {
            let mut curr = pos;
            while curr + 12 < data.len() && &data[curr..curr + 4] == b"8BIM" {
                curr += 4;
                let res_id = BigEndian::read_u16(&data[curr..curr + 2]);
                curr += 2;

                let name_len = data[curr] as usize;
                curr += 1 + name_len;
                if curr % 2 != 0 {
                    curr += 1;
                }

                let block_size = BigEndian::read_u32(&data[curr..curr + 4]) as usize;
                curr += 4;

                if res_id == 0x0404 && curr + block_size <= data.len() {
                    let mut iptc = IptcHeader::new();
                    iptc.parse(&data[curr..curr + block_size]);
                    let instructions = iptc.get_combined_fields(FIELD_SPECIAL_INSTRUCTIONS, " ");
                    if !instructions.is_empty() {
                        return Ok(instructions);
                    }
                }

                curr += block_size;
                if curr % 2 != 0 {
                    curr += 1;
                }
            }
        }
        Ok(String::new())
    }

    pub fn save_crytif_rgba8(
        path: &Path,
        width: u32,
        height: u32,
        rgba_pixels: &[u8],
        instructions: &str,
    ) -> io::Result<()> {
        let file = File::create(path)?;
        let mut w = io::BufWriter::new(file);

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(rgba_pixels)?;
        let compressed_strip = encoder.finish()?;

        let mut iptc = IptcHeader::new();
        iptc.fields.insert(
            FIELD_SPECIAL_INSTRUCTIONS,
            vec![instructions.as_bytes().to_vec()],
        );
        let iptc_bytes = iptc.build_bytes();

        let mut photoshop_data = Vec::new();
        photoshop_data.extend_from_slice(b"8BIM");
        photoshop_data.write_u16::<BigEndian>(0x0404)?;
        photoshop_data.write_u16::<BigEndian>(0x0000)?;
        photoshop_data.write_u32::<BigEndian>(iptc_bytes.len() as u32)?;
        photoshop_data.extend_from_slice(&iptc_bytes);
        if photoshop_data.len() % 2 != 0 {
            photoshop_data.push(0);
        }

        w.write_all(b"II\x2A\x00")?;
        let ifd_offset = 8u32;
        w.write_u32::<LittleEndian>(ifd_offset)?;

        let num_entries = 15u16; // 15 Sorted TIFF Tags
        w.write_u16::<LittleEndian>(num_entries)?;

        let mut extra_data = Vec::new();
        let bits_per_sample_off = 8 + 2 + (num_entries as u32 * 12) + 4 + extra_data.len() as u32;
        for _ in 0..4 {
            extra_data.write_u16::<LittleEndian>(8)?;
        }

        let ps_data_off = 8 + 2 + (num_entries as u32 * 12) + 4 + extra_data.len() as u32;
        extra_data.write_all(&photoshop_data)?;

        let strip_off = 8 + 2 + (num_entries as u32 * 12) + 4 + extra_data.len() as u32;
        extra_data.write_all(&compressed_strip)?;

        let mut write_tag =
            |tag: u16, t_type: u16, count: u32, val_or_off: u32| -> io::Result<()> {
                w.write_u16::<LittleEndian>(tag)?;
                w.write_u16::<LittleEndian>(t_type)?;
                w.write_u32::<LittleEndian>(count)?;
                w.write_u32::<LittleEndian>(val_or_off)?;
                Ok(())
            };

        write_tag(256, 4, 1, width)?;
        write_tag(257, 4, 1, height)?;
        write_tag(258, 3, 4, bits_per_sample_off)?;
        write_tag(259, 3, 1, 8)?;
        write_tag(262, 3, 1, 2)?;
        write_tag(273, 4, 1, strip_off)?;
        write_tag(274, 3, 1, 1)?;
        write_tag(277, 3, 1, 4)?;
        write_tag(278, 4, 1, height)?;
        write_tag(279, 4, 1, compressed_strip.len() as u32)?;
        write_tag(284, 3, 1, 1)?;
        write_tag(317, 3, 1, 1)?;
        write_tag(338, 3, 1, 2)?; // Tag 338: ExtraSamples = 2 (Unassociated Alpha)
        write_tag(339, 3, 1, 1)?;
        write_tag(34377, 1, photoshop_data.len() as u32, ps_data_off)?;

        w.write_u32::<LittleEndian>(0)?;
        w.write_all(&extra_data)?;
        Ok(())
    }
}
