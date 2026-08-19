use crate::math::vector::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct NormalMapOptions {
    pub amplitude: f32,
    pub compute_occlusion: bool,
    pub wrap_u: bool,
    pub wrap_v: bool,
    pub invert_sign: bool,
}

impl Default for NormalMapOptions {
    fn default() -> Self {
        Self {
            amplitude: 1.0,
            compute_occlusion: true,
            wrap_u: true,
            wrap_v: true,
            invert_sign: false,
        }
    }
}

pub fn compute_normal_map(
    heightmap: &[u8],
    width: usize,
    height: usize,
    opts: NormalMapOptions,
) -> Vec<u8> {
    let mut out = vec![0u8; width * height * 4];

    let get_height = |x: isize, y: isize| -> f32 {
        let cx = if opts.wrap_u {
            x.rem_euclid(width as isize) as usize
        } else {
            x.clamp(0, width as isize - 1) as usize
        };
        let cy = if opts.wrap_v {
            y.rem_euclid(height as isize) as usize
        } else {
            y.clamp(0, height as isize - 1) as usize
        };
        heightmap[cy * width + cx] as f32 / 255.0
    };

    for y in 0..height as isize {
        for x in 0..width as isize {
            let v00 = get_height(x - 1, y - 1);
            let v01 = get_height(x, y - 1);
            let v02 = get_height(x + 1, y - 1);
            let v10 = get_height(x - 1, y);
            let v12 = get_height(x + 1, y);
            let v20 = get_height(x - 1, y + 1);
            let v21 = get_height(x, y + 1);
            let v22 = get_height(x + 1, y + 1);

            let tot_delta_x = (v00 - v02) + (v10 - v12) * 2.0 + (v20 - v22);
            let delta_zx = tot_delta_x * opts.amplitude / 6.0;

            let tot_delta_y = (v00 - v20) + (v01 - v21) * 2.0 + (v02 - v22);
            let delta_zy = tot_delta_y * opts.amplitude / 6.0;

            let normal = Vec3::new(delta_zx, delta_zy, 1.0).normalize();

            let mut alpha = 1.0f32;
            if opts.compute_occlusion {
                let c = get_height(x, y);
                let mut delta_acc = 0.0f32;
                for &(nx, ny) in &[
                    (x - 1, y - 1),
                    (x, y - 1),
                    (x + 1, y - 1),
                    (x - 1, y),
                    (x + 1, y),
                    (x - 1, y + 1),
                    (x, y + 1),
                    (x + 1, y + 1),
                ] {
                    let h_diff = get_height(nx, ny) - c;
                    if h_diff > 0.0 {
                        delta_acc += h_diff;
                    }
                }
                delta_acc *= 0.125 * opts.amplitude;
                let r = (1.0 + delta_acc * delta_acc).sqrt();
                alpha = ((r - delta_acc) / r).clamp(0.0, 1.0);
            }

            let nx = if opts.invert_sign {
                -normal.x
            } else {
                normal.x
            };
            let ny = if opts.invert_sign {
                -normal.y
            } else {
                normal.y
            };

            let out_idx = ((y as usize) * width + (x as usize)) * 4;
            out[out_idx] = ((nx * 0.5 + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8;
            out[out_idx + 1] = ((ny * 0.5 + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8;
            out[out_idx + 2] = ((normal.z * 0.5 + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8;
            out[out_idx + 3] = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }

    out
}
