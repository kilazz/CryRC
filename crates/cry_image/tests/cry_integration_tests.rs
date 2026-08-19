use cry_image::*;
use std::fs;

#[test]
fn test_crytif_special_instructions_roundtrip() {
    let temp_dir = std::env::temp_dir().join("cry_image_tif_test");
    let _ = fs::create_dir_all(&temp_dir);
    let tif_path = temp_dir.join("test_texture.tif");

    let width = 8u32;
    let height = 8u32;
    let rgba_pixels = vec![128u8; (width * height * 4) as usize];
    let custom_instructions = "/preset=AlbedoWithCoverage /reduce=0 /colorspace=sRGB,auto";

    CryTifIO::save_crytif_rgba8(&tif_path, width, height, &rgba_pixels, custom_instructions)
        .unwrap();

    let read_back = CryTifIO::read_special_instructions(&tif_path).unwrap();
    assert_eq!(read_back, custom_instructions);

    let _ = fs::remove_file(tif_path);
}

#[test]
fn test_full_image_compiler_pipeline() {
    let temp_dir = std::env::temp_dir().join("cry_image_compiler_test");
    let _ = fs::create_dir_all(&temp_dir);
    let src_tif = temp_dir.join("foliage_diff.tif");
    let out_dds = temp_dir.join("foliage_diff.dds");

    let width = 16u32;
    let height = 16u32;
    let rgba_pixels = vec![200u8; (width * height * 4) as usize];
    CryTifIO::save_crytif_rgba8(&src_tif, width, height, &rgba_pixels, "/preset=Albedo").unwrap();

    let mut compiler = ImageCompiler::new(ImageProperties::default());
    compiler.split_for_streaming = true;

    let output_files = compiler.process_file(&src_tif, &out_dds, None).unwrap();
    assert!(!output_files.is_empty());
    assert!(out_dds.exists());

    let mut details = Vec::new();
    ImageDetails::collect_dds_details(&out_dds, &mut details).unwrap();
    assert!(details.iter().any(|d| d.name == "width" && d.value == "16"));
    assert!(details.iter().any(|d| d.name == "mipCount"));

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_color_chart_3d_lut_generation() {
    let chart = Lut3DColorChart::default();
    let img_opt = chart.generate_chart_image();
    assert!(img_opt.is_some());

    let (w, h, data) = img_opt.unwrap();
    assert_eq!(w, 256);
    assert_eq!(h, 16);
    assert_eq!(data.len(), 256 * 16 * 4);
}

#[test]
fn test_streaming_texture_splitter_chunks() {
    let temp_dir = std::env::temp_dir().join("cry_stream_test");
    let _ = fs::create_dir_all(&temp_dir);

    let src_dds = temp_dir.join("stream_test.dds");
    let payload = vec![
        0u8;
        (64 / 4) * (64 / 4) * 8
            + (32 / 4) * (32 / 4) * 8
            + (16 / 4) * (16 / 4) * 8
            + (8 / 4) * (8 / 4) * 8
            + 8
            + 8
    ];

    DdsIO::save_dds_file(
        &src_dds,
        64,
        64,
        6,
        DxgiFormat::BC1Unorm,
        false,
        false,
        &payload,
    )
    .unwrap();

    let splitter = TextureSplitter::new(TextureSplitterConfig {
        persistent_mips: 2,
        dont_split: false,
    });

    let chunks = splitter.process_dds_file(&src_dds, &src_dds).unwrap();
    assert!(
        chunks.len() >= 2,
        "Splitter must generate multiple streamable chunks"
    );

    for chunk in &chunks {
        assert!(chunk.exists(), "Chunk file {:?} must exist on disk", chunk);
    }

    let _ = fs::remove_dir_all(temp_dir);
}
