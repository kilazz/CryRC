// Copyright 2001-2026 Crytek GmbH / Crytek Group. All rights reserved.
// Unified CryEngine Texture Processing Engine & Block Compressor

pub mod api;
pub mod color_types;
pub mod compressors;
pub mod converters;
pub mod filtering;
pub mod flags;
pub mod formats;
pub mod image_compiler;
pub mod image_details;
pub mod image_object;
pub mod image_properties;
pub mod math;
pub mod operations;
pub mod pipeline;
pub mod pixel_formats;
pub mod quantize;
pub mod streaming;
pub mod tables;

pub use color_types::{ColorRGBA8, ColorRGBAf};

// =============================================================================
// Top-Level Public Compression & Decompression APIs
// =============================================================================
pub use api::{
    compress_block, compress_image, compress_image_hdr, decompress_image, decompress_image_hdr,
    get_storage_requirements,
};

// =============================================================================
// Block Compressor Implementations (BC1-BC7, BC6H, CTX1, ASTC, PVRTC/ETC2/EAC)
// =============================================================================
pub use compressors::astc_compressor::{
    AstcBlockDim, AstcCompressor, AstcPixelFormat, AstcQuality,
};
pub use compressors::bc1::{
    compress_bc1_block, read_color_block_bc1, write_color_block_3, write_color_block_4,
};
pub use compressors::bc2::{
    compress_alpha_bc2, decompress_alpha_bc2, read_alpha_block_bc2, write_alpha_block_bc2,
};
pub use compressors::bc3::{
    compress_alpha_bc3, decompress_alpha_bc3, read_alpha_block_bc3, write_alpha_block_bc3,
};
pub use compressors::bc4::{
    compress_bc4, compress_bc4_i16, compress_bc4_signed, compress_bc4_u16, decompress_bc4,
    decompress_bc4_i16, decompress_bc4_signed, decompress_bc4_u16,
};
pub use compressors::bc5::{
    compress_bc5, compress_bc5_i16, compress_bc5_normals, compress_bc5_normals_signed,
    compress_bc5_signed, compress_bc5_u16, decompress_bc5, decompress_bc5_i16,
    decompress_bc5_normals, decompress_bc5_normals_signed, decompress_bc5_signed,
    decompress_bc5_u16,
};
pub use compressors::bc6h::{
    compress_bc6h_block, decompress_bc6h_block, float_to_half, half_to_float, sign_extend,
    unquantize_bc6h,
};
pub use compressors::bc7::{compress_bc7_block, decompress_bc7_block};
pub use compressors::ctx1::{
    compress_ctx1_block, decompress_ctx1_block, decompress_ctx1_normals_block,
};
pub use compressors::pvrtc_compressor::{PvrPixelFormat, PvrQuality, PvrtcCompressor};

// =============================================================================
// Flags & Options
// =============================================================================
pub use flags::{ColorMetric, CompressionOptions, FitStrategy, Format, MipmapFilter, QualityLevel};

// =============================================================================
// CryEngine Image Structures & Compiler
// =============================================================================
pub use image_compiler::{ImageCompiler, LoadedSourceImage};
pub use image_details::ImageDetails;
pub use image_object::{
    EAlphaContent, ECubemap, EIF_ATTACHEDALPHA, EIF_CUBEMAP, EIF_DECAL, EIF_FILESINGLE,
    EIF_RENORMALIZEDTEXTURE, EIF_SPLITTED, EIF_SRGBREAD, EIF_VOLUMETEXTURE, ImageObject, MipLevel,
};
pub use image_properties::{
    EColorModel, EInputColorSpace, EOutputColorSpace, ImageProperties, ReduceItem,
};
pub use pixel_formats::{CPixelFormats, DxgiFormat, EPixelFormat, PixelFormatInfo};
pub use streaming::{TextureHelper, TextureSplitter, TextureSplitterConfig};

// =============================================================================
// Color Conversion & Normal Processing (Scharr, Farid, Sobel, Toksvig)
// =============================================================================
pub use converters::colorspaces::*;
pub use converters::{
    BumpProperties, ChannelConverter, NormalFilterType, NormalProcessing, Rgb9E5,
};

// =============================================================================
// Image Processing Pipeline, Mipmaps, RDO & Post-Processing
// =============================================================================
pub use operations::color_chart::{ColorChart, Lut3DColorChart};
pub use operations::combine_normals::CombineNormals;
pub use operations::normalize::RangeNormalizer;
pub use pipeline::alpha::{demultiply_alpha, premultiply_alpha};
pub use pipeline::dither::{dither_block_rgba, dither_image_rgba};
pub use pipeline::metrics::{ImageMetrics, TrackedStat, compute_ssim};
pub use pipeline::mipmaps::{AlphaCoverageOptions, generate_mipmaps_rgba};
pub use pipeline::normalmap::{NormalMapOptions, compute_normal_map};
pub use pipeline::rdo::{
    ReduceEntropyParams, apply_rdo_optimization, reduce_entropy, reduce_entropy_strided,
};
pub use pipeline::ultrasmooth::{ULTRASMOOTH_BLOCK_MSE_SCALE, compute_block_mse_scales};

// =============================================================================
// SIMD Math & Geometry Vectors
// =============================================================================
pub use math::bitstream::BitStream128;
pub use math::normal::{
    DEVIANCE_BASE, DEVIANCE_MAX, add_deviance, codebook_3_normal, codebook_4_normal, complement_z,
    min_deviance_3, min_deviance_4, snorm_to_unorm, unorm_to_snorm,
};
pub use math::pca::{
    Sym2x2, Sym3x3, Sym4x4, compute_weighted_covariance3, compute_weighted_covariance4,
    estimate_principle_component, estimate_principle_component_vec4, get_principle_projection_vec3,
    get_principle_projection_vec4, solve_least_squares_vec3, solve_least_squares_vec4,
};
pub use math::vector::{Col3, Col4, Vec3, Vec4};

// =============================================================================
// Container I/O (CryTIF, DDS, Radiance HDR, IPTC, TGA)
// =============================================================================
pub use formats::crytif_io::CryTifIO;
pub use formats::dds_io::DdsIO;
pub use formats::hdr_rgbe_io::HdrRgbeIO;
pub use formats::iptc_header::{FIELD_SPECIAL_INSTRUCTIONS, IptcHeader};
pub use formats::tga_io::TgaIO;
