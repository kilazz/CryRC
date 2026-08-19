pub mod decode;
pub mod encode;
pub mod fit;
pub mod quant;
pub mod tables;

pub use decode::decompress_bc6h_block;
pub use encode::write_bc6h_block;
pub use fit::{HDRClusterFit, HDRRangeFit, HDRSet};
pub use quant::{float_to_half, half_to_float, sign_extend, unquantize_bc6h};
pub use tables::{BC6H_CONFIGS, BC6HModeConfig};

use crate::flags::FitStrategy;
use crate::math::vector::Vec3;

/// Compresses a 4x4 block of 32-bit floating-point HDR RGB pixels into a 16-byte BC6H block.
pub fn compress_bc6h_block(
    rgb: &[[f32; 3]; 16],
    mask: u16,
    metric: &Vec3,
    is_signed: bool,
    strategy: FitStrategy,
    out_block: &mut [u8; 16],
) {
    let mut best_error = f32::MAX;
    let mut best_block = [0u8; 16];

    let initial_1subset = HDRSet::new(rgb, mask, 1, 0);

    for mode in 11..=14 {
        let mut block = [0u8; 16];
        let err = match strategy {
            FitStrategy::FastRange => {
                let fit = HDRRangeFit::new(&initial_1subset);
                fit.compress(&initial_1subset, mode, metric, is_signed, &mut block)
            }
            FitStrategy::Cluster(iters) => {
                let fit = HDRClusterFit::new(iters);
                fit.compress(&initial_1subset, mode, metric, is_signed, &mut block)
            }
        };

        if err < best_error {
            best_error = err;
            best_block = block;
        }
    }

    for p in 0..32 {
        let subset_p = HDRSet::from_initial(&initial_1subset, p);

        for mode in 1..=10 {
            let mut block = [0u8; 16];
            let err = match strategy {
                FitStrategy::FastRange => {
                    let fit = HDRRangeFit::new(&subset_p);
                    fit.compress(&subset_p, mode, metric, is_signed, &mut block)
                }
                FitStrategy::Cluster(iters) => {
                    let fit = HDRClusterFit::new(iters);
                    fit.compress(&subset_p, mode, metric, is_signed, &mut block)
                }
            };

            if err < best_error {
                best_error = err;
                best_block = block;
            }
        }
    }

    *out_block = best_block;
}
