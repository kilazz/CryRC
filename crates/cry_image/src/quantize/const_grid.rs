use crate::math::vector::{Vec3, Vec4};
use crate::tables::qlut::q_lut_value;

/// 3-channel compile-time static quantizer (RGB).
pub struct Quantizer3<const RB: u32, const GB: u32, const BB: u32>;

impl<const RB: u32, const GB: u32, const BB: u32> Quantizer3<RB, GB, BB> {
    const RM: u32 = (1 << RB) - 1;
    const GM: u32 = (1 << GB) - 1;
    const BM: u32 = (1 << BB) - 1;

    const RR: u32 = 1 << (8 - RB);
    const GR: u32 = 1 << (8 - GB);
    const BR: u32 = 1 << (8 - BB);

    const GRID: [f32; 3] = [Self::RM as f32, Self::GM as f32, Self::BM as f32];
    const GRID_INV: [u32; 3] = [1 << RB, 1 << GB, 1 << BB];
    const GRID_GAP: [f32; 3] = [
        (0.5 * (Self::RR * Self::RM) as f32) / 255.0,
        (0.5 * (Self::GR * Self::GM) as f32) / 255.0,
        (0.5 * (Self::BR * Self::BM) as f32) / 255.0,
    ];

    #[inline(always)]
    pub fn snap_to_lattice(val: &Vec3) -> Vec3 {
        let clamped = val.clamp(0.0, 1.0);
        let p_r = (Self::GRID[0] * clamped.x + Self::GRID_GAP[0]).floor() as usize;
        let p_g = (Self::GRID[1] * clamped.y + Self::GRID_GAP[1]).floor() as usize;
        let p_b = (Self::GRID[2] * clamped.z + Self::GRID_GAP[2]).floor() as usize;

        Vec3::new(
            q_lut_value(RB, p_r.min(Self::RM as usize)),
            q_lut_value(GB, p_g.min(Self::GM as usize)),
            q_lut_value(BB, p_b.min(Self::BM as usize)),
        )
    }

    #[inline(always)]
    pub fn quantize_to_int(val: &Vec3) -> [u32; 3] {
        let qf = Self::snap_to_lattice(val);
        [
            (((qf.x * 255.0).floor() as u32) * Self::GRID_INV[0]) >> 8,
            (((qf.y * 255.0).floor() as u32) * Self::GRID_INV[1]) >> 8,
            (((qf.z * 255.0).floor() as u32) * Self::GRID_INV[2]) >> 8,
        ]
    }
}

/// 4-channel compile-time static quantizer (RGBA).
pub struct Quantizer4<const RB: u32, const GB: u32, const BB: u32, const AB: u32>;

impl<const RB: u32, const GB: u32, const BB: u32, const AB: u32> Quantizer4<RB, GB, BB, AB> {
    const RM: u32 = (1 << RB) - 1;
    const GM: u32 = (1 << GB) - 1;
    const BM: u32 = (1 << BB) - 1;
    const AM: u32 = (1 << AB) - 1;

    const RR: u32 = 1 << (8 - RB);
    const GR: u32 = 1 << (8 - GB);
    const BR: u32 = 1 << (8 - BB);
    const AR: u32 = 1 << (8 - AB);

    const GRID: [f32; 4] = [
        Self::RM as f32,
        Self::GM as f32,
        Self::BM as f32,
        Self::AM as f32,
    ];
    const GRID_INV: [u32; 4] = [1 << RB, 1 << GB, 1 << BB, 1 << AB];
    const GRID_GAP: [f32; 4] = [
        (0.5 * (Self::RR * Self::RM) as f32) / 255.0,
        (0.5 * (Self::GR * Self::GM) as f32) / 255.0,
        (0.5 * (Self::BR * Self::BM) as f32) / 255.0,
        (0.5 * (Self::AR * Self::AM) as f32) / 255.0,
    ];

    #[inline(always)]
    pub fn snap_to_lattice(val: &Vec4) -> Vec4 {
        let clamped = val.clamp(0.0, 1.0);
        let p_r = (Self::GRID[0] * clamped.x + Self::GRID_GAP[0]).floor() as usize;
        let p_g = (Self::GRID[1] * clamped.y + Self::GRID_GAP[1]).floor() as usize;
        let p_b = (Self::GRID[2] * clamped.z + Self::GRID_GAP[2]).floor() as usize;
        let p_a = (Self::GRID[3] * clamped.w + Self::GRID_GAP[3]).floor() as usize;

        Vec4::new(
            q_lut_value(RB, p_r.min(Self::RM as usize)),
            q_lut_value(GB, p_g.min(Self::GM as usize)),
            q_lut_value(BB, p_b.min(Self::BM as usize)),
            q_lut_value(AB, p_a.min(Self::AM as usize)),
        )
    }

    #[inline(always)]
    pub fn quantize_to_int(val: &Vec4) -> [u32; 4] {
        let qf = Self::snap_to_lattice(val);
        [
            (((qf.x * 255.0).floor() as u32) * Self::GRID_INV[0]) >> 8,
            (((qf.y * 255.0).floor() as u32) * Self::GRID_INV[1]) >> 8,
            (((qf.z * 255.0).floor() as u32) * Self::GRID_INV[2]) >> 8,
            (((qf.w * 255.0).floor() as u32) * Self::GRID_INV[3]) >> 8,
        ]
    }
}
