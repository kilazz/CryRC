use super::chunk_file::{CChunkFile, ChunkType};
use byteorder::{LittleEndian, WriteBytesExt};

pub const EXPORT_FLAG_MERGE_ALL_NODES: u32 = 1 << 0;
pub const EXPORT_FLAG_USE_CUSTOM_NORMALS: u32 = 1 << 1;
pub const EXPORT_FLAG_WANT_F32_VERTICES: u32 = 1 << 2;
pub const EXPORT_FLAG_EIGHT_WEIGHTS_PER_VERTEX: u32 = 1 << 3;
pub const EXPORT_FLAG_MAKE_VCLOTH: u32 = 1 << 4;

#[derive(Debug, Clone)]
pub struct ExportFlags {
    pub merge_all_nodes: bool,
    pub use_custom_normals: bool,
    pub want_f32_vertices: bool,
    pub eight_weights_per_vertex: bool,
    pub make_vcloth: bool,
    pub rc_version: [u16; 4],
    pub rc_version_string: String,
}

impl Default for ExportFlags {
    fn default() -> Self {
        Self {
            merge_all_nodes: true,
            use_custom_normals: true,
            want_f32_vertices: false,
            eight_weights_per_vertex: false,
            make_vcloth: false,
            rc_version: [1, 2, 0, 0],
            rc_version_string: "CryEngine RC (Rust)".to_string(),
        }
    }
}

pub struct SaverAnim;

impl SaverAnim {
    pub fn save_export_flags(chunk_file: &mut CChunkFile, flags: &ExportFlags) {
        let mut data = Vec::new();
        let mut bitflags = 0u32;

        if flags.merge_all_nodes {
            bitflags |= EXPORT_FLAG_MERGE_ALL_NODES;
        }
        if flags.use_custom_normals {
            bitflags |= EXPORT_FLAG_USE_CUSTOM_NORMALS;
        }
        if flags.want_f32_vertices {
            bitflags |= EXPORT_FLAG_WANT_F32_VERTICES;
        }
        if flags.eight_weights_per_vertex {
            bitflags |= EXPORT_FLAG_EIGHT_WEIGHTS_PER_VERTEX;
        }
        if flags.make_vcloth {
            bitflags |= EXPORT_FLAG_MAKE_VCLOTH;
        }

        data.write_u32::<LittleEndian>(bitflags).unwrap();
        data.write_u32::<LittleEndian>(0).unwrap();

        for &v in &flags.rc_version {
            data.write_u16::<LittleEndian>(v).unwrap();
        }

        let mut rc_str = [0u8; 32];
        let bytes = flags.rc_version_string.as_bytes();
        let len = bytes.len().min(31);
        rc_str[..len].copy_from_slice(&bytes[..len]);
        data.extend_from_slice(&rc_str);

        chunk_file.add_chunk(ChunkType::ExportFlags, 0x0923, data);
    }

    pub fn save_timing(chunk_file: &mut CChunkFile, start_sample: i32, end_sample: i32) {
        let mut data = Vec::new();
        let num_samples = (end_sample - start_sample + 1).max(1) as u32;
        let samples_per_sec = 30.0f32;
        let start_time_sec = start_sample as f32 / samples_per_sec;

        data.write_u32::<LittleEndian>(num_samples).unwrap();
        data.write_f32::<LittleEndian>(samples_per_sec).unwrap();
        data.write_f32::<LittleEndian>(start_time_sec).unwrap();

        chunk_file.add_chunk(ChunkType::Timing, 0x0919, data);
    }
}
