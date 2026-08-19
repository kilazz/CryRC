use crate::math::vector::Vec4;
use crate::tables::qlut::q_lut_value;

/// Dynamic quantizer for BC7 with variable runtime channel bit depths.
#[derive(Debug, Clone)]
pub struct VQuantizer {
    pub grid: Vec4,
    pub grid_gap: Vec4,
    pub bits: [u32; 4],
}

impl VQuantizer {
    pub fn new(rb: u32, gb: u32, bb: u32, ab: u32, _sb: i32) -> Self {
        let rm = ((1 << rb) - 1) as f32;
        let gm = ((1 << gb) - 1) as f32;
        let bm = ((1 << bb) - 1) as f32;
        let am = if ab > 0 { ((1 << ab) - 1) as f32 } else { 0.0 };

        let rr = (1 << (8 - rb)) as f32;
        let gr = (1 << (8 - gb)) as f32;
        let br = (1 << (8 - bb)) as f32;
        let ar = if ab > 0 { (1 << (8 - ab)) as f32 } else { 0.0 };

        Self {
            grid: Vec4::new(rm, gm, bm, am),
            grid_gap: Vec4::new(
                (0.5 * rr * rm) / 255.0,
                (0.5 * gr * gm) / 255.0,
                (0.5 * br * bm) / 255.0,
                if ab > 0 { (0.5 * ar * am) / 255.0 } else { 0.0 },
            ),
            bits: [rb, gb, bb, ab],
        }
    }

    #[inline(always)]
    pub fn snap_to_lattice(&self, val: &Vec4) -> Vec4 {
        let clamped = val.clamp(0.0, 1.0);
        let p = (clamped * self.grid + self.grid_gap).floor();

        Vec4::new(
            q_lut_value(self.bits[0], p.x as usize),
            q_lut_value(self.bits[1], p.y as usize),
            q_lut_value(self.bits[2], p.z as usize),
            if self.bits[3] > 0 {
                q_lut_value(self.bits[3], p.w as usize)
            } else {
                1.0
            },
        )
    }
}
