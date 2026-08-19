#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct TrackedStat {
    pub count: u32,
    pub sum: f64,
    pub sum_sq: f64,
}

impl TrackedStat {
    #[inline(always)]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline(always)]
    pub fn update(&mut self, val: f64) {
        self.count += 1;
        self.sum += val;
        self.sum_sq += val * val;
    }

    #[inline(always)]
    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }

    #[inline(always)]
    pub fn variance(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            let n = self.count as f64;
            ((n * self.sum_sq - self.sum * self.sum) / (n * n)).max(0.0)
        }
    }

    #[inline(always)]
    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct ImageMetrics {
    pub max_err: f64,
    pub mean: f64,
    pub mean_squared: f64,
    pub root_mean_squared: f64,
    pub peak_snr: f64,
}

impl ImageMetrics {
    pub fn compute(
        a: &[u8],
        b: &[u8],
        width: usize,
        height: usize,
        first_channel: usize,
        num_channels: usize,
    ) -> Self {
        assert_eq!(a.len(), width * height * 4);
        assert_eq!(b.len(), width * height * 4);

        if width == 0 || height == 0 {
            return Self {
                max_err: 0.0,
                mean: 0.0,
                mean_squared: 0.0,
                root_mean_squared: 0.0,
                peak_snr: 100.0,
            };
        }

        let mut sum2: f64 = 0.0;
        let mut sum: f64 = 0.0;
        let mut max_err: f64 = 0.0;
        let total_pixels = width * height;

        for i in 0..total_pixels {
            let off = i * 4;
            for c in 0..num_channels {
                let ch = first_channel + c;
                let diff = (a[off + ch] as i32 - b[off + ch] as i32).abs() as f64;
                max_err = max_err.max(diff);
                sum += diff;
                sum2 += diff * diff;
            }
        }

        let total_values = (total_pixels * num_channels.clamp(1, 4)) as f64;
        let mean = sum / total_values;
        let mean_squared = sum2 / total_values;
        let root_mean_squared = mean_squared.sqrt();

        let peak_snr = if root_mean_squared == 0.0 {
            100.0
        } else {
            (20.0 * (255.0 / root_mean_squared).log10()).clamp(0.0, 100.0)
        };

        Self {
            max_err,
            mean,
            mean_squared,
            root_mean_squared,
            peak_snr,
        }
    }
}

pub fn compute_ssim(a: &[u8], b: &[u8], width: usize, height: usize) -> [f32; 4] {
    if width == 0 || height == 0 {
        return [1.0, 1.0, 1.0, 1.0];
    }
    let mut sum_ssim = [0.0f64; 4];
    let total = (width * height) as f64;

    for i in 0..width * height {
        let off = i * 4;
        for c in 0..4 {
            let v1 = a[off + c] as f64;
            let v2 = b[off + c] as f64;
            let diff = (v1 - v2).abs();
            sum_ssim[c] += 1.0 - (diff / 255.0);
        }
    }

    [
        (sum_ssim[0] / total) as f32,
        (sum_ssim[1] / total) as f32,
        (sum_ssim[2] / total) as f32,
        (sum_ssim[3] / total) as f32,
    ]
}
