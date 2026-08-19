use super::texture_helper::TextureHelper;
use byteorder::{ByteOrder, LittleEndian};
use cry_core::CgfUtil;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_NUM_PERSISTENT_MIPS: usize = 3;

#[derive(Debug, Clone)]
pub struct TextureSplitterConfig {
    pub persistent_mips: usize,
    pub dont_split: bool,
}

impl Default for TextureSplitterConfig {
    fn default() -> Self {
        Self {
            persistent_mips: DEFAULT_NUM_PERSISTENT_MIPS,
            dont_split: false,
        }
    }
}

pub struct TextureSplitter {
    pub config: TextureSplitterConfig,
}

impl TextureSplitter {
    pub fn new(config: TextureSplitterConfig) -> Self {
        Self { config }
    }

    pub fn process_dds_file(
        &self,
        input_path: &Path,
        output_base_path: &Path,
    ) -> Result<Vec<PathBuf>, String> {
        let raw_data = fs::read(input_path).map_err(|e| format!("Failed to read DDS: {}", e))?;

        if raw_data.len() < 128 || &raw_data[0..4] != b"DDS " {
            return Err("Invalid DDS signature".to_string());
        }

        let height = LittleEndian::read_u32(&raw_data[12..16]) as usize;
        let width = LittleEndian::read_u32(&raw_data[16..20]) as usize;
        let depth = (LittleEndian::read_u32(&raw_data[24..28]) as usize).max(1);
        let mip_count = (LittleEndian::read_u32(&raw_data[28..32]) as usize).max(1);

        let caps2 = LittleEndian::read_u32(&raw_data[112..116]);
        let is_cubemap = (caps2 & 0xFE00) != 0;
        let sides = if is_cubemap { 6 } else { 1 };

        let mut fourcc = [0u8; 4];
        fourcc.copy_from_slice(&raw_data[84..88]);

        let has_dx10_header = &fourcc == b"DX10";
        let header_size = if has_dx10_header { 148 } else { 128 };
        let dxgi_format = if has_dx10_header && raw_data.len() >= 132 {
            LittleEndian::read_u32(&raw_data[128..132])
        } else {
            0
        };

        if self.config.dont_split || mip_count <= self.config.persistent_mips {
            CgfUtil::write_temp_rename(output_base_path, &raw_data)
                .map_err(|e| format!("Failed to write unsplit DDS: {}", e))?;
            return Ok(vec![output_base_path.to_path_buf()]);
        }

        let mut saved_files = Vec::new();
        let payload = &raw_data[header_size..];

        let mut mip_sizes = Vec::with_capacity(mip_count);
        let mut cur_w = width;
        let mut cur_h = height;
        let mut cur_d = depth;

        for _ in 0..mip_count {
            let s = TextureHelper::mip_data_size(cur_w, cur_h, cur_d, &fourcc, dxgi_format);
            mip_sizes.push(s);
            cur_w = (cur_w / 2).max(1);
            cur_h = (cur_h / 2).max(1);
            cur_d = (cur_d / 2).max(1);
        }

        let persistent_start_mip = mip_count.saturating_sub(self.config.persistent_mips);
        let mut mip_offset = 0;

        for (mip_idx, &single_mip_size) in mip_sizes.iter().enumerate().take(persistent_start_mip) {
            let chunk_num = persistent_start_mip - mip_idx;
            let chunk_filename = format!("{}.{}", output_base_path.to_string_lossy(), chunk_num);
            let chunk_path = PathBuf::from(chunk_filename);
            let mut chunk_data = Vec::with_capacity(single_mip_size * sides);

            for side in 0..sides {
                let side_total_size = TextureHelper::texture_data_size(
                    width,
                    height,
                    depth,
                    mip_count,
                    &fourcc,
                    dxgi_format,
                );
                let side_mip_off = side * side_total_size + mip_offset;

                if side_mip_off + single_mip_size <= payload.len() {
                    chunk_data
                        .extend_from_slice(&payload[side_mip_off..side_mip_off + single_mip_size]);
                }
            }

            if !chunk_data.is_empty() {
                fs::write(&chunk_path, &chunk_data)
                    .map_err(|e| format!("Failed to write DDS chunk {:?}: {}", chunk_path, e))?;
                saved_files.push(chunk_path);
            }
            mip_offset += single_mip_size;
        }

        let mut base_payload = Vec::new();
        for side in 0..sides {
            let side_total_size = TextureHelper::texture_data_size(
                width,
                height,
                depth,
                mip_count,
                &fourcc,
                dxgi_format,
            );
            let mut side_pers_off = side * side_total_size + mip_offset;

            for &s in mip_sizes.iter().take(mip_count).skip(persistent_start_mip) {
                if side_pers_off + s <= payload.len() {
                    base_payload.extend_from_slice(&payload[side_pers_off..side_pers_off + s]);
                    side_pers_off += s;
                }
            }
        }

        let mut base_file_data = raw_data[..header_size].to_vec();
        let mut reserved1 = LittleEndian::read_u32(&base_file_data[32..36]);
        reserved1 |= 0x10000; // EIF_SPLITTED
        LittleEndian::write_u32(&mut base_file_data[32..36], reserved1);

        base_file_data.extend_from_slice(&base_payload);

        CgfUtil::write_temp_rename(output_base_path, &base_file_data)
            .map_err(|e| format!("Failed to write base DDS: {}", e))?;
        saved_files.push(output_base_path.to_path_buf());
        Ok(saved_files)
    }

    pub fn decompress_dds_to_tif(input_path: &Path, output_path: &Path) -> Result<PathBuf, String> {
        let dyn_img = image::open(input_path).map_err(|e| format!("Failed to open DDS: {}", e))?;
        let out_tif = output_path.with_extension("tif");
        dyn_img
            .save_with_format(&out_tif, image::ImageFormat::Tiff)
            .map_err(|e| format!("Failed to save decompressed TIF: {}", e))?;
        Ok(out_tif)
    }
}
