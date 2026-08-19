pub mod bitstream;
pub mod normal;
pub mod pca;
pub mod vector;

pub use bitstream::BitStream128;
pub use normal::{
    DEVIANCE_BASE, DEVIANCE_MAX, add_deviance, codebook_3_normal, codebook_4_normal, complement_z,
    min_deviance_3, min_deviance_4, snorm_to_unorm, unorm_to_snorm,
};
pub use pca::{
    Sym2x2, Sym3x3, Sym4x4, compute_weighted_covariance3, compute_weighted_covariance4,
    estimate_principle_component, estimate_principle_component_vec4, get_principle_projection_vec3,
    get_principle_projection_vec4, solve_least_squares_vec3, solve_least_squares_vec4,
};
pub use vector::{Col3, Col4, Vec3, Vec4};
