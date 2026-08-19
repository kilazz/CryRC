use byteorder::{LittleEndian, WriteBytesExt};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

pub struct TgaIO;

impl TgaIO {
    pub fn save_tga_32bpp(
        path: &Path,
        width: u16,
        height: u16,
        bgra_pixels: &[u8],
    ) -> io::Result<()> {
        let file = File::create(path)?;
        let mut w = io::BufWriter::new(file);

        w.write_u8(0)?;
        w.write_u8(0)?;
        w.write_u8(2)?; // Uncompressed True-Color
        w.write_all(&[0u8; 5])?;
        w.write_u16::<LittleEndian>(0)?;
        w.write_u16::<LittleEndian>(0)?;
        w.write_u16::<LittleEndian>(width)?;
        w.write_u16::<LittleEndian>(height)?;
        w.write_u8(32)?;
        w.write_u8(8 | (1 << 5))?; // 8-bit alpha, top-to-bottom
        w.write_all(bgra_pixels)?;
        Ok(())
    }
}
