use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use cry_image::*;
use std::hint::black_box;

fn generate_bench_image(width: usize, height: usize) -> Vec<u8> {
    let mut data = vec![0u8; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 4;
            data[idx] = (x ^ y) as u8;
            data[idx + 1] = ((x * 255) / width) as u8;
            data[idx + 2] = ((y * 255) / height) as u8;
            data[idx + 3] = 255;
        }
    }
    data
}

fn bench_bc_compression(c: &mut Criterion) {
    let width = 256;
    let height = 256;
    let pixels = (width * height) as u64;
    let rgba = generate_bench_image(width, height);

    let mut group = c.benchmark_group("texture_compression");
    group.throughput(Throughput::Elements(pixels));

    // BC1
    let mut opts_bc1 = CompressionOptions::bc1();
    opts_bc1.strategy = FitStrategy::FastRange;
    group.bench_with_input(BenchmarkId::new("BC1", "FastRange"), &rgba, |b, data| {
        b.iter(|| {
            let mut out = [0u8; 8];
            for chunk in data.chunks_exact(64) {
                let px: [[u8; 4]; 16] = core::array::from_fn(|i| {
                    [
                        chunk[i * 4],
                        chunk[i * 4 + 1],
                        chunk[i * 4 + 2],
                        chunk[i * 4 + 3],
                    ]
                });
                compress_bc1_block(black_box(&px), 0xFFFF, opts_bc1, black_box(&mut out));
            }
        });
    });

    opts_bc1.strategy = FitStrategy::Cluster(8);
    group.bench_with_input(BenchmarkId::new("BC1", "ClusterFit_8"), &rgba, |b, data| {
        b.iter(|| {
            let mut out = [0u8; 8];
            for chunk in data.chunks_exact(64) {
                let px: [[u8; 4]; 16] = core::array::from_fn(|i| {
                    [
                        chunk[i * 4],
                        chunk[i * 4 + 1],
                        chunk[i * 4 + 2],
                        chunk[i * 4 + 3],
                    ]
                });
                compress_bc1_block(black_box(&px), 0xFFFF, opts_bc1, black_box(&mut out));
            }
        });
    });

    // BC7
    for quality in [
        QualityLevel::Ultrafast,
        QualityLevel::Fast,
        QualityLevel::Normal,
    ] {
        let mut opts_bc7 = CompressionOptions::bc7();
        opts_bc7.quality = quality;
        let q_name = format!("{:?}", quality);

        group.bench_with_input(BenchmarkId::new("BC7", &q_name), &rgba, |b, data| {
            b.iter(|| {
                let mut out = [0u8; 16];
                for chunk in data.chunks_exact(64) {
                    let px: [[u8; 4]; 16] = core::array::from_fn(|i| {
                        [
                            chunk[i * 4],
                            chunk[i * 4 + 1],
                            chunk[i * 4 + 2],
                            chunk[i * 4 + 3],
                        ]
                    });
                    compress_bc7_block(
                        black_box(&px),
                        0xFFFF,
                        0,
                        &Vec4::splat(1.0),
                        quality,
                        black_box(&mut out),
                    );
                }
            });
        });
    }

    // SSIM Metric
    group.bench_with_input(BenchmarkId::new("Metrics", "SSIM"), &rgba, |b, data| {
        b.iter(|| compute_ssim(black_box(data), black_box(data), width, height));
    });

    group.finish();
}

criterion_group!(benches, bench_bc_compression);
criterion_main!(benches);
