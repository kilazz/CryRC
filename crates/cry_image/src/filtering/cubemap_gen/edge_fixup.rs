// Copyright 2001-2026 Crytek GmbH / Crytek Group. All rights reserved.
// Complete 12-Edge & Corner Cubemap Seamless Fixup

use super::image_surface::ImageSurface;

pub struct EdgeFixup;

impl EdgeFixup {
    /// Smoothes boundary edge pixels and corners across all 6 cube faces to eliminate seam filtering artifacts.
    ///
    /// Face layout:
    /// - Face 0: +X
    /// - Face 1: -X
    /// - Face 2: +Y
    /// - Face 3: -Y
    /// - Face 4: +Z
    /// - Face 5: -Z
    pub fn fixup_cube_edges(surfaces: &mut [ImageSurface; 6], fixup_width: usize) {
        let size = surfaces[0].width;
        let channels = surfaces[0].channels;
        if fixup_width == 0 || size < 2 {
            return;
        }

        let blend_width = fixup_width.min(size / 2);

        for w in 0..blend_width {
            let factor = 0.5 * (1.0 - (w as f32 / blend_width as f32));

            // =========================================================================
            // 1. Horizontal Border Edges (Meeting around Y-axis)
            // =========================================================================
            for i in 0..size {
                for c in 0..channels {
                    // +Z (4) right edge meets +X (0) left edge
                    let idx_4r = (i * size + (size - 1 - w)) * channels + c;
                    let idx_0l = (i * size + w) * channels + c;
                    let (v4, v0) = Self::compute_blend(
                        surfaces[4].data[idx_4r],
                        surfaces[0].data[idx_0l],
                        factor,
                    );
                    surfaces[4].data[idx_4r] = v4;
                    surfaces[0].data[idx_0l] = v0;

                    // +X (0) right edge meets -Z (5) left edge
                    let idx_0r = (i * size + (size - 1 - w)) * channels + c;
                    let idx_5l = (i * size + w) * channels + c;
                    let (v0, v5) = Self::compute_blend(
                        surfaces[0].data[idx_0r],
                        surfaces[5].data[idx_5l],
                        factor,
                    );
                    surfaces[0].data[idx_0r] = v0;
                    surfaces[5].data[idx_5l] = v5;

                    // -Z (5) right edge meets -X (1) left edge
                    let idx_5r = (i * size + (size - 1 - w)) * channels + c;
                    let idx_1l = (i * size + w) * channels + c;
                    let (v5, v1) = Self::compute_blend(
                        surfaces[5].data[idx_5r],
                        surfaces[1].data[idx_1l],
                        factor,
                    );
                    surfaces[5].data[idx_5r] = v5;
                    surfaces[1].data[idx_1l] = v1;

                    // -X (1) right edge meets +Z (4) left edge
                    let idx_1r = (i * size + (size - 1 - w)) * channels + c;
                    let idx_4l = (i * size + w) * channels + c;
                    let (v1, v4) = Self::compute_blend(
                        surfaces[1].data[idx_1r],
                        surfaces[4].data[idx_4l],
                        factor,
                    );
                    surfaces[1].data[idx_1r] = v1;
                    surfaces[4].data[idx_4l] = v4;
                }
            }

            // =========================================================================
            // 2. Top Border Edges (Meeting Face 2: +Y)
            // =========================================================================
            for i in 0..size {
                for c in 0..channels {
                    // +Z (4) top edge meets +Y (2) bottom edge
                    let idx_4t = (w * size + i) * channels + c;
                    let idx_2b = ((size - 1 - w) * size + i) * channels + c;
                    let (v4, v2) = Self::compute_blend(
                        surfaces[4].data[idx_4t],
                        surfaces[2].data[idx_2b],
                        factor,
                    );
                    surfaces[4].data[idx_4t] = v4;
                    surfaces[2].data[idx_2b] = v2;

                    // -Z (5) top edge meets +Y (2) top edge (inverted X)
                    let idx_5t = (w * size + i) * channels + c;
                    let idx_2t = (w * size + (size - 1 - i)) * channels + c;
                    let (v5, v2) = Self::compute_blend(
                        surfaces[5].data[idx_5t],
                        surfaces[2].data[idx_2t],
                        factor,
                    );
                    surfaces[5].data[idx_5t] = v5;
                    surfaces[2].data[idx_2t] = v2;

                    // +X (0) top edge meets +Y (2) right edge (rotated)
                    let idx_0t = (w * size + i) * channels + c;
                    let idx_2r = (i * size + (size - 1 - w)) * channels + c;
                    let (v0, v2) = Self::compute_blend(
                        surfaces[0].data[idx_0t],
                        surfaces[2].data[idx_2r],
                        factor,
                    );
                    surfaces[0].data[idx_0t] = v0;
                    surfaces[2].data[idx_2r] = v2;

                    // -X (1) top edge meets +Y (2) left edge (rotated)
                    let idx_1t = (w * size + i) * channels + c;
                    let idx_2l = ((size - 1 - i) * size + w) * channels + c;
                    let (v1, v2) = Self::compute_blend(
                        surfaces[1].data[idx_1t],
                        surfaces[2].data[idx_2l],
                        factor,
                    );
                    surfaces[1].data[idx_1t] = v1;
                    surfaces[2].data[idx_2l] = v2;
                }
            }

            // =========================================================================
            // 3. Bottom Border Edges (Meeting Face 3: -Y)
            // =========================================================================
            for i in 0..size {
                for c in 0..channels {
                    // +Z (4) bottom edge meets -Y (3) top edge
                    let idx_4b = ((size - 1 - w) * size + i) * channels + c;
                    let idx_3t = (w * size + i) * channels + c;
                    let (v4, v3) = Self::compute_blend(
                        surfaces[4].data[idx_4b],
                        surfaces[3].data[idx_3t],
                        factor,
                    );
                    surfaces[4].data[idx_4b] = v4;
                    surfaces[3].data[idx_3t] = v3;

                    // -Z (5) bottom edge meets -Y (3) bottom edge (inverted X)
                    let idx_5b = ((size - 1 - w) * size + i) * channels + c;
                    let idx_3b = ((size - 1 - w) * size + (size - 1 - i)) * channels + c;
                    let (v5, v3) = Self::compute_blend(
                        surfaces[5].data[idx_5b],
                        surfaces[3].data[idx_3b],
                        factor,
                    );
                    surfaces[5].data[idx_5b] = v5;
                    surfaces[3].data[idx_3b] = v3;

                    // +X (0) bottom edge meets -Y (3) right edge (rotated)
                    let idx_0b = ((size - 1 - w) * size + i) * channels + c;
                    let idx_3r = ((size - 1 - i) * size + (size - 1 - w)) * channels + c;
                    let (v0, v3) = Self::compute_blend(
                        surfaces[0].data[idx_0b],
                        surfaces[3].data[idx_3r],
                        factor,
                    );
                    surfaces[0].data[idx_0b] = v0;
                    surfaces[3].data[idx_3r] = v3;

                    // -X (1) bottom edge meets -Y (3) left edge (rotated)
                    let idx_1b = ((size - 1 - w) * size + i) * channels + c;
                    let idx_3l = (i * size + w) * channels + c;
                    let (v1, v3) = Self::compute_blend(
                        surfaces[1].data[idx_1b],
                        surfaces[3].data[idx_3l],
                        factor,
                    );
                    surfaces[1].data[idx_1b] = v1;
                    surfaces[3].data[idx_3l] = v3;
                }
            }
        }

        // =========================================================================
        // 4. Corner Fixup (Average 3-way adjacent corner vertices)
        // =========================================================================
        Self::fixup_corners(surfaces, size, channels);
    }

