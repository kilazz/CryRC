// Copyright 2001-2026 Crytek GmbH / Crytek Group. All rights reserved.
// Texture Compression Format Flags and Pipeline Configuration Options

use crate::math::vector::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    Bc1,
    Bc2,
    Bc3,
    Bc4,
    Bc5,
    Bc6h,
    Bc7,
    Ctx1,
}

impl Format {
    #[inline(always)]
    pub const fn bytes_per_block(&self) -> usize {
        match self {
            Format::Bc1 | Format::Bc4 | Format::Ctx1 => 8,
            Format::Bc2 | Format::Bc3 | Format::Bc5 | Format::Bc6h | Format::Bc7 => 16,
        }
    }

    pub const fn dxgi_format(&self, is_signed: bool, is_srgb: bool) -> u32 {
        match self {
            Format::Bc1 => {
                if is_srgb {
                    72
                } else {
                    71
                }
            }
            Format::Bc2 => {
                if is_srgb {
                    75
                } else {
                    74
                }
            }
            Format::Bc3 => {
                if is_srgb {
                    78
                } else {
                    77
                }
            }
            Format::Bc4 => {
                if is_signed {
                    81
                } else {
                    80
                }
            }
            Format::Bc5 => {
                if is_signed {
                    84
                } else {
                    83
                }
            }
            Format::Bc6h => {
                if is_signed {
                    96
                } else {
                    95
                }
            }
            Format::Bc7 => {
                if is_srgb {
                    99
                } else {
                    98
                }
            }
            Format::Ctx1 => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QualityLevel {
    Ultrafast,
    Fast,
    #[default]
    Normal,
    Slow,
    Slowest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMetric {
    Perceptual,
    Uniform,
    Unit,
}

impl ColorMetric {
    #[inline(always)]
    pub fn vector(&self) -> Vec3 {
        match self {
            ColorMetric::Uniform => Vec3::new(1.0, 1.0, 1.0),
            ColorMetric::Perceptual => Vec3::new(0.2125 / 0.7154, 1.0, 0.0721 / 0.7154),
            ColorMetric::Unit => Vec3::new(0.5, 0.5, 0.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitStrategy {
    FastRange,
    Cluster(usize),
}

/// Resampling and MIP generation filter kernels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MipmapFilter {
    #[default]
    Box,
    MitchellNetravali,
    CatmullRom,
    Lanczos3,
    KaiserSinc,
    Point,
}

#[derive(Debug, Clone, Copy)]
pub struct CompressionOptions {
    pub format: Format,
    pub metric: ColorMetric,
    pub strategy: FitStrategy,
    pub quality: QualityLevel,
    pub weight_by_alpha: bool,
    pub is_1bit_alpha: bool,
    pub alpha_iterative_fit: bool,
    pub is_signed: bool,
    pub is_normal_map: bool,
    pub srgb: bool,
    pub dither_rgb: bool,
    pub dither_a: bool,
    pub rdo_lambda: f32,
    pub rdo_ultrasmooth: bool,
    pub rdo_lookback_window: usize,
    pub rdo_try_two_matches: bool,
    pub rdo_smooth_block_scale: Option<f32>,
}

impl Default for CompressionOptions {
    fn default() -> Self {
        Self {
            format: Format::Bc1,
            metric: ColorMetric::Perceptual,
            strategy: FitStrategy::Cluster(8),
            quality: QualityLevel::Normal,
            weight_by_alpha: false,
            is_1bit_alpha: false,
            alpha_iterative_fit: false,
            is_signed: false,
            is_normal_map: false,
            srgb: false,
            dither_rgb: false,
            dither_a: false,
            rdo_lambda: 0.0,
            rdo_ultrasmooth: true,
            rdo_lookback_window: 256,
            rdo_try_two_matches: true,
            rdo_smooth_block_scale: None,
        }
    }
}

impl CompressionOptions {
    pub const fn bc1() -> Self {
        Self {
            format: Format::Bc1,
            metric: ColorMetric::Perceptual,
            strategy: FitStrategy::Cluster(8),
            quality: QualityLevel::Normal,
            weight_by_alpha: false,
            is_1bit_alpha: false,
            alpha_iterative_fit: false,
            is_signed: false,
            is_normal_map: false,
            srgb: false,
            dither_rgb: false,
            dither_a: false,
            rdo_lambda: 0.0,
            rdo_ultrasmooth: true,
            rdo_lookback_window: 256,
            rdo_try_two_matches: true,
            rdo_smooth_block_scale: None,
        }
    }

    pub const fn bc2() -> Self {
        Self {
            format: Format::Bc2,
            metric: ColorMetric::Perceptual,
            strategy: FitStrategy::Cluster(8),
            quality: QualityLevel::Normal,
            weight_by_alpha: false,
            is_1bit_alpha: false,
            alpha_iterative_fit: false,
            is_signed: false,
            is_normal_map: false,
            srgb: false,
            dither_rgb: false,
            dither_a: false,
            rdo_lambda: 0.0,
            rdo_ultrasmooth: true,
            rdo_lookback_window: 256,
            rdo_try_two_matches: true,
            rdo_smooth_block_scale: None,
        }
    }

    pub const fn bc3() -> Self {
        Self {
            format: Format::Bc3,
            metric: ColorMetric::Perceptual,
            strategy: FitStrategy::Cluster(8),
            quality: QualityLevel::Normal,
            weight_by_alpha: true,
            is_1bit_alpha: false,
            alpha_iterative_fit: true,
            is_signed: false,
            is_normal_map: false,
            srgb: false,
            dither_rgb: false,
            dither_a: false,
            rdo_lambda: 0.0,
            rdo_ultrasmooth: true,
            rdo_lookback_window: 256,
            rdo_try_two_matches: true,
            rdo_smooth_block_scale: None,
        }
    }

    pub const fn bc4() -> Self {
        Self {
            format: Format::Bc4,
            metric: ColorMetric::Uniform,
            strategy: FitStrategy::Cluster(8),
            quality: QualityLevel::Normal,
            weight_by_alpha: false,
            is_1bit_alpha: false,
            alpha_iterative_fit: true,
            is_signed: false,
            is_normal_map: false,
            srgb: false,
            dither_rgb: false,
            dither_a: false,
            rdo_lambda: 0.0,
            rdo_ultrasmooth: true,
            rdo_lookback_window: 256,
            rdo_try_two_matches: true,
            rdo_smooth_block_scale: None,
        }
    }

    pub const fn bc5() -> Self {
        Self {
            format: Format::Bc5,
            metric: ColorMetric::Uniform,
            strategy: FitStrategy::Cluster(8),
            quality: QualityLevel::Normal,
            weight_by_alpha: false,
            is_1bit_alpha: false,
            alpha_iterative_fit: true,
            is_signed: false,
            is_normal_map: false,
            srgb: false,
            dither_rgb: false,
            dither_a: false,
            rdo_lambda: 0.0,
            rdo_ultrasmooth: true,
            rdo_lookback_window: 256,
            rdo_try_two_matches: true,
            rdo_smooth_block_scale: None,
        }
    }

    pub const fn bc5_normals() -> Self {
        Self {
            format: Format::Bc5,
            metric: ColorMetric::Unit,
            strategy: FitStrategy::FastRange,
            quality: QualityLevel::Normal,
            weight_by_alpha: false,
            is_1bit_alpha: false,
            alpha_iterative_fit: true,
            is_signed: false,
            is_normal_map: true,
            srgb: false,
            dither_rgb: false,
            dither_a: false,
            rdo_lambda: 0.0,
            rdo_ultrasmooth: true,
            rdo_lookback_window: 256,
            rdo_try_two_matches: true,
            rdo_smooth_block_scale: None,
        }
    }

    pub const fn bc6h() -> Self {
        Self {
            format: Format::Bc6h,
            metric: ColorMetric::Uniform,
            strategy: FitStrategy::Cluster(4),
            quality: QualityLevel::Normal,
            weight_by_alpha: false,
            is_1bit_alpha: false,
            alpha_iterative_fit: false,
            is_signed: false,
            is_normal_map: false,
            srgb: false,
            dither_rgb: false,
            dither_a: false,
            rdo_lambda: 0.0,
            rdo_ultrasmooth: true,
            rdo_lookback_window: 256,
            rdo_try_two_matches: true,
            rdo_smooth_block_scale: None,
        }
    }

    pub const fn bc7() -> Self {
        Self {
            format: Format::Bc7,
            metric: ColorMetric::Perceptual,
            strategy: FitStrategy::Cluster(8),
            quality: QualityLevel::Normal,
            weight_by_alpha: true,
            is_1bit_alpha: false,
            alpha_iterative_fit: true,
            is_signed: false,
            is_normal_map: false,
            srgb: false,
            dither_rgb: false,
            dither_a: false,
            rdo_lambda: 0.0,
            rdo_ultrasmooth: true,
            rdo_lookback_window: 256,
            rdo_try_two_matches: true,
            rdo_smooth_block_scale: None,
        }
    }

    pub const fn bc7_ultrafast() -> Self {
        Self {
            format: Format::Bc7,
            metric: ColorMetric::Perceptual,
            strategy: FitStrategy::FastRange,
            quality: QualityLevel::Ultrafast,
            weight_by_alpha: true,
            is_1bit_alpha: false,
            alpha_iterative_fit: false,
            is_signed: false,
            is_normal_map: false,
            srgb: false,
            dither_rgb: false,
            dither_a: false,
            rdo_lambda: 0.0,
            rdo_ultrasmooth: true,
            rdo_lookback_window: 256,
            rdo_try_two_matches: true,
            rdo_smooth_block_scale: None,
        }
    }

    pub const fn bc7_fast() -> Self {
        Self {
            format: Format::Bc7,
            metric: ColorMetric::Perceptual,
            strategy: FitStrategy::Cluster(4),
            quality: QualityLevel::Fast,
            weight_by_alpha: true,
            is_1bit_alpha: false,
            alpha_iterative_fit: false,
            is_signed: false,
            is_normal_map: false,
            srgb: false,
            dither_rgb: false,
            dither_a: false,
            rdo_lambda: 0.0,
            rdo_ultrasmooth: true,
            rdo_lookback_window: 256,
            rdo_try_two_matches: true,
            rdo_smooth_block_scale: None,
        }
    }

    pub const fn bc7_slow() -> Self {
        Self {
            format: Format::Bc7,
            metric: ColorMetric::Perceptual,
            strategy: FitStrategy::Cluster(8),
            quality: QualityLevel::Slow,
            weight_by_alpha: true,
            is_1bit_alpha: false,
            alpha_iterative_fit: true,
            is_signed: false,
            is_normal_map: false,
            srgb: false,
            dither_rgb: false,
            dither_a: false,
            rdo_lambda: 0.0,
            rdo_ultrasmooth: true,
            rdo_lookback_window: 256,
            rdo_try_two_matches: true,
            rdo_smooth_block_scale: None,
        }
    }

    pub const fn bc7_slowest() -> Self {
        Self {
            format: Format::Bc7,
            metric: ColorMetric::Perceptual,
            strategy: FitStrategy::Cluster(8),
            quality: QualityLevel::Slowest,
            weight_by_alpha: true,
            is_1bit_alpha: false,
            alpha_iterative_fit: true,
            is_signed: false,
            is_normal_map: false,
            srgb: false,
            dither_rgb: false,
            dither_a: false,
            rdo_lambda: 0.0,
            rdo_ultrasmooth: true,
            rdo_lookback_window: 256,
            rdo_try_two_matches: true,
            rdo_smooth_block_scale: None,
        }
    }

    pub const fn ctx1() -> Self {
        Self {
            format: Format::Ctx1,
            metric: ColorMetric::Unit,
            strategy: FitStrategy::FastRange,
            quality: QualityLevel::Normal,
            weight_by_alpha: false,
            is_1bit_alpha: false,
            alpha_iterative_fit: false,
            is_signed: false,
            is_normal_map: true,
            srgb: false,
            dither_rgb: false,
            dither_a: false,
            rdo_lambda: 0.0,
            rdo_ultrasmooth: true,
            rdo_lookback_window: 256,
            rdo_try_two_matches: true,
            rdo_smooth_block_scale: None,
        }
    }
}
