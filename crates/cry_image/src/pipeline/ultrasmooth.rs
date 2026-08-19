use crate::pipeline::metrics::TrackedStat;

const ULTRASMOOTH_BLOCK_STD_DEV_THRESHOLD: f32 = 2.9;
const DARK_THRESHOLD: f32 = 13.0;
const BRIGHT_THRESHOLD: f32 = 222.0;
pub const ULTRASMOOTH_BLOCK_MSE_SCALE: f32 = 120.0;

pub fn compute_block_mse_scales(
    pixels: &[u8],
    width: usize,
    height: usize,
    blocks_x: usize,
    blocks_y: usize,
) -> Vec<f32> {
    let total_blocks = blocks_x * blocks_y;
    let mut scales = vec![-1.0f32; total_blocks];

    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let mut y_stats = TrackedStat::new();
            let mut rgb_stats = [TrackedStat::new(); 3];

            for dy in 0..4 {
                let py = (by * 4 + dy).min(height - 1);
                for dx in 0..4 {
                    let px = (bx * 4 + dx).min(width - 1);
                    let off = (py * width + px) * 4;
                    let r = pixels[off];
                    let g = pixels[off + 1];
                    let b = pixels[off + 2];

                    let luma = ((13938 * r as u32 + 46869 * g as u32 + 4729 * b as u32 + 32768)
                        >> 16) as f64;
                    y_stats.update(luma);
                    rgb_stats[0].update(r as f64);
                    rgb_stats[1].update(g as f64);
                    rgb_stats[2].update(b as f64);
                }
            }

            let max_std_dev = rgb_stats[0]
                .std_dev()
                .max(rgb_stats[1].std_dev())
                .max(rgb_stats[2].std_dev()) as f32;
            let y_avg = y_stats.mean() as f32;

            if max_std_dev < ULTRASMOOTH_BLOCK_STD_DEV_THRESHOLD
                && (DARK_THRESHOLD..BRIGHT_THRESHOLD).contains(&y_avg)
            {
                scales[by * blocks_x + bx] = ULTRASMOOTH_BLOCK_MSE_SCALE;
            }
        }
    }

    scales
}
