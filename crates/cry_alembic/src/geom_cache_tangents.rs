use super::geom_cache_file::{QTangent, TANGENT_QUAT_PRECISION};
use cry_core::math::Matrix33;

pub fn encode_qtangent(mut frame: Matrix33, reflection: bool) -> QTangent {
    frame.orthonormalize_fast();
    let mut q = frame.to_quat();

    q.v[0] = -q.v[0];
    q.v[1] = -q.v[1];
    q.v[2] = -q.v[2];

    if q.w < 0.0 {
        q.v[0] = -q.v[0];
        q.v[1] = -q.v[1];
        q.v[2] = -q.v[2];
        q.w = -q.w;
    }

    let multiplier = ((1 << (TANGENT_QUAT_PRECISION - 1)) - 1) as f32; // 32767.0
    let bias = 1.0 / multiplier;
    let bias_scale = (1.0 - bias * bias).max(0.0).sqrt();

    if q.w.abs() < bias {
        q.v[0] *= bias_scale;
        q.v[1] *= bias_scale;
        q.v[2] *= bias_scale;
        q.w = bias;
    }

    if reflection {
        q.v[0] = -q.v[0];
        q.v[1] = -q.v[1];
        q.v[2] = -q.v[2];
        q.w = -q.w;
    }

    [
        (q.v[0] * multiplier).clamp(-32768.0, 32767.0) as i16,
        (q.v[1] * multiplier).clamp(-32768.0, 32767.0) as i16,
        (q.v[2] * multiplier).clamp(-32768.0, 32767.0) as i16,
        (q.w * multiplier).clamp(-32768.0, 32767.0) as i16,
    ]
}