    #[inline(always)]
    fn compute_blend(val_a: f32, val_b: f32, factor: f32) -> (f32, f32) {
        let avg = (val_a + val_b) * 0.5;
        (
            val_a * (1.0 - factor) + avg * factor,
            val_b * (1.0 - factor) + avg * factor,
        )
    }

    fn fixup_corners(surfaces: &mut [ImageSurface; 6], size: usize, channels: usize) {
        let last = size - 1;
        let off = |x: usize, y: usize, c: usize| -> usize { (y * size + x) * channels + c };

        for c in 0..channels {
            // Corner 1: (+X top-left, +Y bottom-right, +Z top-right)
            let avg1 = (surfaces[0].data[off(0, 0, c)]
                + surfaces[2].data[off(last, last, c)]
                + surfaces[4].data[off(last, 0, c)])
                / 3.0;
            surfaces[0].data[off(0, 0, c)] = avg1;
            surfaces[2].data[off(last, last, c)] = avg1;
            surfaces[4].data[off(last, 0, c)] = avg1;

            // Corner 2: (+X top-right, +Y top-right, -Z top-left)
            let avg2 = (surfaces[0].data[off(last, 0, c)]
                + surfaces[2].data[off(last, 0, c)]
                + surfaces[5].data[off(0, 0, c)])
                / 3.0;
            surfaces[0].data[off(last, 0, c)] = avg2;
            surfaces[2].data[off(last, 0, c)] = avg2;
            surfaces[5].data[off(0, 0, c)] = avg2;

            // Corner 3: (-X top-left, +Y top-left, -Z top-right)
            let avg3 = (surfaces[1].data[off(0, 0, c)]
                + surfaces[2].data[off(0, 0, c)]
                + surfaces[5].data[off(last, 0, c)])
                / 3.0;
            surfaces[1].data[off(0, 0, c)] = avg3;
            surfaces[2].data[off(0, 0, c)] = avg3;
            surfaces[5].data[off(last, 0, c)] = avg3;

            // Corner 4: (-X top-right, +Y bottom-left, +Z top-left)
            let avg4 = (surfaces[1].data[off(last, 0, c)]
                + surfaces[2].data[off(0, last, c)]
                + surfaces[4].data[off(0, 0, c)])
                / 3.0;
            surfaces[1].data[off(last, 0, c)] = avg4;
            surfaces[2].data[off(0, last, c)] = avg4;
            surfaces[4].data[off(0, 0, c)] = avg4;

            // Corner 5: (+X bottom-left, -Y top-right, +Z bottom-right)
            let avg5 = (surfaces[0].data[off(0, last, c)]
                + surfaces[3].data[off(last, 0, c)]
                + surfaces[4].data[off(last, last, c)])
                / 3.0;
            surfaces[0].data[off(0, last, c)] = avg5;
            surfaces[3].data[off(last, 0, c)] = avg5;
            surfaces[4].data[off(last, last, c)] = avg5;

            // Corner 6: (+X bottom-right, -Y bottom-right, -Z bottom-left)
            let avg6 = (surfaces[0].data[off(last, last, c)]
                + surfaces[3].data[off(last, last, c)]
                + surfaces[5].data[off(0, last, c)])
                / 3.0;
            surfaces[0].data[off(last, last, c)] = avg6;
            surfaces[3].data[off(last, last, c)] = avg6;
            surfaces[5].data[off(0, last, c)] = avg6;

            // Corner 7: (-X bottom-left, -Y bottom-left, -Z bottom-right)
            let avg7 = (surfaces[1].data[off(0, last, c)]
                + surfaces[3].data[off(0, last, c)]
                + surfaces[5].data[off(last, last, c)])
                / 3.0;
            surfaces[1].data[off(0, last, c)] = avg7;
            surfaces[3].data[off(0, last, c)] = avg7;
            surfaces[5].data[off(last, last, c)] = avg7;

            // Corner 8: (-X bottom-right, -Y top-left, +Z bottom-left)
            let avg8 = (surfaces[1].data[off(last, last, c)]
                + surfaces[3].data[off(0, 0, c)]
                + surfaces[4].data[off(0, last, c)])
                / 3.0;
            surfaces[1].data[off(last, last, c)] = avg8;
            surfaces[3].data[off(0, 0, c)] = avg8;
            surfaces[4].data[off(0, last, c)] = avg8;
        }
    }
}
