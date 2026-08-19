use crate::color_types::ColorRGBAf;

pub struct MeasureRanges;

impl MeasureRanges {
    pub fn calculate_average_brightness(pixels: &[ColorRGBAf], _w: usize, _h: usize) -> f32 {
        if pixels.is_empty() {
            return 0.5;
        }
        let sum: f32 = pixels.iter().map(|p| (p.r + p.g + p.b) / 3.0).sum();
        sum / pixels.len() as f32
    }
}
