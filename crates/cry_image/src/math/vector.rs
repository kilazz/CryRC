use std::ops::{
    Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub, SubAssign,
};

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;
#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C, align(16))]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub _pad: f32,
}

impl Vec3 {
    #[inline(always)]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z, _pad: 0.0 }
    }

    #[inline(always)]
    pub const fn splat(v: f32) -> Self {
        Self {
            x: v,
            y: v,
            z: v,
            _pad: 0.0,
        }
    }

    #[inline(always)]
    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    #[inline(always)]
    pub fn length_squared(&self) -> f32 {
        self.dot(self)
    }

    #[inline(always)]
    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline(always)]
    pub fn normalize(&self) -> Self {
        let len_sq = self.length_squared();
        if len_sq > f32::EPSILON {
            *self * (1.0 / len_sq.sqrt())
        } else {
            *self
        }
    }

    #[inline(always)]
    pub fn clamp(&self, min: f32, max: f32) -> Self {
        Self::new(
            self.x.clamp(min, max),
            self.y.clamp(min, max),
            self.z.clamp(min, max),
        )
    }

    #[inline(always)]
    pub fn floor(&self) -> Self {
        Self::new(self.x.floor(), self.y.floor(), self.z.floor())
    }

    #[inline(always)]
    pub fn ceil(&self) -> Self {
        Self::new(self.x.ceil(), self.y.ceil(), self.z.ceil())
    }

    #[inline(always)]
    pub fn round(&self) -> Self {
        Self::new(self.x.round(), self.y.round(), self.z.round())
    }

    #[inline(always)]
    pub fn extend(&self, w: f32) -> Vec4 {
        Vec4::new(self.x, self.y, self.z, w)
    }
}

impl Index<usize> for Vec3 {
    type Output = f32;
    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("Vec3 index out of bounds: {}", index),
        }
    }
}

impl IndexMut<usize> for Vec3 {
    #[inline(always)]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            _ => panic!("Vec3 index out of bounds: {}", index),
        }
    }
}

impl Add for Vec3 {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul for Vec3 {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: Self) -> Self {
        Self::new(self.x * rhs.x, self.y * rhs.y, self.z * rhs.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl Mul<Vec3> for f32 {
    type Output = Vec3;
    #[inline(always)]
    fn mul(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self * rhs.x, self * rhs.y, self * rhs.z)
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;
    #[inline(always)]
    fn div(self, rhs: f32) -> Self {
        let inv = 1.0 / rhs;
        Self::new(self.x * inv, self.y * inv, self.z * inv)
    }
}

impl Neg for Vec3 {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

impl AddAssign for Vec3 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl SubAssign for Vec3 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl MulAssign<f32> for Vec3 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

impl DivAssign<f32> for Vec3 {
    #[inline(always)]
    fn div_assign(&mut self, rhs: f32) {
        let inv = 1.0 / rhs;
        self.x *= inv;
        self.y *= inv;
        self.z *= inv;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C, align(16))]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    #[inline(always)]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    #[inline(always)]
    pub const fn splat(v: f32) -> Self {
        Self {
            x: v,
            y: v,
            z: v,
            w: v,
        }
    }

    #[inline(always)]
    pub fn to_vec3(&self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    #[inline(always)]
    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    #[inline(always)]
    pub fn length_squared(&self) -> f32 {
        self.dot(self)
    }

    #[inline(always)]
    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline(always)]
    pub fn normalize(&self) -> Self {
        let len_sq = self.length_squared();
        if len_sq > f32::EPSILON {
            *self * (1.0 / len_sq.sqrt())
        } else {
            *self
        }
    }

    #[inline(always)]
    pub fn clamp(&self, min: f32, max: f32) -> Self {
        Self::new(
            self.x.clamp(min, max),
            self.y.clamp(min, max),
            self.z.clamp(min, max),
            self.w.clamp(min, max),
        )
    }

    #[inline(always)]
    pub fn floor(&self) -> Self {
        Self::new(
            self.x.floor(),
            self.y.floor(),
            self.z.floor(),
            self.w.floor(),
        )
    }

    #[inline(always)]
    pub fn ceil(&self) -> Self {
        Self::new(self.x.ceil(), self.y.ceil(), self.z.ceil(), self.w.ceil())
    }

    #[inline(always)]
    pub fn round(&self) -> Self {
        Self::new(
            self.x.round(),
            self.y.round(),
            self.z.round(),
            self.w.round(),
        )
    }
}

impl Index<usize> for Vec4 {
    type Output = f32;
    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            3 => &self.w,
            _ => panic!("Vec4 index out of bounds: {}", index),
        }
    }
}

impl IndexMut<usize> for Vec4 {
    #[inline(always)]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            3 => &mut self.w,
            _ => panic!("Vec4 index out of bounds: {}", index),
        }
    }
}

