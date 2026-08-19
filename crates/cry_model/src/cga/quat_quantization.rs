use cry_core::math::Quat;
use std::f32::consts::FRAC_1_SQRT_2;

pub const RANGE_15BIT: f32 = FRAC_1_SQRT_2;
pub const MAX_15BIT_F: f32 = 23170.0;
pub const RANGE_20BIT: f32 = FRAC_1_SQRT_2;
pub const MAX_20BIT_F: f32 = 741454.0;
pub const RANGE_21BIT: f32 = FRAC_1_SQRT_2;
pub const MAX_21BIT_F: f32 = 1482909.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ECompressionFormat {
    NoCompress = 0,
    SmallTree48BitQuat = 5,
    SmallTree64BitExtQuat = 8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C)]
pub struct SmallTree64BitExtQuat {
    pub m_1: u32,
    pub m_2: u32,
}

impl SmallTree64BitExtQuat {
    pub fn from_quat(mut q: Quat) -> Self {
        q.normalize();
        let comps = [q.v[0], q.v[1], q.v[2], q.w];

        let mut max_idx = 0;
        let mut max_val = comps[0].abs();
        for (i, &c) in comps.iter().enumerate().skip(1) {
            if c.abs() > max_val {
                max_val = c.abs();
                max_idx = i;
            }
        }

        let sign = if comps[max_idx] < 0.0 { -1.0 } else { 1.0 };
        let mut other_indices = [0usize; 3];
        let mut cur = 0;
        for i in 0..4 {
            if i != max_idx {
                other_indices[cur] = i;
                cur += 1;
            }
        }

        let p0 = (((comps[other_indices[0]] * sign + RANGE_21BIT) * MAX_21BIT_F + 0.5) as u64)
            & 0x1F_FFFF;
        let p1 = (((comps[other_indices[1]] * sign + RANGE_21BIT) * MAX_21BIT_F + 0.5) as u64)
            & 0x1F_FFFF;
        let p2 = (((comps[other_indices[2]] * sign + RANGE_20BIT) * MAX_20BIT_F + 0.5) as u64)
            & 0xF_FFFF;

        let mut val = p0 | (p1 << 21) | (p2 << 42);
        val |= (max_idx as u64) << 62;

        Self {
            m_1: (val & 0xFFFF_FFFF) as u32,
            m_2: ((val >> 32) & 0xFFFF_FFFF) as u32,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C)]
pub struct SmallTree48BitQuat {
    pub m_1: u16,
    pub m_2: u16,
    pub m_3: u16,
}

impl SmallTree48BitQuat {
    pub fn from_quat(mut q: Quat) -> Self {
        q.normalize();
        let comps = [q.v[0], q.v[1], q.v[2], q.w];

        let mut max_idx = 0;
        let mut max_val = comps[0].abs();
        for (i, &c) in comps.iter().enumerate().skip(1) {
            if c.abs() > max_val {
                max_val = c.abs();
                max_idx = i;
            }
        }

        let sign = if comps[max_idx] < 0.0 { -1.0 } else { 1.0 };
        let mut other = [0u64; 3];
        let mut cur = 0;
        for (i, &c) in comps.iter().enumerate() {
            if i != max_idx {
                other[cur] = (((c * sign + RANGE_15BIT) * MAX_15BIT_F + 0.5) as u64) & 0x7FFF;
                cur += 1;
            }
        }

        let mut val = other[0] | (other[1] << 15) | (other[2] << 30);
        val |= (max_idx as u64) << 46;

        Self {
            m_1: (val & 0xFFFF) as u16,
            m_2: ((val >> 16) & 0xFFFF) as u16,
            m_3: ((val >> 32) & 0xFFFF) as u16,
        }
    }
}
