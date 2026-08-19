use crate::math::vector::{Vec3, Vec4};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct Sym2x2(pub [f32; 3]);

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct Sym3x3(pub [f32; 6]);

impl Sym3x3 {
    #[inline(always)]
    pub const fn new(s: f32) -> Self {
        Self([s; 6])
    }
    #[inline(always)]
    pub fn row0(&self) -> Vec3 {
        Vec3::new(self.0[0], self.0[1], self.0[2])
    }
    #[inline(always)]
    pub fn row1(&self) -> Vec3 {
        Vec3::new(self.0[1], self.0[3], self.0[4])
    }
    #[inline(always)]
    pub fn row2(&self) -> Vec3 {
        Vec3::new(self.0[2], self.0[4], self.0[5])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct Sym4x4(pub [f32; 10]);

impl Sym4x4 {
    #[inline(always)]
    pub const fn new(s: f32) -> Self {
        Self([s; 10])
    }
    #[inline(always)]
    pub fn row0(&self) -> Vec4 {
        Vec4::new(self.0[0], self.0[1], self.0[2], self.0[3])
    }
    #[inline(always)]
    pub fn row1(&self) -> Vec4 {
        Vec4::new(self.0[1], self.0[4], self.0[5], self.0[6])
    }
    #[inline(always)]
    pub fn row2(&self) -> Vec4 {
        Vec4::new(self.0[2], self.0[5], self.0[7], self.0[8])
    }
    #[inline(always)]
    pub fn row3(&self) -> Vec4 {
        Vec4::new(self.0[3], self.0[6], self.0[8], self.0[9])
    }
}

pub fn compute_weighted_covariance3(points: &[Vec3], weights: &[f32]) -> (Sym3x3, Vec3) {
    let n = points.len();
    if n == 0 {
        return (Sym3x3::new(0.0), Vec3::splat(0.0));
    }

    let mut total_weight = 0.0f32;
    let mut center = Vec3::splat(0.0);
    for i in 0..n {
        total_weight += weights[i];
        center += points[i] * weights[i];
    }
    if total_weight > f32::EPSILON {
        center /= total_weight;
    }

    let mut cov = Sym3x3::new(0.0);
    for i in 0..n {
        let a = points[i] - center;
        let b = a * weights[i];
        cov.0[0] += a.x * b.x;
        cov.0[1] += a.x * b.y;
        cov.0[2] += a.x * b.z;
        cov.0[3] += a.y * b.y;
        cov.0[4] += a.y * b.z;
        cov.0[5] += a.z * b.z;
    }
    (cov, center)
}

pub fn compute_weighted_covariance4(points: &[Vec4], weights: &[f32]) -> (Sym4x4, Vec4) {
    let n = points.len();
    if n == 0 {
        return (Sym4x4::new(0.0), Vec4::splat(0.0));
    }

    let mut total_weight = 0.0f32;
    let mut center = Vec4::splat(0.0);
    for i in 0..n {
        total_weight += weights[i];
        center += points[i] * weights[i];
    }
    if total_weight > f32::EPSILON {
        center /= total_weight;
    }

    let mut cov = Sym4x4::new(0.0);
    for i in 0..n {
        let a = points[i] - center;
        let b = a * weights[i];
        cov.0[0] += a.x * b.x;
        cov.0[1] += a.x * b.y;
        cov.0[2] += a.x * b.z;
        cov.0[3] += a.x * b.w;
        cov.0[4] += a.y * b.y;
        cov.0[5] += a.y * b.z;
        cov.0[6] += a.y * b.w;
        cov.0[7] += a.z * b.z;
        cov.0[8] += a.z * b.w;
        cov.0[9] += a.w * b.w;
    }
    (cov, center)
}

pub fn estimate_principle_component(cov: &Sym3x3) -> Vec3 {
    let r0 = cov.row0();
    let r1 = cov.row1();
    let r2 = cov.row2();

    let mut v =
        if r0.length_squared() > r1.length_squared() && r0.length_squared() > r2.length_squared() {
            r0
        } else if r1.length_squared() > r2.length_squared() {
            r1
        } else {
            r2
        };

    for _ in 0..8 {
        let x = v.dot(&r0);
        let y = v.dot(&r1);
        let z = v.dot(&r2);
        v = Vec3::new(x, y, z);
        let max_val = v.x.abs().max(v.y.abs()).max(v.z.abs());
        if max_val > f32::EPSILON {
            v /= max_val;
        }
    }
    v.normalize()
}

pub fn estimate_principle_component_vec4(cov: &Sym4x4) -> Vec4 {
    let r0 = cov.row0();
    let r1 = cov.row1();
    let r2 = cov.row2();
    let r3 = cov.row3();

    let mut v = if r0.length_squared() > r1.length_squared()
        && r0.length_squared() > r2.length_squared()
        && r0.length_squared() > r3.length_squared()
    {
        r0
    } else if r1.length_squared() > r2.length_squared() && r1.length_squared() > r3.length_squared()
    {
        r1
    } else if r2.length_squared() > r3.length_squared() {
        r2
    } else {
        r3
    };

    for _ in 0..8 {
        let x = v.dot(&r0);
        let y = v.dot(&r1);
        let z = v.dot(&r2);
        let w = v.dot(&r3);
        v = Vec4::new(x, y, z, w);
        let max_val = v.x.abs().max(v.y.abs()).max(v.z.abs()).max(v.w.abs());
        if max_val > f32::EPSILON {
            v /= max_val;
        }
    }
    v.normalize()
}

pub fn get_principle_projection_vec3(
    principle: &Vec3,
    centroid: &Vec3,
    points: &[Vec3],
) -> (Vec3, Vec3) {
    let len_sq = principle.dot(principle);
    if len_sq < f32::EPSILON {
        return (*centroid, *centroid);
    }
    let div = 1.0 / len_sq;

    let mut min = f32::MAX;
    let mut max = -f32::MAX;
    for p in points {
        let len = (*p - *centroid).dot(principle);
        if len < min {
            min = len;
        }
        if len > max {
            max = len;
        }
    }

    let start = *centroid + *principle * (min * div);
    let end = *centroid + *principle * (max * div);
    (start, end)
}

pub fn get_principle_projection_vec4(
    principle: &Vec4,
    centroid: &Vec4,
    points: &[Vec4],
) -> (Vec4, Vec4) {
    let len_sq = principle.dot(principle);
    if len_sq < f32::EPSILON {
        return (*centroid, *centroid);
    }
    let div = 1.0 / len_sq;

    let mut min = f32::MAX;
    let mut max = -f32::MAX;
    for p in points {
        let len = (*p - *centroid).dot(principle);
        if len < min {
            min = len;
        }
        if len > max {
            max = len;
        }
    }

    let start = *centroid + *principle * (min * div);
    let end = *centroid + *principle * (max * div);
    (start, end)
}

#[inline(always)]
pub fn solve_least_squares_vec3(
    alphax_sum: Vec3,
    betax_sum: Vec3,
    alpha2_sum: f32,
    beta2_sum: f32,
    alphabeta_sum: f32,
) -> Option<(Vec3, Vec3)> {
    let denom = alpha2_sum * beta2_sum - alphabeta_sum * alphabeta_sum;
    if denom.abs() > f32::EPSILON {
        let factor = 1.0 / denom;
        let a = (alphax_sum * beta2_sum - betax_sum * alphabeta_sum) * factor;
        let b = (betax_sum * alpha2_sum - alphax_sum * alphabeta_sum) * factor;
        Some((a, b))
    } else {
        None
    }
}

#[inline(always)]
pub fn solve_least_squares_vec4(
    alphax_sum: Vec4,
    betax_sum: Vec4,
    alpha2_sum: f32,
    beta2_sum: f32,
    alphabeta_sum: f32,
) -> Option<(Vec4, Vec4)> {
    let denom = alpha2_sum * beta2_sum - alphabeta_sum * alphabeta_sum;
    if denom.abs() > f32::EPSILON {
        let factor = 1.0 / denom;
        let a = (alphax_sum * beta2_sum - betax_sum * alphabeta_sum) * factor;
        let b = (betax_sum * alpha2_sum - alphax_sum * alphabeta_sum) * factor;
        Some((a, b))
    } else {
        None
    }
}
