use super::quat::Quat;
use super::vec::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix33 {
    pub m: [[f32; 3]; 3],
}

impl Default for Matrix33 {
    fn default() -> Self {
        Self::identity()
    }
}

impl Matrix33 {
    pub fn identity() -> Self {
        Self {
            m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    pub fn set_row(&mut self, row: usize, v: [f32; 3]) {
        self.m[row] = v;
    }

    pub fn orthonormalize_fast(&mut self) {
        let mut r0 = self.m[0];
        let mut r1 = self.m[1];
        let mut r2 = self.m[2];

        normalize_slice(&mut r0);

        let dot01 = r0[0] * r1[0] + r0[1] * r1[1] + r0[2] * r1[2];
        r1[0] -= dot01 * r0[0];
        r1[1] -= dot01 * r0[1];
        r1[2] -= dot01 * r0[2];
        normalize_slice(&mut r1);

        r2[0] = r0[1] * r1[2] - r0[2] * r1[1];
        r2[1] = r0[2] * r1[0] - r0[0] * r1[2];
        r2[2] = r0[0] * r1[1] - r0[1] * r1[0];

        self.m[0] = r0;
        self.m[1] = r1;
        self.m[2] = r2;
    }

    pub fn set_rotation_v0_v1(&mut self, v0: Vec3, v1: Vec3) {
        let dot = (v0.x * v1.x + v0.y * v1.y + v0.z * v1.z).clamp(-1.0, 1.0);
        if dot > 0.9999 {
            *self = Self::identity();
            return;
        }
        if dot < -0.9999 {
            self.m = [[-1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]];
            return;
        }

        let mut c = Vec3::new(
            v0.y * v1.z - v0.z * v1.y,
            v0.z * v1.x - v0.x * v1.z,
            v0.x * v1.y - v0.y * v1.x,
        );
        let len = (c.x * c.x + c.y * c.y + c.z * c.z).sqrt().max(1e-6);
        c.x /= len;
        c.y /= len;
        c.z /= len;

        let s = (1.0 - dot * dot).max(0.0).sqrt();
        let h = (1.0 - dot) / (1.0 - dot * dot).max(1e-6);

        self.m[0][0] = dot + h * c.x * c.x;
        self.m[0][1] = h * c.x * c.y - s * c.z;
        self.m[0][2] = h * c.x * c.z + s * c.y;

        self.m[1][0] = h * c.x * c.y + s * c.z;
        self.m[1][1] = dot + h * c.y * c.y;
        self.m[1][2] = h * c.y * c.z - s * c.x;

        self.m[2][0] = h * c.x * c.z - s * c.y;
        self.m[2][1] = h * c.y * c.z + s * c.x;
        self.m[2][2] = dot + h * c.z * c.z;
    }

    pub fn transform_vector(&self, v: Vec3) -> Vec3 {
        Vec3::new(
            self.m[0][0] * v.x + self.m[0][1] * v.y + self.m[0][2] * v.z,
            self.m[1][0] * v.x + self.m[1][1] * v.y + self.m[1][2] * v.z,
            self.m[2][0] * v.x + self.m[2][1] * v.y + self.m[2][2] * v.z,
        )
    }

    pub fn to_quat(&self) -> Quat {
        let trace = self.m[0][0] + self.m[1][1] + self.m[2][2];
        if trace > 0.0 {
            let s = 0.5 / (trace + 1.0).sqrt();
            Quat::new(
                (self.m[2][1] - self.m[1][2]) * s,
                (self.m[0][2] - self.m[2][0]) * s,
                (self.m[1][0] - self.m[0][1]) * s,
                0.25 / s,
            )
        } else if self.m[0][0] > self.m[1][1] && self.m[0][0] > self.m[2][2] {
            let s = 2.0 * (1.0 + self.m[0][0] - self.m[1][1] - self.m[2][2]).sqrt();
            Quat::new(
                0.25 * s,
                (self.m[0][1] + self.m[1][0]) / s,
                (self.m[0][2] + self.m[2][0]) / s,
                (self.m[2][1] - self.m[1][2]) / s,
            )
        } else if self.m[1][1] > self.m[2][2] {
            let s = 2.0 * (1.0 + self.m[1][1] - self.m[0][0] - self.m[2][2]).sqrt();
            Quat::new(
                (self.m[0][1] + self.m[1][0]) / s,
                0.25 * s,
                (self.m[1][2] + self.m[2][1]) / s,
                (self.m[0][2] - self.m[2][0]) / s,
            )
        } else {
            let s = 2.0 * (1.0 + self.m[2][2] - self.m[0][0] - self.m[1][1]).sqrt();
            Quat::new(
                (self.m[0][2] + self.m[2][0]) / s,
                (self.m[1][2] + self.m[2][1]) / s,
                0.25 * s,
                (self.m[1][0] - self.m[0][1]) / s,
            )
        }
    }
}

fn normalize_slice(v: &mut [f32; 3]) {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 1e-6 {
        let inv = 1.0 / len;
        v[0] *= inv;
        v[1] *= inv;
        v[2] *= inv;
    }
}
