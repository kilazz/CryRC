// Copyright 2006-2026 Crytek GmbH / Crytek Group. All rights reserved.

use super::controller::KeyTimesData;
use super::quat_quantization::SmallTree64BitExtQuat;
use cry_core::math::{Quat, Vec3};

#[derive(Debug, Clone)]
pub struct CompressionSettings {
    pub position_epsilon: f32,
    pub rotation_epsilon_degrees: f32,
    pub scale_epsilon: f32,
}

impl Default for CompressionSettings {
    fn default() -> Self {
        Self {
            position_epsilon: 0.0005,
            rotation_epsilon_degrees: 0.1,
            scale_epsilon: 1e-5,
        }
    }
}

pub struct AnimationCompressor;

impl AnimationCompressor {
    pub fn compress_rotations(
        rotations: &[Quat],
        times: &[i32],
        tolerance_deg: f32,
    ) -> (Vec<SmallTree64BitExtQuat>, KeyTimesData) {
        if rotations.is_empty() {
            return (Vec::new(), KeyTimesData::F32(Vec::new()));
        }

        let num_keys = rotations.len();
        let cos_half_max = (tolerance_deg.to_radians() * 0.5).cos();

        let mut out_keys = Vec::new();
        let mut out_times = Vec::new();

        let mut first_idx = 0;
        out_keys.push(SmallTree64BitExtQuat::from_quat(rotations[0]));
        out_times.push(times[0] as f32);

        while first_idx + 2 < num_keys {
            let mut last_idx = first_idx + 2;
            while last_idx < num_keys {
                let q_start = rotations[first_idx];
                let q_end = rotations[last_idx];
                let count = (last_idx - first_idx) as f32;

                let mut exceeds = false;
                for step in 1..(last_idx - first_idx) {
                    let factor = step as f32 / count;
                    let mut interp = Quat::IDENTITY;
                    interp.nlerp(q_start, q_end, factor);

                    let actual = rotations[first_idx + step];
                    let cos_half = 1.0 - (interp.dot(&actual).abs() - 1.0).abs();
                    if cos_half < cos_half_max {
                        exceeds = true;
                        break;
                    }
                }

                if exceeds {
                    break;
                }
                last_idx += 1;
            }

            first_idx = last_idx - 1;
            out_keys.push(SmallTree64BitExtQuat::from_quat(rotations[first_idx]));
            out_times.push(times[first_idx] as f32);
        }

        if first_idx < num_keys - 1 {
            out_keys.push(SmallTree64BitExtQuat::from_quat(*rotations.last().unwrap()));
            out_times.push(*times.last().unwrap() as f32);
        }

        (out_keys, KeyTimesData::F32(out_times))
    }

    pub fn compress_positions(
        positions: &[Vec3],
        times: &[i32],
        tolerance: f32,
    ) -> (Vec<Vec3>, KeyTimesData) {
        if positions.is_empty() {
            return (Vec::new(), KeyTimesData::F32(Vec::new()));
        }

        let num_keys = positions.len();
        let max_dist_sq = tolerance * tolerance;

        let mut out_keys = Vec::new();
        let mut out_times = Vec::new();

        let mut first_idx = 0;
        out_keys.push(positions[0]);
        out_times.push(times[0] as f32);

        while first_idx + 2 < num_keys {
            let mut last_idx = first_idx + 2;
            while last_idx < num_keys {
                let p_start = positions[first_idx];
                let p_end = positions[last_idx];
                let count = (last_idx - first_idx) as f32;

                let mut exceeds = false;
                for step in 1..(last_idx - first_idx) {
                    let factor = step as f32 / count;
                    let interp = Vec3::new(
                        p_start.x * (1.0 - factor) + p_end.x * factor,
                        p_start.y * (1.0 - factor) + p_end.y * factor,
                        p_start.z * (1.0 - factor) + p_end.z * factor,
                    );

                    let actual = positions[first_idx + step];
                    let d_sq = (interp.x - actual.x).powi(2)
                        + (interp.y - actual.y).powi(2)
                        + (interp.z - actual.z).powi(2);
                    if d_sq > max_dist_sq {
                        exceeds = true;
                        break;
                    }
                }

                if exceeds {
                    break;
                }
                last_idx += 1;
            }

            first_idx = last_idx - 1;
            out_keys.push(positions[first_idx]);
            out_times.push(times[first_idx] as f32);
        }

        if first_idx < num_keys - 1 {
            out_keys.push(*positions.last().unwrap());
            out_times.push(*times.last().unwrap() as f32);
        }

        (out_keys, KeyTimesData::F32(out_times))
    }
}
