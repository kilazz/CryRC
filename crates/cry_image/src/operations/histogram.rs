use crate::color_types::ColorRGBAf;
use crate::converters::colorspaces::rgbl::Rgbl;

pub const HISTOGRAM_BINS: usize = 256;

#[derive(Debug, Clone)]
pub struct Histogram {
    pub bins: [u64; HISTOGRAM_BINS],
}

impl Default for Histogram {
    fn default() -> Self {
        Self {
            bins: [0; HISTOGRAM_BINS],
        }
    }
}

impl Histogram {
    pub fn compute_luminance_histogram(pixels: &[ColorRGBAf]) -> Self {
        let mut hist = Self::default();
        for p in pixels {
            let l = Rgbl::get_luminance_f32(p.r, p.g, p.b).clamp(0.0, 1.0);
            let bin = ((l * (HISTOGRAM_BINS - 1) as f32).floor() as usize).min(HISTOGRAM_BINS - 1);
            hist.bins[bin] += 1;
        }
        hist
    }
}
