pub mod decode;
pub mod encode;
pub mod fit;
pub mod tables;

pub use decode::{apply_rotation, decompress_bc7_block, interpolate_color};
pub use encode::{
    encode_bc7_block_mode6, expand_quantized, fix_bc7_anchor_indices, quantize_endpoint,
    write_bc7_block,
};
pub use fit::{
    PaletteClusterFit, PaletteRangeFit, PaletteSet, handle_alpha_block, handle_opaque_block,
};
pub use tables::{BC7_MODE_INFO, BC7_TABLES, BC7ModeInfo, Bc7Tables};

use crate::flags::QualityLevel;
use crate::math::vector::Vec4;

/// Compresses a 4x4 block of RGBA pixels into a 16-byte BC7 block using the specified quality preset.
pub fn compress_bc7_block(
    rgba: &[[u8; 4]; 16],
    mask: u16,
    _flags: u32,
    metric: &Vec4,
    quality: QualityLevel,
    out_block: &mut [u8; 16],
) {
    let mut lo_a = 255u8;
    for (i, px) in rgba.iter().enumerate() {
        if (mask & (1 << i)) != 0 {
            lo_a = lo_a.min(px[3]);
        }
    }

    if lo_a < 255 {
        handle_alpha_block(rgba, mask, metric, quality, out_block);
    } else {
        handle_opaque_block(rgba, mask, metric, quality, out_block);
    }
}
