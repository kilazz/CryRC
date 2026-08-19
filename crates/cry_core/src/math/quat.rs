use super::vec::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    pub v: [f32; 3],
    pub w: f32,
}

impl Default for Quat {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Quat {
    pub const IDENTITY: Self = Self {
        v: [0.0, 0.0, 0.0],
        w: 1.0,
    };

    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { v: [x, y, z], w }
    }

    pub fn normalize(&mut self) {
        let len_sq =
            self.v[0] * self.v[0] + self.v[1] * self.v[1] + self.v[2] * self.v[2] + self.w * self.w;
        if len_sq > 0.0 {
            let inv_len = 1.0 / len_sq.sqrt();
            self.v[0] *= inv_len;
            self.v[1] *= inv_len;
            self.v[2] *= inv_len;
            self.w *= inv_len;
        }
    }

    #[inline]
    pub fn dot(&self, other: &Self) -> f32 {
        self.v[0] * other.v[0] + self.v[1] * other.v[1] + self.v[2] * other.v[2] + self.w * other.w
    }

    pub fn log(&self) -> [f32; 3] {
        let v_len = (self.v[0] * self.v[0] + self.v[1] * self.v[1] + self.v[2] * self.v[2]).sqrt();
        if v_len > 1e-6 {
            let angle = v_len.atan2(self.w);
            let factor = angle / v_len;
            [self.v[0] * factor, self.v[1] * factor, self.v[2] * factor]
        } else {
            [0.0, 0.0, 0.0]
        }
    }

    pub fn exp(v: [f32; 3]) -> Self {
        let angle = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if angle > 1e-6 {
            let factor = angle.sin() / angle;
            Self::new(v[0] * factor, v[1] * factor, v[2] * factor, angle.cos())
        } else {
            Self::IDENTITY
        }
    }

    pub fn nlerp(&mut self, q0: Self, q1: Self, t: f32) {
        let mut target = q1;
        if q0.dot(&q1) < 0.0 {
            target = Quat::new(-q1.v[0], -q1.v[1], -q1.v[2], -q1.w);
        }
        self.v[0] = q0.v[0] * (1.0 - t) + target.v[0] * t;
        self.v[1] = q0.v[1] * (1.0 - t) + target.v[1] * t;
        self.v[2] = q0.v[2] * (1.0 - t) + target.v[2] * t;
        self.w = q0.w * (1.0 - t) + target.w * t;
        self.normalize();
    }

    pub fn inverted(&self) -> Self {
        Self::new(-self.v[0], -self.v[1], -self.v[2], self.w)
    }

    pub fn rotate_vector(&self, p: Vec3) -> Vec3 {
        let qv = Vec3::new(self.v[0], self.v[1], self.v[2]);
        let uv = qv.cross(p);
        let uuv = qv.cross(uv);
        Vec3::new(
            p.x + ((uv.x * self.w) + uuv.x) * 2.0,
            p.y + ((uv.y * self.w) + uuv.y) * 2.0,
            p.z + ((uv.z * self.w) + uuv.z) * 2.0,
        )
    }
}
