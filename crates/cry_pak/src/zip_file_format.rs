pub const CDR_END_SIGNATURE: u32 = 0x06054b50;
pub const CDR_FILE_SIGNATURE: u32 = 0x02014b50;
pub const LOCAL_FILE_SIGNATURE: u32 = 0x04034b50;

pub const METHOD_STORE: u16 = 0;
pub const METHOD_DEFLATE: u16 = 8;
pub const METHOD_DEFLATE_AND_ENCRYPT: u16 = 11;
pub const TEA_DELTA: u32 = 0x9e3779b9;

pub fn btea(v: &mut [u32], n: i32, k: &[u32; 4]) {
    let rounds: u32 = (6 + 52 / n.abs()) as u32;
    let mut sum: u32 = 0;

    if n > 1 {
        let mut z = v[(n - 1) as usize];
        for _ in 0..rounds {
            sum = sum.wrapping_add(TEA_DELTA);
            let e = (sum >> 2) & 3;
            for p in 0..(n - 1) as usize {
                let y = v[p + 1];
                let mx = (((z >> 5) ^ (y << 2)).wrapping_add((y >> 3) ^ (z << 4)))
                    ^ ((sum ^ y).wrapping_add(k[(p & 3) ^ (e as usize)] ^ z));
                v[p] = v[p].wrapping_add(mx);
                z = v[p];
            }
            let y = v[0];
            let mx = (((z >> 5) ^ (y << 2)).wrapping_add((y >> 3) ^ (z << 4)))
                ^ ((sum ^ y).wrapping_add(k[((n - 1) as usize & 3) ^ (e as usize)] ^ z));
            v[(n - 1) as usize] = v[(n - 1) as usize].wrapping_add(mx);
            z = v[(n - 1) as usize];
        }
    } else if n < -1 {
        let num = (-n) as usize;
        sum = rounds.wrapping_mul(TEA_DELTA);
        let mut y = v[0];
        for _ in 0..rounds {
            let e = (sum >> 2) & 3;
            for p in (1..num).rev() {
                let z = v[p - 1];
                let mx = (((z >> 5) ^ (y << 2)).wrapping_add((y >> 3) ^ (z << 4)))
                    ^ ((sum ^ y).wrapping_add(k[(p & 3) ^ (e as usize)] ^ z));
                v[p] = v[p].wrapping_sub(mx);
                y = v[p];
            }
            let z = v[num - 1];
            let mx = (((z >> 5) ^ (y << 2)).wrapping_add((y >> 3) ^ (z << 4)))
                ^ ((sum ^ y).wrapping_add(k[(e as usize) & 3] ^ z));
            v[0] = v[0].wrapping_sub(mx);
            y = v[0];
            sum = sum.wrapping_sub(TEA_DELTA);
        }
    }
}

pub fn encrypt_buffer(buffer: &mut [u8], key: &[u32; 4]) {
    let len_u32 = buffer.len() / 4;
    if len_u32 < 2 {
        return;
    }
    let mut words = vec![0u32; len_u32];
    for (i, chunk) in buffer[..len_u32 * 4].chunks_exact(4).enumerate() {
        words[i] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }
    btea(&mut words, len_u32 as i32, key);
    for (i, &w) in words.iter().enumerate() {
        buffer[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
}
