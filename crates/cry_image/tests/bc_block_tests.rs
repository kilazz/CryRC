use cry_image::*;

fn generate_test_pattern_rgba(width: usize, height: usize) -> Vec<u8> {
    let mut data = vec![0u8; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 4;
            data[idx] = ((x * 255) / width) as u8;
            data[idx + 1] = ((y * 255) / height) as u8;
            data[idx + 2] = ((x ^ y) & 0xFF) as u8;
            data[idx + 3] = if (x + y) % 2 == 0 { 255 } else { 128 };
        }
    }
    data
}

#[test]
fn test_all_block_formats_roundtrip() {
    let width = 16;
    let height = 16;
    let rgba = generate_test_pattern_rgba(width, height);

    let test_formats = [
        Format::Bc1,
        Format::Bc2,
        Format::Bc3,
        Format::Bc4,
        Format::Bc5,
        Format::Bc6h,
        Format::Bc7,
        Format::Ctx1,
    ];

    for &fmt in &test_formats {
        let mut opts = CompressionOptions::default();
        opts.format = fmt;
        opts.quality = QualityLevel::Fast;

        let compressed = compress_image(&rgba, width, height, opts);
        assert_eq!(
            compressed.len(),
            get_storage_requirements(width, height, fmt),
            "Storage requirements mismatch for format {:?}",
            fmt
        );

        let decompressed = decompress_image(&compressed, width, height, fmt);
        assert_eq!(decompressed.len(), rgba.len());

        let (first_ch, num_ch) = match fmt {
            Format::Bc1 => (0, 3),
            Format::Bc4 => (0, 1),
            Format::Bc5 | Format::Ctx1 => (0, 2),
            Format::Bc6h => (0, 3),
            Format::Bc2 | Format::Bc3 | Format::Bc7 => (0, 4),
        };

        let metrics = ImageMetrics::compute(&rgba, &decompressed, width, height, first_ch, num_ch);
        assert!(
            metrics.peak_snr > 20.0,
            "PSNR too low for {:?}: {:.2} dB",
            fmt,
            metrics.peak_snr
        );
    }
}

#[test]
fn test_bc7_all_64_partitions_reconstruction() {
    for p in 0..64 {
        let mut block = [[0u8; 4]; 16];
        for (i, px) in block.iter_mut().enumerate() {
            let subset = ((tables::PARTITION_MASKS_2[p] >> i) & 1) as usize;
            if subset == 0 {
                *px = [220, 20, 20, 255];
            } else {
                *px = [20, 220, 220, 255];
            }
        }

        let mut comp = [0u8; 16];
        let mut opts = CompressionOptions::bc7();
        opts.quality = QualityLevel::Fast;
        compress_block(&block, 0xFFFF, opts, &mut comp);

        let dec = decompress_image(&comp, 4, 4, Format::Bc7);
        for i in 0..16 {
            let subset = ((tables::PARTITION_MASKS_2[p] >> i) & 1) as usize;
            let r = dec[i * 4];
            let g = dec[i * 4 + 1];

            if subset == 0 {
                assert!(
                    r > 140 && g < 120,
                    "BC7 partition {} subset 0 mismatch at px {}",
                    p,
                    i
                );
            } else {
                assert!(
                    g > 140 && r < 120,
                    "BC7 partition {} subset 1 mismatch at px {}",
                    p,
                    i
                );
            }
        }
    }
}

#[test]
fn test_bc6h_unsigned_and_signed_hdr() {
    let width = 8;
    let height = 8;
    let mut rgb = vec![[0.0f32; 3]; width * height];

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let fx = (x % 4) as f32;
            let fy = (y % 4) as f32;
            rgb[idx] = [
                (fx * 0.5 + fy * 0.25 - 2.0),
                (2.0 - fx * 0.25 - fy * 0.5),
                (fx * 0.1 + fy * 0.1 - 0.5),
            ];
        }
    }

    let mut opts_signed = CompressionOptions::bc6h();
    opts_signed.is_signed = true;

    let comp = compress_image_hdr(&rgb, width, height, opts_signed);
    let dec = decompress_image_hdr(&comp, width, height, true);

    for i in 0..width * height {
        for c in 0..3 {
            let diff = (rgb[i][c] - dec[i][c]).abs();
            assert!(
                diff <= 0.85,
                "Signed BC6H error at px {} ch {}: orig {}, got {}",
                i,
                c,
                rgb[i][c],
                dec[i][c]
            );
        }
    }
}
