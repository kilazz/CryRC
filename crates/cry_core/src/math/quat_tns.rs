#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct QuatTNS {
    pub q: [f32; 4],
    pub t: [f32; 3],
    pub s: [f32; 3],
}

impl QuatTNS {
    pub fn identity() -> Self {
        Self {
            q: [0.0, 0.0, 0.0, 1.0],
            t: [0.0, 0.0, 0.0],
            s: [1.0, 1.0, 1.0],
        }
    }
}
