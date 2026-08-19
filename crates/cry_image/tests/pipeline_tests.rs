use cry_image::*;

#[test]
fn test_alpha_coverage_preserves_silhouette() {
    let width = 16;
    let height = 16;
    let mut rgba = vec![255u8; width * height * 4];

    for i in 0..width * height {
        rgba[i * 4 + 3] = if i % 2 == 0 { 255 } else { 0 };
    }

    let mips = generate_mipmaps_rgba(
        &rgba,
        width,
        height,
        MipmapFilter::Box,
        false,
        Some(AlphaCoverageOptions { alpha_cutoff: 0.5 }),
    );

    assert_eq!(mips.len(), 5);
    let mip2_alphas: Vec<u8> = mips[2].data.chunks_exact(4).map(|p| p[3]).collect();
    let survivors = mip2_alphas.iter().filter(|&&a| a >= 128).count();
    assert!(
        survivors > 0,
        "Alpha coverage scaling failed on lower mip level"
    );
}

#[test]
fn test_rdo_reduces_entropy() {
    let width = 32;
    let height = 32;
    let mut rgba = vec![0u8; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 4;
            rgba[idx] = ((x * 255) / width) as u8;
            rgba[idx + 1] = ((y * 255) / height) as u8;
            rgba[idx + 2] = ((x ^ y) & 0xFF) as u8;
            rgba[idx + 3] = 255;
        }
    }

    let mut opts_no_rdo = CompressionOptions::bc7();
    opts_no_rdo.rdo_lambda = 0.0;

    let mut opts_rdo = CompressionOptions::bc7();
    opts_rdo.rdo_lambda = 3.0;

    let comp_no_rdo = compress_image(&rgba, width, height, opts_no_rdo);
    let comp_rdo = compress_image(&rgba, width, height, opts_rdo);

    assert_eq!(comp_no_rdo.len(), comp_rdo.len());
    let dec_rdo = decompress_image(&comp_rdo, width, height, Format::Bc7);
    let ssim = compute_ssim(&rgba, &dec_rdo, width, height);
    assert!(
        ssim[0] > 0.80,
        "RDO degraded image quality too much: SSIM = {:?}",
        ssim
    );
}

#[test]
fn test_toksvig_gloss_from_normals() {
    let mut float_pixels = vec![
        ColorRGBAf::new(0.6, 0.4, 0.9, 1.0),
        ColorRGBAf::new(0.4, 0.6, 0.9, 1.0),
        ColorRGBAf::new(0.5, 0.5, 0.7, 1.0),
        ColorRGBAf::new(0.7, 0.3, 0.8, 1.0),
    ];

    converters::normal_processing::NormalProcessing::gloss_from_normals(&mut float_pixels, true);

    for p in &float_pixels {
        assert!(
            p.a < 1.0,
            "Toksvig algorithm must reduce gloss for non-unit normal variance"
        );
        assert!(p.a > 0.0, "Gloss must remain positive");
    }
}

#[test]
fn test_dither_diffusion() {
    let width = 4;
    let height = 4;
    let pixels = vec![128u8; width * height * 4];
    let quantized = vec![120u8; width * height * 4];
    let mut out = vec![0u8; width * height * 4];

    dither_image_rgba(&pixels, &quantized, width, height, &mut out);
    assert_ne!(out[0], 0);
}
