use super::vec::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix34 {
    pub m: [[f32; 4]; 3],
}

impl Default for Matrix34 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Matrix34 {
    pub const IDENTITY: Self = Self {
        m: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ],
    };

    pub fn transform_point(&self, p: &Vec3) -> Vec3 {
        Vec3::new(
            self.m[0][0] * p.x + self.m[0][1] * p.y + self.m[0][2] * p.z + self.m[0][3],
            self.m[1][0] * p.x + self.m[1][1] * p.y + self.m[1][2] * p.z + self.m[1][3],
            self.m[2][0] * p.x + self.m[2][1] * p.y + self.m[2][2] * p.z + self.m[2][3],
        )
    }

    pub fn rotate_vector(&self, v: &Vec3) -> Vec3 {
        let mut res = Vec3::new(
            self.m[0][0] * v.x + self.m[0][1] * v.y + self.m[0][2] * v.z,
            self.m[1][0] * v.x + self.m[1][1] * v.y + self.m[1][2] * v.z,
            self.m[2][0] * v.x + self.m[2][1] * v.y + self.m[2][2] * v.z,
        );
        let len = (res.x * res.x + res.y * res.y + res.z * res.z).sqrt();
        if len > 1e-6 {
            res.x /= len;
            res.y /= len;
            res.z /= len;
        }
        res
    }

    pub fn get_translation(&self) -> Vec3 {
        Vec3::new(self.m[0][3], self.m[1][3], self.m[2][3])
    }

    pub fn set_translation(&mut self, t: Vec3) {
        self.m[0][3] = t.x;
        self.m[1][3] = t.y;
        self.m[2][3] = t.z;
    }

    pub fn is_identity(&self) -> bool {
        self == &Self::IDENTITY
    }
}