impl Add for Vec4 {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self {
        Self::new(
            self.x + rhs.x,
            self.y + rhs.y,
            self.z + rhs.z,
            self.w + rhs.w,
        )
    }
}

impl Sub for Vec4 {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self {
        Self::new(
            self.x - rhs.x,
            self.y - rhs.y,
            self.z - rhs.z,
            self.w - rhs.w,
        )
    }
}

impl Mul for Vec4 {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: Self) -> Self {
        Self::new(
            self.x * rhs.x,
            self.y * rhs.y,
            self.z * rhs.z,
            self.w * rhs.w,
        )
    }
}

impl Mul<f32> for Vec4 {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs, self.w * rhs)
    }
}

impl Mul<Vec4> for f32 {
    type Output = Vec4;
    #[inline(always)]
    fn mul(self, rhs: Vec4) -> Vec4 {
        Vec4::new(self * rhs.x, self * rhs.y, self * rhs.z, self * rhs.w)
    }
}

impl Div<f32> for Vec4 {
    type Output = Self;
    #[inline(always)]
    fn div(self, rhs: f32) -> Self {
        let inv = 1.0 / rhs;
        Self::new(self.x * inv, self.y * inv, self.z * inv, self.w * inv)
    }
}

impl Div<Vec4> for Vec4 {
    type Output = Self;
    #[inline(always)]
    fn div(self, rhs: Self) -> Self {
        Self::new(
            self.x / rhs.x,
            self.y / rhs.y,
            self.z / rhs.z,
            self.w / rhs.w,
        )
    }
}

impl Neg for Vec4 {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, -self.w)
    }
}

impl AddAssign for Vec4 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl SubAssign for Vec4 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl MulAssign<f32> for Vec4 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

impl DivAssign<f32> for Vec4 {
    #[inline(always)]
    fn div_assign(&mut self, rhs: f32) {
        let inv = 1.0 / rhs;
        self.x *= inv;
        self.y *= inv;
        self.z *= inv;
        self.w *= inv;
    }
}

impl DivAssign<Vec4> for Vec4 {
    #[inline(always)]
    fn div_assign(&mut self, rhs: Vec4) {
        self.x /= rhs.x;
        self.y /= rhs.y;
        self.z /= rhs.z;
        self.w /= rhs.w;
    }
}

#[inline(always)]
pub fn dist_sq_vec4_simd(a: &Vec4, b: &Vec4) -> f32 {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    unsafe {
        let va = _mm_loadu_ps(a as *const Vec4 as *const f32);
        let vb = _mm_loadu_ps(b as *const Vec4 as *const f32);
        let diff = _mm_sub_ps(va, vb);
        let sq = _mm_mul_ps(diff, diff);
        let shuf = _mm_shuffle_ps(sq, sq, 0xEE);
        let sums = _mm_add_ps(sq, shuf);
        let shuf2 = _mm_shuffle_ps(sums, sums, 0x01);
        let res = _mm_add_ss(sums, shuf2);
        _mm_cvtss_f32(res)
    }
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let va = vld1q_f32(a as *const Vec4 as *const f32);
        let vb = vld1q_f32(b as *const Vec4 as *const f32);
        let diff = vsubq_f32(va, vb);
        let sq = vmulq_f32(diff, diff);
        vaddvq_f32(sq)
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
    {
        (*a - *b).length_squared()
    }
}

