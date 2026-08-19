use super::chunk_file::{CChunkFile, ChunkType};
use byteorder::{LittleEndian, WriteBytesExt};
use cry_core::math::Matrix34;

pub struct SaverANM;

impl SaverANM {
    pub fn save_node(
        chunk_file: &mut CChunkFile,
        name: &str,
        local_tm: &Matrix34,
        pos_cont_id: i32,
        rot_cont_id: i32,
        scl_cont_id: i32,
    ) {
        let mut data = Vec::new();

        let mut name_buf = [0u8; 64];
        let bytes = name.as_bytes();
        let len = bytes.len().min(63);
        name_buf[..len].copy_from_slice(&bytes[..len]);
        data.extend_from_slice(&name_buf);

        data.write_i32::<LittleEndian>(-1).unwrap();
        data.write_i32::<LittleEndian>(-1).unwrap();
        data.write_u32::<LittleEndian>(0).unwrap();
        data.write_i32::<LittleEndian>(-1).unwrap();

        let tm = [
            local_tm.m[0][0],
            local_tm.m[1][0],
            local_tm.m[2][0],
            0.0,
            local_tm.m[0][1],
            local_tm.m[1][1],
            local_tm.m[2][1],
            0.0,
            local_tm.m[0][2],
            local_tm.m[1][2],
            local_tm.m[2][2],
            0.0,
            local_tm.m[0][3] * 100.0,
            local_tm.m[1][3] * 100.0,
            local_tm.m[2][3] * 100.0,
            1.0,
        ];

        for &val in &tm {
            data.write_f32::<LittleEndian>(val).unwrap();
        }

        data.write_i32::<LittleEndian>(pos_cont_id).unwrap();
        data.write_i32::<LittleEndian>(rot_cont_id).unwrap();
        data.write_i32::<LittleEndian>(scl_cont_id).unwrap();
        data.write_u32::<LittleEndian>(0).unwrap();

        chunk_file.add_chunk(ChunkType::Node, 0x0824, data);
    }
}
