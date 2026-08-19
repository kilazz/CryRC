use crate::math::vector::Vec3;

pub const DEVIANCE_MAX: f32 = -1.0;
pub const DEVIANCE_BASE: f32 = 0.0;

/// Reconstructs the Z-component of a unit normal vector on the hemisphere:
/// `z = sqrt(max(0, 1 - x^2 - y^2))`
#[inline(always)]
pub fn complement_z(xy: Vec3) -> Vec3 {
    let x = xy.x;
    let y = xy.y;
    let len_sq = x * x + y * y;

    let z = if len_sq < 1.0 {
        (1.0 - len_sq).sqrt()
    } else {
        0.0
    };

    if len_sq > 1.0 {
        let inv_len = 1.0 / len_sq.sqrt();
        Vec3::new(x * inv_len, y * inv_len, 0.0)
    } else {
        Vec3::new(x, y, z)
    }
}

/// Converts a color-encoded normal [0.0, 1.0] to signed direction space [-1.0, 1.0].
#[inline(always)]
pub fn unorm_to_snorm(v: Vec3) -> Vec3 {
    Vec3::new(v.x * 2.0 - 1.0, v.y * 2.0 - 1.0, v.z * 2.0 - 1.0)
}

/// Converts a signed direction vector [-1.0, 1.0] back to normalized color space [0.0, 1.0].
#[inline(always)]
pub fn snorm_to_unorm(v: Vec3) -> Vec3 {
    Vec3::new(
        (v.x * 0.5 + 0.5).clamp(0.0, 1.0),
        (v.y * 0.5 + 0.5).clamp(0.0, 1.0),
        (v.z * 0.5 + 0.5).clamp(0.0, 1.0),
    )
}

/// Accumulates spherical angular error from cosine angle deviation:
/// `error += (cos(theta) - 1)^2 * weight`
#[inline(always)]
pub fn add_deviance(cosine_dist: f32, total_error: &mut f32, weight: f32) {
    let ang = cosine_dist + DEVIANCE_MAX;
    let sqr = ang * ang;
    *total_error += sqr * weight;
}

#[inline(always)]
pub fn min_deviance_3(normal: &Vec3, codes: &[Vec3; 3]) -> (f32, usize) {
    let d0 = normal.dot(&codes[0]);
    let d1 = normal.dot(&codes[1]);
    let d2 = normal.dot(&codes[2]);

    let mut max_dot = d0;
    let mut best_idx = 0;

    if d1 > max_dot {
        max_dot = d1;
        best_idx = 1;
    }
    if d2 > max_dot {
        max_dot = d2;
        best_idx = 2;
    }

    (max_dot, best_idx)
}

#[inline(always)]
pub fn min_deviance_4(normal: &Vec3, codes: &[Vec3; 4]) -> (f32, usize) {
    let d0 = normal.dot(&codes[0]);
    let d1 = normal.dot(&codes[1]);
    let d2 = normal.dot(&codes[2]);
    let d3 = normal.dot(&codes[3]);

    let mut max_dot = d0;
    let mut best_idx = 0;

    if d1 > max_dot {
        max_dot = d1;
        best_idx = 1;
    }
    if d2 > max_dot {
        max_dot = d2;
        best_idx = 2;
    }
    if d3 > max_dot {
        max_dot = d3;
        best_idx = 3;
    }

    (max_dot, best_idx)
}

#[inline(always)]
pub fn codebook_3_normal(start: &Vec3, end: &Vec3) -> [Vec3; 3] {
    let s = unorm_to_snorm(*start);
    let e = unorm_to_snorm(*end);

    [s.normalize(), e.normalize(), ((s + e) * 0.5).normalize()]
}

#[inline(always)]
pub fn codebook_4_normal(start: &Vec3, end: &Vec3) -> [Vec3; 4] {
    let s = unorm_to_snorm(*start);
    let e = unorm_to_snorm(*end);

    [
        s.normalize(),
        e.normalize(),
        (s * (2.0 / 3.0) + e * (1.0 / 3.0)).normalize(),
        (s * (1.0 / 3.0) + e * (2.0 / 3.0)).normalize(),
    ]
}
