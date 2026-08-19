// Copyright 2001-2026 Crytek GmbH / Crytek Group. All rights reserved.
// 3D Color Grading LUT Chart (16x16x16, 256x16 2D Texture Representation)

use super::color_chart_base::ColorChart;

pub const PS_RED: usize = 16;
pub const PS_GREEN: usize = 16;
pub const PS_BLUE: usize = 16;
pub const NUM_COLORS: usize = PS_RED * PS_GREEN * PS_BLUE; // 4096 entries

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LutColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone)]
pub struct Lut3DColorChart {
    pub mapping: Vec<LutColor>,
}

impl Default for Lut3DColorChart {
    fn default() -> Self {
        let mut chart = Self {
            mapping: Vec::with_capacity(NUM_COLORS),
        };
        chart.generate_default();
        chart
    }
}

impl ColorChart for Lut3DColorChart {
    /// Generates the canonical neutral/identity 3D LUT mapping.
    fn generate_default(&mut self) {
        self.mapping.clear();
        for b in 0..PS_BLUE {
            for g in 0..PS_GREEN {
                for r in 0..PS_RED {
                    let cr = ((255 * r) / (PS_RED - 1).max(1)) as u8;
                    let cg = ((255 * g) / (PS_GREEN - 1).max(1)) as u8;
                    let cb = ((255 * b) / (PS_BLUE - 1).max(1)) as u8;
                    self.mapping.push(LutColor {
                        r: cr,
                        g: cg,
                        b: cb,
                    });
                }
            }
        }
    }

    /// Reads color chart mapping from a 256x16 BGRA source texture.
    fn generate_from_input(
        &mut self,
        width: usize,
        height: usize,
        bgra: &[u8],
        pitch: usize,
    ) -> Result<(), String> {
        let lut_w = PS_RED * PS_BLUE; // 256
        let lut_h = PS_GREEN; // 16

        if width < lut_w || height < lut_h {
            self.generate_default();
            return Ok(());
        }

        self.mapping.clear();
        for b in 0..PS_BLUE {
            for g in 0..PS_GREEN {
                for r in 0..PS_RED {
                    let src_x = b * PS_RED + r;
                    let src_y = g;
                    let off = src_y * pitch + src_x * 4;

                    if off + 3 < bgra.len() {
                        self.mapping.push(LutColor {
                            b: bgra[off],
                            g: bgra[off + 1],
                            r: bgra[off + 2],
                        });
                    } else {
                        let cr = ((255 * r) / (PS_RED - 1).max(1)) as u8;
                        let cg = ((255 * g) / (PS_GREEN - 1).max(1)) as u8;
                        let cb = ((255 * b) / (PS_BLUE - 1).max(1)) as u8;
                        self.mapping.push(LutColor {
                            r: cr,
                            g: cg,
                            b: cb,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Generates a 256x16 32-bit BGRA texture image suitable for saving as a CryEngine color chart texture.
    fn generate_chart_image(&self) -> Option<(usize, usize, Vec<u8>)> {
        let lut_w = PS_RED * PS_BLUE;
        let lut_h = PS_GREEN;
        let mut pixels = vec![0u8; lut_w * lut_h * 4];

        let mut src_idx = 0;
        for b in 0..PS_BLUE {
            for g in 0..PS_GREEN {
                for r in 0..PS_RED {
                    let c = &self.mapping[src_idx];
                    let dst_x = b * PS_RED + r;
                    let dst_y = g;
                    let dst_off = (dst_y * lut_w + dst_x) * 4;

                    pixels[dst_off] = c.b;
                    pixels[dst_off + 1] = c.g;
                    pixels[dst_off + 2] = c.r;
                    pixels[dst_off + 3] = 0xFF;
                    src_idx += 1;
                }
            }
        }
        Some((lut_w, lut_h, pixels))
    }
}
