use crate::color_types::ColorRGBAf;
use cry_core::math::Vec3;
use rayon::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NormalFilterType {
    None,
    #[default]
    Scharr3x3,
    Sobel3x3,
    Farid5x5,
    Gauss,
}

#[derive(Debug, Clone)]
pub struct BumpProperties {
    pub filter_type: NormalFilterType,
    pub bump_strength: f32,
    pub blur_amount: f32,
    pub invert: bool,
}

impl Default for BumpProperties {
    fn default() -> Self {
        Self {
            filter_type: NormalFilterType::Scharr3x3,
            bump_strength: 5.0,
            blur_amount: 0.0,
            invert: false,
        }
    }
}

pub struct NormalProcessing;

impl NormalProcessing {
    /// Converts a height/displacement map into a high-precision 3D normal map using 2026+ differential operators
    pub fn bump_to_normal_map(
        height_pixels: &[ColorRGBAf],
        width: usize,
        height: usize,
        props: &BumpProperties,
        use_alpha_as_height: bool,
    ) -> Vec<ColorRGBAf> {
        let mut output = vec![ColorRGBAf::default(); width * height];
        let sign = if props.invert { -1.0 } else { 1.0 };
        let strength = props.bump_strength.abs() * 0.1 * sign;

        let get_h = |x: usize, y: usize| -> f32 {
            let p = height_pixels[y * width + x];
            if use_alpha_as_height {
                p.a
            } else {
                p.r * 0.299 + p.g * 0.587 + p.b * 0.114
            }
        };

        for y in 0..height {
            let y_m2 = (y + height - 2) % height;
            let y_m1 = (y + height - 1) % height;
            let y_p1 = (y + 1) % height;
            let y_p2 = (y + 2) % height;

            for x in 0..width {
                let x_m2 = (x + width - 2) % width;
                let x_m1 = (x + width - 1) % width;
                let x_p1 = (x + 1) % width;
                let x_p2 = (x + 2) % width;

                let (dx, dy) = match props.filter_type {
                    NormalFilterType::Scharr3x3 => {
                        // Optimal rotation-invariant 3x3 Scharr operator ([3, 10, 3])
                        let tl = get_h(x_m1, y_m1);
                        let t = get_h(x, y_m1);
                        let tr = get_h(x_p1, y_m1);
                        let l = get_h(x_m1, y);
                        let r = get_h(x_p1, y);
                        let bl = get_h(x_m1, y_p1);
                        let b = get_h(x, y_p1);
                        let br = get_h(x_p1, y_p1);

                        let g_x =
                            (tr * 3.0 + r * 10.0 + br * 3.0) - (tl * 3.0 + l * 10.0 + bl * 3.0);
                        let g_y =
                            (bl * 3.0 + b * 10.0 + br * 3.0) - (tl * 3.0 + t * 10.0 + tr * 3.0);
                        (g_x * (1.0 / 32.0), g_y * (1.0 / 32.0))
                    }
                    NormalFilterType::Farid5x5 => {
                        // 5-Tap Separable Farid Differentiator
                        let p = [0.030320, 0.249724, 0.439911, 0.249724, 0.030320];
                        let d = [-0.104550, -0.540870, 0.0, 0.540870, 0.104550];

                        let xs = [x_m2, x_m1, x, x_p1, x_p2];
                        let ys = [y_m2, y_m1, y, y_p1, y_p2];

                        let mut g_x = 0.0f32;
                        let mut g_y = 0.0f32;

                        for (iy, &py_idx) in ys.iter().enumerate() {
                            for (ix, &px_idx) in xs.iter().enumerate() {
                                let h_val = get_h(px_idx, py_idx);
                                g_x += h_val * d[ix] * p[iy];
                                g_y += h_val * p[ix] * d[iy];
                            }
                        }
                        (g_x, g_y)
                    }
                    NormalFilterType::Sobel3x3 => {
                        // Standard Sobel 3x3
                        let tl = get_h(x_m1, y_m1);
                        let t = get_h(x, y_m1);
                        let tr = get_h(x_p1, y_m1);
                        let l = get_h(x_m1, y);
                        let r = get_h(x_p1, y);
                        let bl = get_h(x_m1, y_p1);
                        let b = get_h(x, y_p1);
                        let br = get_h(x_p1, y_p1);

                        let g_x = (tr + r * 2.0 + br) - (tl + l * 2.0 + bl);
                        let g_y = (bl + b * 2.0 + br) - (tl + t * 2.0 + tr);
                        (g_x * 0.125, g_y * 0.125)
                    }
                    _ => {
                        let l = get_h(x_m1, y);
                        let r = get_h(x_p1, y);
                        let t = get_h(x, y_m1);
                        let b = get_h(x, y_p1);
                        ((r - l) * 0.5, (b - t) * 0.5)
                    }
                };

                let normal = Vec3::new(-dx * strength, -dy * strength, 1.0).normalized();

                output[y * width + x] = ColorRGBAf {
                    r: (normal.x * 0.5 + 0.5).clamp(0.0, 1.0),
                    g: (normal.y * 0.5 + 0.5).clamp(0.0, 1.0),
                    b: (normal.z * 0.5 + 0.5).clamp(0.0, 1.0),
                    a: height_pixels[y * width + x].a,
                };
            }
        }

        output
    }

    /// Modern Toksvig 2.0 / vMF (von Mises-Fisher) normal variance to roughness conversion
    pub fn gloss_from_normals(pixels: &mut [ColorRGBAf], has_authored_gloss: bool) {
        pixels.par_iter_mut().for_each(|pixel| {
            let nx = pixel.r * 2.0 - 1.0;
            let ny = pixel.g * 2.0 - 1.0;
            let nz = pixel.b * 2.0 - 1.0;
            let len = (nx * nx + ny * ny + nz * nz).sqrt().max(1.0 / 32768.0);

            let authored_smoothness: f32 = if has_authored_gloss { pixel.a } else { 1.0 };
            let mut final_smoothness: f32 = authored_smoothness;

            if len < 1.0 {
                // vMF variance representation
                let variance = ((1.0 - len * len) / (len * len).max(1e-4)).max(0.0);
                let authored_roughness = (1.0 - authored_smoothness).powi(2);
                let total_roughness = (authored_roughness.powi(2) + variance * (1.0 / 3.0)).sqrt();
                final_smoothness = (1.0 - total_roughness.sqrt()).clamp(0.0, 1.0);
            }

            pixel.a = final_smoothness;
        });
    }

    pub fn convert_legacy_gloss(pixels: &mut [ColorRGBAf]) {
        for p in pixels.iter_mut() {
            let s = p.a;
            p.a = 1.0f32 - (1.0f32 - s * 0.7f32).powf(3.0).clamp(0.0, 1.0);
        }
    }
}
