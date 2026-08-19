pub mod alpha;
pub mod dither;
pub mod metrics;
pub mod mipmaps;
pub mod normalmap;
pub mod rdo;
pub mod ultrasmooth;

pub use alpha::{demultiply_alpha, premultiply_alpha};
pub use dither::{dither_block_rgba, dither_image_rgba};
pub use metrics::{ImageMetrics, TrackedStat, compute_ssim};
pub use mipmaps::{AlphaCoverageOptions, generate_mipmaps_rgba};
pub use normalmap::{NormalMapOptions, compute_normal_map};
pub use rdo::{
    ReduceEntropyParams, apply_rdo_optimization, reduce_entropy, reduce_entropy_strided,
};
pub use ultrasmooth::{ULTRASMOOTH_BLOCK_MSE_SCALE, compute_block_mse_scales};
