pub use crate::flags::MipmapFilter;
use crate::math::vector::Vec4;
use crate::tables::srgb::{linear_to_srgb, srgb_to_linear};
use std::f32::consts::PI;

pub struct MipLevel {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
pub struct AlphaCoverageOptions {
    pub alpha_cutoff: f32,
}

// --- Mathematical Filter Kernels ---

#[inline(always)]
pub fn filter_sinc(x: f32) -> f32 {
    if x.abs() < 1e-5 {
        1.0
    } else {
        let px = PI * x;
        px.sin() / px
    }
}

#[inline(always)]
pub fn bessel_i0(x: f32) -> f32 {
    let mut sum = 1.0;
    let mut term = 1.0;
    let x2 = x * x * 0.25;
    for k in 1..=12 {
        term *= x2 / (k as f32 * k as f32);
        sum += term;
        if term < 1e-7 {
            break;
        }
    }
    sum
}

#[inline(always)]
pub fn filter_mitchell_netravali(x: f32) -> f32 {
    let b = 1.0 / 3.0;
    let c = 1.0 / 3.0;
    let ax = x.abs();
    if ax < 1.0 {
        ((12.0 - 9.0 * b - 6.0 * c) * ax * ax * ax
            + (-18.0 + 12.0 * b + 6.0 * c) * ax * ax
            + (6.0 - 2.0 * b))
            / 6.0
    } else if ax < 2.0 {
        ((-b - 6.0 * c) * ax * ax * ax
            + (6.0 * b + 30.0 * c) * ax * ax
            + (-12.0 * b - 48.0 * c) * ax
            + (8.0 * b + 24.0 * c))
            / 6.0
    } else {
        0.0
    }
}

#[inline(always)]
pub fn filter_catmull_rom(x: f32) -> f32 {
    let ax = x.abs();
    if ax < 1.0 {
        1.5 * ax * ax * ax - 2.5 * ax * ax + 1.0
    } else if ax < 2.0 {
        -0.5 * ax * ax * ax + 2.5 * ax * ax - 4.0 * ax + 2.0
    } else {
        0.0
    }
}

#[inline(always)]
pub fn filter_lanczos3(x: f32) -> f32 {
    let ax = x.abs();
    if ax < 3.0 {
        filter_sinc(ax) * filter_sinc(ax / 3.0)
    } else {
        0.0
    }
}

#[inline(always)]
pub fn filter_kaiser_sinc(x: f32) -> f32 {
    let ax = x.abs();
    let radius = 3.0;
    let alpha = 4.0;
    if ax < radius {
        let window =
            bessel_i0(alpha * (1.0 - (ax / radius).powi(2)).max(0.0).sqrt()) / bessel_i0(alpha);
        filter_sinc(ax) * window
    } else {
        0.0
    }
}

pub fn generate_mipmaps_rgba(
    src_rgba: &[u8],
    width: usize,
    height: usize,
    filter: MipmapFilter,
    srgb: bool,
    alpha_coverage: Option<AlphaCoverageOptions>,
) -> Vec<MipLevel> {
    let mut levels = Vec::new();
    levels.push(MipLevel {
        width,
        height,
        data: src_rgba.to_vec(),
    });

    let mut cur_w = width;
    let mut cur_h = height;
    let mut cur_data = src_rgba.to_vec();

    let target_coverage = if let Some(opts) = alpha_coverage {
        calculate_alpha_coverage(&cur_data, cur_w, cur_h, opts.alpha_cutoff)
    } else {
        0.0
    };

    while cur_w > 1 || cur_h > 1 {
        let next_w = (cur_w >> 1).max(1);
        let next_h = (cur_h >> 1).max(1);

        let mut next_data = match filter {
            MipmapFilter::Point => downsample_point(&cur_data, cur_w, next_w, next_h),
            MipmapFilter::Box => downsample_box(&cur_data, cur_w, cur_h, next_w, next_h, srgb),
            MipmapFilter::MitchellNetravali => downsample_separable_kernel(
                &cur_data,
                cur_w,
                cur_h,
                next_w,
                next_h,
                srgb,
                2.0,
                filter_mitchell_netravali,
            ),
            MipmapFilter::CatmullRom => downsample_separable_kernel(
                &cur_data,
                cur_w,
                cur_h,
                next_w,
                next_h,
                srgb,
                2.0,
                filter_catmull_rom,
            ),
            MipmapFilter::Lanczos3 => downsample_separable_kernel(
                &cur_data,
                cur_w,
                cur_h,
                next_w,
                next_h,
                srgb,
                3.0,
                filter_lanczos3,
            ),
            MipmapFilter::KaiserSinc => downsample_separable_kernel(
                &cur_data,
                cur_w,
                cur_h,
                next_w,
                next_h,
                srgb,
                3.0,
                filter_kaiser_sinc,
            ),
        };

        if let Some(opts) = alpha_coverage {
            scale_alpha_for_coverage(
                &mut next_data,
                next_w,
                next_h,
                opts.alpha_cutoff,
                target_coverage,
            );
        }

        levels.push(MipLevel {
            width: next_w,
            height: next_h,
            data: next_data.clone(),
        });

        cur_w = next_w;
        cur_h = next_h;
        cur_data = next_data;
    }

    levels
}

fn downsample_box(src: &[u8], sw: usize, sh: usize, dw: usize, dh: usize, srgb: bool) -> Vec<u8> {
    let mut dest = vec![0u8; dw * dh * 4];
    for dy in 0..dh {
        for dx in 0..dw {
            let sx0 = dx * 2;
            let sy0 = dy * 2;
            let sx1 = (sx0 + 1).min(sw - 1);
            let sy1 = (sy0 + 1).min(sh - 1);

            let p00 = get_px(src, sw, sx0, sy0);
            let p10 = get_px(src, sw, sx1, sy0);
            let p01 = get_px(src, sw, sx0, sy1);
            let p11 = get_px(src, sw, sx1, sy1);

            let out_idx = (dy * dw + dx) * 4;

            if srgb {
                let r = (srgb_to_linear(p00[0])
                    + srgb_to_linear(p10[0])
                    + srgb_to_linear(p01[0])
                    + srgb_to_linear(p11[0]))
                    * 0.25;
                let g = (srgb_to_linear(p00[1])
                    + srgb_to_linear(p10[1])
                    + srgb_to_linear(p01[1])
                    + srgb_to_linear(p11[1]))
                    * 0.25;
                let b = (srgb_to_linear(p00[2])
                    + srgb_to_linear(p10[2])
                    + srgb_to_linear(p01[2])
                    + srgb_to_linear(p11[2]))
                    * 0.25;
                let a = (p00[3] as u32 + p10[3] as u32 + p01[3] as u32 + p11[3] as u32 + 2) / 4;

                dest[out_idx] = linear_to_srgb(r);
                dest[out_idx + 1] = linear_to_srgb(g);
                dest[out_idx + 2] = linear_to_srgb(b);
                dest[out_idx + 3] = a as u8;
            } else {
                for c in 0..4 {
                    dest[out_idx + c] =
                        ((p00[c] as u32 + p10[c] as u32 + p01[c] as u32 + p11[c] as u32 + 2) / 4)
                            as u8;
                }
            }
        }
    }
    dest
}

fn downsample_point(src: &[u8], sw: usize, dw: usize, dh: usize) -> Vec<u8> {
    let mut dest = vec![0u8; dw * dh * 4];
    for dy in 0..dh {
        for dx in 0..dw {
            let sx = dx * 2;
            let sy = dy * 2;
            let px = get_px(src, sw, sx, sy);
            let out_idx = (dy * dw + dx) * 4;
            dest[out_idx..out_idx + 4].copy_from_slice(&px);
        }
    }
    dest
}

#[allow(clippy::too_many_arguments)]
fn downsample_separable_kernel<F>(
    src: &[u8],
    sw: usize,
    sh: usize,
    dw: usize,
    dh: usize,
    srgb: bool,
    radius: f32,
    kernel: F,
) -> Vec<u8>
where
    F: Fn(f32) -> f32,
{
    let mut dest = vec![0u8; dw * dh * 4];
    let taps = (radius * 2.0).ceil() as isize;

    for dy in 0..dh {
        let center_y = (dy as f32 + 0.5) * 2.0 - 0.5;

        for dx in 0..dw {
            let center_x = (dx as f32 + 0.5) * 2.0 - 0.5;
            let mut accum = Vec4::splat(0.0);
            let mut total_weight = 0.0f32;

            for ky in -taps..=taps {
                let sy =
                    ((center_y + ky as f32).round() as isize).clamp(0, sh as isize - 1) as usize;
                let wy = kernel(ky as f32 / radius);

                for kx in -taps..=taps {
                    let sx = ((center_x + kx as f32).round() as isize).clamp(0, sw as isize - 1)
                        as usize;
                    let wx = kernel(kx as f32 / radius);
                    let w = wx * wy;

                    if w.abs() > 1e-6 {
                        let p = get_px(src, sw, sx, sy);
                        let sample = if srgb {
                            Vec4::new(
                                srgb_to_linear(p[0]),
                                srgb_to_linear(p[1]),
                                srgb_to_linear(p[2]),
                                p[3] as f32 / 255.0,
                            )
                        } else {
                            Vec4::new(p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32)
                        };

                        accum += sample * w;
                        total_weight += w;
                    }
                }
            }

            let inv_w = if total_weight.abs() > 1e-6 {
                1.0 / total_weight
            } else {
                1.0
            };
            let result = accum * inv_w;
            let out_idx = (dy * dw + dx) * 4;

            if srgb {
                dest[out_idx] = linear_to_srgb(result.x);
                dest[out_idx + 1] = linear_to_srgb(result.y);
                dest[out_idx + 2] = linear_to_srgb(result.z);
                dest[out_idx + 3] = (result.w * 255.0).round().clamp(0.0, 255.0) as u8;
            } else {
                dest[out_idx] = result.x.round().clamp(0.0, 255.0) as u8;
                dest[out_idx + 1] = result.y.round().clamp(0.0, 255.0) as u8;
                dest[out_idx + 2] = result.z.round().clamp(0.0, 255.0) as u8;
                dest[out_idx + 3] = result.w.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    dest
}

#[inline(always)]
fn get_px(src: &[u8], w: usize, x: usize, y: usize) -> [u8; 4] {
    let idx = (y * w + x) * 4;
    [src[idx], src[idx + 1], src[idx + 2], src[idx + 3]]
}

pub fn calculate_alpha_coverage(rgba: &[u8], width: usize, height: usize, cutoff: f32) -> f32 {
    if width <= 1 || height <= 1 {
        return 1.0;
    }
    let cutoff_u8 = (cutoff * 255.0).round().clamp(0.0, 255.0) as u8;
    let mut count = 0usize;
    let total = width * height;

    for i in 0..total {
        if rgba[i * 4 + 3] >= cutoff_u8 {
            count += 1;
        }
    }
    count as f32 / total as f32
}

pub fn scale_alpha_for_coverage(
    rgba: &mut [u8],
    _width: usize,
    _height: usize,
    cutoff: f32,
    target_coverage: f32,
) {
    if target_coverage <= 0.0 || target_coverage >= 1.0 {
        return;
    }

    let mut min_scale = 0.0f32;
    let mut max_scale = 4.0f32;
    let mut best_scale = 1.0f32;
    let original_alphas: Vec<u8> = rgba.chunks_exact(4).map(|px| px[3]).collect();

    for _ in 0..10 {
        let scale = (min_scale + max_scale) * 0.5;
        let mut pass_count = 0usize;
        let cutoff_u8 = (cutoff * 255.0).round() as u32;

        for &a in &original_alphas {
            let scaled = ((a as f32 * scale).round().clamp(0.0, 255.0)) as u32;
            if scaled >= cutoff_u8 {
                pass_count += 1;
            }
        }

        let coverage = pass_count as f32 / original_alphas.len() as f32;
        if coverage < target_coverage {
            min_scale = scale;
        } else {
            max_scale = scale;
        }
        best_scale = scale;
    }

    for (i, px) in rgba.chunks_exact_mut(4).enumerate() {
        let a = original_alphas[i] as f32 * best_scale;
        px[3] = a.round().clamp(0.0, 255.0) as u8;
    }
}
