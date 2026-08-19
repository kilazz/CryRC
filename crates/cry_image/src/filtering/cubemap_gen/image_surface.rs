#[derive(Debug, Clone, Default)]
pub struct ImageSurface {
    pub width: usize,
    pub height: usize,
    pub channels: usize,
    pub data: Vec<f32>,
}

impl ImageSurface {
    pub fn new(width: usize, height: usize, channels: usize) -> Self {
        Self {
            width,
            height,
            channels,
            data: vec![0.0f32; width * height * channels],
        }
    }

    #[inline(always)]
    pub fn get_pixel(&self, u: usize, v: usize) -> &[f32] {
        let off = (v * self.width + u) * self.channels;
        &self.data[off..off + self.channels]
    }
}