#[inline(always)]
pub fn dist_sq_vec3_simd(a: &Vec3, b: &Vec3) -> f32 {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    unsafe {
        let va = _mm_loadu_ps(a as *const Vec3 as *const f32);
        let vb = _mm_loadu_ps(b as *const Vec3 as *const f32);
        let diff = _mm_sub_ps(va, vb);
        let sq = _mm_mul_ps(diff, diff);
        let shuf_y = _mm_shuffle_ps(sq, sq, 0x01);
        let shuf_z = _mm_shuffle_ps(sq, sq, 0x02);
        let sum_xy = _mm_add_ss(sq, shuf_y);
        let sum_xyz = _mm_add_ss(sum_xy, shuf_z);
        _mm_cvtss_f32(sum_xyz)
    }
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let va = vld1q_f32(a as *const Vec3 as *const f32);
        let vb = vld1q_f32(b as *const Vec3 as *const f32);
        let diff = vsubq_f32(va, vb);
        let sq = vmulq_f32(diff, diff);
        vgetq_lane_f32(sq, 0) + vgetq_lane_f32(sq, 1) + vgetq_lane_f32(sq, 2)
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
    {
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        let dz = a.z - b.z;
        dx * dx + dy * dy + dz * dz
    }
}

#[inline(always)]
pub fn find_closest_code_4(pt: Vec4, codes: &[Vec4; 4]) -> (f32, u8) {
    let d0 = dist_sq_vec4_simd(&pt, &codes[0]);
    let d1 = dist_sq_vec4_simd(&pt, &codes[1]);
    let d2 = dist_sq_vec4_simd(&pt, &codes[2]);
    let d3 = dist_sq_vec4_simd(&pt, &codes[3]);

    let mut min_d = d0;
    let mut best_idx = 0u8;
    if d1 < min_d {
        min_d = d1;
        best_idx = 1;
    }
    if d2 < min_d {
        min_d = d2;
        best_idx = 2;
    }
    if d3 < min_d {
        min_d = d3;
        best_idx = 3;
    }
    (min_d, best_idx)
}

#[inline(always)]
pub fn find_closest_code_8(pt: Vec4, codes: &[Vec4; 8]) -> (f32, u8) {
    let (d0, i0) = find_closest_code_4(pt, (&codes[0..4]).try_into().unwrap());
    let (d1, i1) = find_closest_code_4(pt, (&codes[4..8]).try_into().unwrap());
    if d1 < d0 { (d1, i1 + 4) } else { (d0, i0) }
}

#[inline(always)]
pub fn find_closest_code_16(pt: Vec4, codes: &[Vec4; 16]) -> (f32, u8) {
    let (d0, i0) = find_closest_code_8(pt, (&codes[0..8]).try_into().unwrap());
    let (d1, i1) = find_closest_code_8(pt, (&codes[8..16]).try_into().unwrap());
    if d1 < d0 { (d1, i1 + 8) } else { (d0, i0) }
}

#[inline(always)]
pub fn find_closest_code_4_rgb(pt: Vec3, codes: &[Vec3; 4]) -> (f32, u8) {
    let d0 = dist_sq_vec3_simd(&pt, &codes[0]);
    let d1 = dist_sq_vec3_simd(&pt, &codes[1]);
    let d2 = dist_sq_vec3_simd(&pt, &codes[2]);
    let d3 = dist_sq_vec3_simd(&pt, &codes[3]);

    let mut min_d = d0;
    let mut best_idx = 0u8;
    if d1 < min_d {
        min_d = d1;
        best_idx = 1;
    }
    if d2 < min_d {
        min_d = d2;
        best_idx = 2;
    }
    if d3 < min_d {
        min_d = d3;
        best_idx = 3;
    }
    (min_d, best_idx)
}

#[inline(always)]
pub fn find_closest_code_8_rgb(pt: Vec3, codes: &[Vec3; 8]) -> (f32, u8) {
    let (d0, i0) = find_closest_code_4_rgb(pt, (&codes[0..4]).try_into().unwrap());
    let (d1, i1) = find_closest_code_4_rgb(pt, (&codes[4..8]).try_into().unwrap());
    if d1 < d0 { (d1, i1 + 4) } else { (d0, i0) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Col3 {
    pub r: i32,
    pub g: i32,
    pub b: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C, align(16))]
pub struct Col4 {
    pub r: i32,
    pub g: i32,
    pub b: i32,
    pub a: i32,
}
