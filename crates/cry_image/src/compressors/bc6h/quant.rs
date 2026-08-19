use super::tables::BC6HModeConfig;
use crate::math::vector::Vec3;

#[inline(always)]
pub fn half_to_float(h: u16) -> f32 {
    let sign = ((h >> 15) & 0x0001) as u32;
    let exp = ((h >> 10) & 0x001F) as u32;
    let mant = (h & 0x03FF) as u32;

    if exp == 0 {
        if mant == 0 {
            f32::from_bits(sign << 31)
        } else {
            let mut m = mant << 1;
            let mut e = 0;
            while (m & 0x0400) == 0 {
                m <<= 1;
                e += 1;
            }
            let exp_f = (127 - 15 - e) << 23;
            let mant_f = (m & 0x03FF) << 13;
            f32::from_bits((sign << 31) | exp_f | mant_f)
        }
    } else if exp == 31 {
        let exp_f = 0xFF << 23;
        let mant_f = mant << 13;
        f32::from_bits((sign << 31) | exp_f | mant_f)
    } else {
        let exp_f = (exp + 127 - 15) << 23;
        let mant_f = mant << 13;
        f32::from_bits((sign << 31) | exp_f | mant_f)
    }
}

#[inline(always)]
pub fn float_to_half(val: f32) -> u16 {
    let bits = val.to_bits();
    let sign = (bits >> 31) & 1;
    let exp = ((bits >> 23) & 0xFF) as i32;
    let mant = bits & 0x007F_FFFF;

    if exp >= 143 {
        ((sign << 15) | 0x7C00) as u16
    } else if exp <= 112 {
        0
    } else {
        let new_exp = (exp - 127 + 15) as u32;
        let new_mant = mant >> 13;
        ((sign << 15) | (new_exp << 10) | new_mant) as u16
    }
}

#[inline(always)]
pub fn unquantize_bc6h(val: i32, bits: u32, is_signed: bool) -> f32 {
    if !is_signed {
        let uval = (val as u32) & ((1 << bits) - 1);
        if bits == 0 {
            return 0.0;
        }
        if bits >= 15 {
            return half_to_float((uval << (16 - bits)) as u16);
        }

        let unq = if uval == 0 {
            0
        } else if uval == ((1 << bits) - 1) {
            0xFFFF
        } else {
            ((uval << 15) + 0x4000) >> (bits - 1)
        };
        let half_int = ((unq * 31) >> 6) as u16;
        half_to_float(half_int)
    } else {
        if bits == 0 {
            return 0.0;
        }
        if bits >= 15 {
            let uval = (val as u32) & ((1 << bits) - 1);
            return half_to_float((uval << (16 - bits)) as u16);
        }

        let sign = val < 0;
        let abs_val = val.unsigned_abs();
        let max_val = (1 << (bits - 1)) - 1;

        let unq = if abs_val == 0 {
            0
        } else if abs_val >= max_val {
            0x7FFF
        } else {
            ((abs_val << 15) + 0x4000) >> (bits - 1)
        };
        let half_mag = ((unq * 31) >> 6) as u16;
        let half_bits = if sign {
            (1 << 15) | (half_mag & 0x7FFF)
        } else {
            half_mag & 0x7FFF
        };
        half_to_float(half_bits)
    }
}

pub fn quantize_bc6h_channel(val: f32, bits: u32, is_signed: bool) -> i32 {
    if bits == 0 {
        return 0;
    }
    if !is_signed {
        if val <= 0.0 {
            return 0;
        }
        let h = float_to_half(val);
        if bits >= 15 {
            return (h >> (16 - bits)) as i32;
        }
        let mag = (h & 0x7FFF) as u32;
        let unq = ((mag << 6) + 15) / 31;
        let max_val = (1u32 << bits) - 1;
        let q = ((unq * max_val + 0x7FFF) / 0xFFFF).min(max_val);
        q as i32
    } else {
        let sign = val < 0.0;
        let abs_val = val.abs();
        let h = float_to_half(abs_val);
        if bits >= 15 {
            let h_signed = if sign {
                (1u16 << 15) | (h & 0x7FFF)
            } else {
                h & 0x7FFF
            };
            return (h_signed >> (16 - bits)) as i32;
        }
        let mag = (h & 0x7FFF) as u32;
        let unq = ((mag << 6) + 15) / 31;
        let max_val = (1u32 << (bits - 1)) - 1;
        let q = ((unq * max_val + 0x3FFF) / 0x7FFF).min(max_val);
        if sign { -(q as i32) } else { q as i32 }
    }
}

#[inline(always)]
pub fn sign_extend(val: u32, bits: u32) -> i32 {
    if bits == 0 || bits >= 32 {
        return val as i32;
    }
    let shift = 32 - bits;
    ((val << shift) as i32) >> shift
}

pub fn quantize_and_clamp_endpoints(
    start: &[Vec3; 2],
    end: &[Vec3; 2],
    cfg: &BC6HModeConfig,
    is_signed: bool,
) -> ([[[i32; 3]; 2]; 2], [[Vec3; 2]; 2]) {
    let mut q = [[[0i32; 3]; 2]; 2];
    let mut unq = [[Vec3::splat(0.0); 2]; 2];

    let num_subsets = cfg.num_subsets;
    let b_bits = cfg.endpoint_bits;
    let d_bits = cfg.delta_bits;

    for c in 0..3 {
        let max_val = if !is_signed {
            (1i32 << b_bits[c]) - 1
        } else {
            (1i32 << (b_bits[c] - 1)) - 1
        };
        let min_val = if !is_signed {
            0
        } else {
            -(1i32 << (b_bits[c] - 1))
        };
        q[0][0][c] =
            quantize_bc6h_channel(start[0][c], b_bits[c], is_signed).clamp(min_val, max_val);
    }

    for s in 0..num_subsets {
        for ep in 0..2 {
            if s == 0 && ep == 0 {
                continue;
            }
            let pt = if ep == 0 { start[s] } else { end[s] };
            for c in 0..3 {
                let mut val_q = quantize_bc6h_channel(pt[c], b_bits[c], is_signed);
                let db = d_bits[c];
                if db > 0 {
                    let base = q[0][0][c];
                    let min_d = -(1i32 << (db - 1));
                    let max_d = (1i32 << (db - 1)) - 1;
                    let diff = (val_q - base).clamp(min_d, max_d);
                    val_q = base + diff;
                }
                let max_val = if !is_signed {
                    (1i32 << b_bits[c]) - 1
                } else {
                    (1i32 << (b_bits[c] - 1)) - 1
                };
                let min_val = if !is_signed {
                    0
                } else {
                    -(1i32 << (b_bits[c] - 1))
                };
                q[s][ep][c] = val_q.clamp(min_val, max_val);
            }
        }
    }

    for s in 0..num_subsets {
        for ep in 0..2 {
            unq[s][ep] = Vec3::new(
                unquantize_bc6h(q[s][ep][0], b_bits[0], is_signed),
                unquantize_bc6h(q[s][ep][1], b_bits[1], is_signed),
                unquantize_bc6h(q[s][ep][2], b_bits[2], is_signed),
            );
        }
    }

    (q, unq)
}
