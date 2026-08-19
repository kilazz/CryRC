/// Half-Float Log-Lattice Quantizer for BC6H / HDR encoding.
pub struct FQuantizer {
    pub trunc: u32,
    pub grid_prc: u32,
    pub grid_rnd: u32,
    pub grid_dltp: u32,
    pub grid_dltm: i32,
}

impl FQuantizer {
    pub fn new(tb: u32, db: u32) -> Self {
        let grid_prc_raw = (1 << tb) - 1;
        let grid_rnd = grid_prc_raw >> 1;
        let grid_prc = !grid_prc_raw;

        let grid_dlt = if db > 0 {
            (1 << (db & 0xFF)) - 1
        } else {
            (1 << (16 - tb)) - 1
        };

        let grid_dltp = grid_dlt >> 1;
        let grid_dltm = -(grid_dltp as i32) - 1;

        Self {
            trunc: tb,
            grid_prc,
            grid_rnd,
            grid_dltp,
            grid_dltm,
        }
    }

    #[inline(always)]
    pub fn mask_lattice(&self, p: u32) -> u32 {
        (p & self.grid_prc) + self.grid_rnd
    }

    #[inline(always)]
    pub fn delta_lattice(&self, p: i32, b: i32) -> u32 {
        let mut d = (p - b) >> self.trunc;
        d = d.clamp(self.grid_dltm, self.grid_dltp as i32);
        d <<= self.trunc;
        ((d + b) & 0x0000_FFFF) as u32
    }
}
