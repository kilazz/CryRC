use super::vec::Vec3;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub fn reset(&mut self) {
        self.min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
        self.max = Vec3::new(-f32::MAX, -f32::MAX, -f32::MAX);
    }

    pub fn add_point(&mut self, p: Vec3) {
        self.min.x = self.min.x.min(p.x);
        self.min.y = self.min.y.min(p.y);
        self.min.z = self.min.z.min(p.z);

        self.max.x = self.max.x.max(p.x);
        self.max.y = self.max.y.max(p.y);
        self.max.z = self.max.z.max(p.z);
    }

    pub fn add_aabb(&mut self, other: &AABB) {
        self.add_point(other.min);
        self.add_point(other.max);
    }

    pub fn get_size(&self) -> Vec3 {
        Vec3::new(
            (self.max.x - self.min.x).max(0.0001),
            (self.max.y - self.min.y).max(0.0001),
            (self.max.z - self.min.z).max(0.0001),
        )
    }
}
