// Copyright 2001-2026 Crytek GmbH / Crytek Group. All rights reserved.
// BC1 / DXT1 Block Compressor with Accurate 4-Color Opaque & 3-Color 1-Bit Alpha Modes

use crate::flags::{ColorMetric, CompressionOptions, FitStrategy};
use crate::math::normal::{
    DEVIANCE_BASE, add_deviance, codebook_3_normal, codebook_4_normal, min_deviance_3,
    min_deviance_4, snorm_to_unorm, unorm_to_snorm,
};
use crate::math::pca::{
    compute_weighted_covariance3, estimate_principle_component, get_principle_projection_vec3,
};
use crate::math::vector::{Vec3, Vec4};
use crate::quantize::const_grid::Quantizer3;

type Q565 = Quantizer3<5, 6, 5>;

// =============================================================================
// Conversions & Raw Block I/O
// =============================================================================

#[inline(always)]
pub fn float_to_565(color: &Vec3) -> u16 {
    let rgb = Q565::quantize_to_int(color);
    let r = rgb[0] & 0x1F;
    let g = rgb[1] & 0x3F;
    let b = rgb[2] & 0x1F;
    ((r << 11) | (g << 5) | b) as u16
}

#[inline(always)]
pub fn unpack_565(value: u16) -> [u8; 3] {
    let r = ((value >> 11) & 0x1F) as u8;
    let g = ((value >> 5) & 0x3F) as u8;
    let b = (value & 0x1F) as u8;
    [
        (r << 3) | (r >> 2),
        (g << 2) | (g >> 4),
        (b << 3) | (b >> 2),
    ]
}

pub fn write_color_block_3(start: &Vec3, end: &Vec3, indices: &[u8; 16], block: &mut [u8; 8]) {
    let mut a = float_to_565(start);
    let mut b = float_to_565(end);
    let mut remapped = *indices;

    if a > b {
        std::mem::swap(&mut a, &mut b);
        for idx in &mut remapped {
            if *idx == 0 {
                *idx = 1;
            } else if *idx == 1 {
                *idx = 0;
            }
        }
    }
    write_block_raw(a, b, &remapped, block);
}

pub fn write_color_block_4(start: &Vec3, end: &Vec3, indices: &[u8; 16], block: &mut [u8; 8]) {
    let mut a = float_to_565(start);
    let mut b = float_to_565(end);
    let mut remapped = *indices;

    if a < b {
        std::mem::swap(&mut a, &mut b);
        for idx in &mut remapped {
            *idx ^= 1;
        }
    } else if a == b {
        remapped.fill(0);
    }
    write_block_raw(a, b, &remapped, block);
}

#[inline(always)]
fn write_block_raw(a: u16, b: u16, indices: &[u8; 16], block: &mut [u8; 8]) {
    block[0] = (a & 0xFF) as u8;
    block[1] = (a >> 8) as u8;
    block[2] = (b & 0xFF) as u8;
    block[3] = (b >> 8) as u8;

    for i in 0..4 {
        let offset = i * 4;
        block[4 + i] = (indices[offset] & 0x03)
            | ((indices[offset + 1] & 0x03) << 2)
            | ((indices[offset + 2] & 0x03) << 4)
            | ((indices[offset + 3] & 0x03) << 6);
    }
}

pub fn read_color_block_bc1(block: &[u8; 8], out_rgb: &mut [[u8; 3]; 16]) {
    let mut rgba = [[0u8; 4]; 16];
    read_color_block_bc1_rgba(block, &mut rgba);
    for i in 0..16 {
        out_rgb[i] = [rgba[i][0], rgba[i][1], rgba[i][2]];
    }
}

pub fn read_color_block_bc1_rgba(block: &[u8; 8], out_rgba: &mut [[u8; 4]; 16]) {
    let a = (block[0] as u16) | ((block[1] as u16) << 8);
    let b = (block[2] as u16) | ((block[3] as u16) << 8);

    let c0 = unpack_565(a);
    let c1 = unpack_565(b);

    let mut palette = [[0u8; 4]; 4];
    palette[0] = [c0[0], c0[1], c0[2], 255];
    palette[1] = [c1[0], c1[1], c1[2], 255];

    if a > b {
        for i in 0..3 {
            palette[2][i] = ((2 * c0[i] as u16 + c1[i] as u16) / 3) as u8;
            palette[3][i] = ((c0[i] as u16 + 2 * c1[i] as u16) / 3) as u8;
        }
        palette[2][3] = 255;
        palette[3][3] = 255;
    } else {
        for i in 0..3 {
            palette[2][i] = ((c0[i] as u16 + c1[i] as u16) / 2) as u8;
        }
        palette[2][3] = 255;
        palette[3] = [0, 0, 0, 0]; // 1-bit alpha transparent
    }

    for i in 0..4 {
        let byte = block[4 + i];
        for j in 0..4 {
            let idx = (byte >> (j * 2)) & 0x03;
            out_rgba[i * 4 + j] = palette[idx as usize];
        }
    }
}

// =============================================================================
// Color Set & Fitting Context
// =============================================================================

#[derive(Debug, Clone)]
pub struct ColorSet {
    pub count: usize,
    pub points: [Vec3; 16],
    pub weights: [f32; 16],
    pub remap: [Option<u8>; 16],
    pub transparent: bool,
}

impl ColorSet {
    pub fn new(
        rgba: &[[u8; 4]; 16],
        mask: u16,
        weight_by_alpha: bool,
        is_1bit_alpha: bool,
    ) -> Self {
        let mut count = 0;
        let mut points = [Vec3::splat(0.0); 16];
        let mut weights = [0.0; 16];
        let mut remap = [None; 16];
        let mut transparent = false;

        for (i, remap_slot) in remap.iter_mut().enumerate() {
            if (mask & (1 << i)) == 0 {
                *remap_slot = None;
                continue;
            }

            let r = rgba[i][0];
            let g = rgba[i][1];
            let b = rgba[i][2];
            let a = rgba[i][3];

            // 1-Bit Alpha Cutout ONLY triggers if format explicitly allows 1-bit alpha (BC1a)
            if is_1bit_alpha && a < 128 {
                transparent = true;
                *remap_slot = None;
                continue;
            }

            let mut weight = 1.0;
            if weight_by_alpha && !is_1bit_alpha {
                weight = (a as f32 + 1.0) / 256.0;
            }

            let pt = Vec3::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
            let mut found = None;

            for (j, &existing) in points[..count].iter().enumerate() {
                if (existing - pt).length_squared() < 1e-6 {
                    found = Some(j);
                    break;
                }
            }

            match found {
                Some(idx) => {
                    *remap_slot = Some(idx as u8);
                    weights[idx] += weight;
                }
                None => {
                    *remap_slot = Some(count as u8);
                    points[count] = pt;
                    weights[count] = weight;
                    count += 1;
                }
            }
        }

        for w in &mut weights[..count] {
            *w = w.sqrt();
        }

        Self {
            count,
            points,
            weights,
            remap,
            transparent,
        }
    }
}

pub struct ColorFitContext<'a> {
    pub color_set: &'a ColorSet,
    pub metric: Vec3,
    pub best_error: f32,
    pub start: Vec3,
    pub end: Vec3,
    pub indices: [u8; 16],
}

impl<'a> ColorFitContext<'a> {
    pub fn new(color_set: &'a ColorSet, metric_type: ColorMetric) -> Self {
        let mut metric = metric_type.vector();
        if color_set.transparent {
            metric *= 0.5;
        }
        Self {
            color_set,
            metric,
            best_error: f32::MAX,
            start: Vec3::splat(0.0),
            end: Vec3::splat(0.0),
            indices: [0; 16],
        }
    }

    #[inline(always)]
    pub fn try_update_result(&mut self, error: f32, start: Vec3, end: Vec3, indices: [u8; 16]) {
        if error < self.best_error {
            self.best_error = error;
            self.start = start;
            self.end = end;
            self.indices = indices;
        }
    }

    #[inline(always)]
    pub fn remap_indices(&self, closest: &[u8]) -> [u8; 16] {
        let mut result = [0; 16];
        for (i, slot) in result.iter_mut().enumerate() {
            *slot = match self.color_set.remap[i] {
                None => 3,
                Some(uidx) => closest[uidx as usize],
            };
        }
        result
    }
}

fn compress_single_fit(context: &mut ColorFitContext, block: &mut [u8; 8]) {
    let p = context.color_set.points[0];
    let start = p;
    let end = p;

    if context.color_set.transparent {
        let indices = [0u8; 16];
        let remapped = context.remap_indices(&indices);
        write_color_block_3(&start, &end, &remapped, block);
    } else {
        let indices = [0u8; 16];
        let remapped = context.remap_indices(&indices);
        write_color_block_4(&start, &end, &remapped, block);
    }
}

fn compress_range_fit(context: &mut ColorFitContext, block: &mut [u8; 8]) {
    let set = context.color_set;
    let count = set.count;

    let active_points = &set.points[..count];
    let active_weights = &set.weights[..count];
    let (covariance, centroid) = compute_weighted_covariance3(active_points, active_weights);

    let principle = estimate_principle_component(&covariance);
    let (start_proj, end_proj) = project_points_clamped(&principle, &centroid, active_points);

    let start = Q565::snap_to_lattice(&start_proj);
    let end = Q565::snap_to_lattice(&end_proj);

    if context.color_set.transparent {
        let codes = [
            context.metric * start,
            context.metric * end,
            context.metric * (0.5 * start + 0.5 * end),
        ];

        let mut closest = [0u8; 16];
        let mut error = 0.0f32;

        for (i, slot) in closest[..count].iter_mut().enumerate() {
            let value = context.metric * set.points[i];
            let d0 = (value - codes[0]).length_squared();
            let d1 = (value - codes[1]).length_squared();
            let d2 = (value - codes[2]).length_squared();

            let mut min_dist = d0;
            let mut idx = 0u8;
            if d1 < min_dist {
                min_dist = d1;
                idx = 1;
            }
            if d2 < min_dist {
                min_dist = d2;
                idx = 2;
            }

            *slot = idx;
            error += min_dist * set.weights[i];
        }

        if error < context.best_error {
            let remapped_indices = context.remap_indices(&closest);
            context.try_update_result(error, start, end, remapped_indices);
            write_color_block_3(&start, &end, &remapped_indices, block);
        }
    } else {
        let codes = [
            context.metric * start,
            context.metric * end,
            context.metric * ((2.0 / 3.0) * start + (1.0 / 3.0) * end),
            context.metric * ((1.0 / 3.0) * start + (2.0 / 3.0) * end),
        ];

        let mut closest = [0u8; 16];
        let mut error = 0.0f32;

        for (i, slot) in closest[..count].iter_mut().enumerate() {
            let value = context.metric * set.points[i];
            let d0 = (value - codes[0]).length_squared();
            let d1 = (value - codes[1]).length_squared();
            let d2 = (value - codes[2]).length_squared();
            let d3 = (value - codes[3]).length_squared();

            let mut min_dist = d0;
            let mut idx = 0u8;
            if d1 < min_dist {
                min_dist = d1;
                idx = 1;
            }
            if d2 < min_dist {
                min_dist = d2;
                idx = 2;
            }
            if d3 < min_dist {
                min_dist = d3;
                idx = 3;
            }

            *slot = idx;
            error += min_dist * set.weights[i];
        }

        if error < context.best_error {
            let remapped_indices = context.remap_indices(&closest);
            context.try_update_result(error, start, end, remapped_indices);
            write_color_block_4(&start, &end, &remapped_indices, block);
        }
    }
}

fn project_points_clamped(principle: &Vec3, centroid: &Vec3, points: &[Vec3]) -> (Vec3, Vec3) {
    let length_sq = principle.dot(principle);
    if length_sq < f32::EPSILON {
        return (*centroid, *centroid);
    }

    let div = 1.0 / length_sq;
    let mut min_len = f32::MAX;
    let mut max_len = -f32::MAX;

    for p in points {
        let offset = *p - *centroid;
        let len = offset.dot(principle);
        if len < min_len {
            min_len = len;
        }
        if len > max_len {
            max_len = len;
        }
    }

    let mut start = *centroid + *principle * (min_len * div);
    let mut end = *centroid + *principle * (max_len * div);

    start = clip_to_unit_cube(start, *principle);
    end = clip_to_unit_cube(end, *principle);

    (start, end)
}

fn clip_to_unit_cube(mut point: Vec3, dir: Vec3) -> Vec3 {
    let eps = 1.0 / (255.0 * 255.0);

    if point.x < -eps && dir.x.abs() > f32::EPSILON {
        point -= dir * (point.x / dir.x);
    }
    if point.y < -eps && dir.y.abs() > f32::EPSILON {
        point -= dir * (point.y / dir.y);
    }
    if point.z < -eps && dir.z.abs() > f32::EPSILON {
        point -= dir * (point.z / dir.z);
    }

    if point.x > 1.0 + eps && dir.x.abs() > f32::EPSILON {
        point -= dir * ((point.x - 1.0) / dir.x);
    }
    if point.y > 1.0 + eps && dir.y.abs() > f32::EPSILON {
        point -= dir * ((point.y - 1.0) / dir.y);
    }
    if point.z > 1.0 + eps && dir.z.abs() > f32::EPSILON {
        point -= dir * ((point.z - 1.0) / dir.z);
    }

    point.clamp(0.0, 1.0)
}

fn compress_cluster_fit(context: &mut ColorFitContext, max_iterations: usize, block: &mut [u8; 8]) {
    if context.color_set.transparent {
        cluster_fit_3(context, max_iterations, block);
    } else {
        cluster_fit_4(context, max_iterations, block);
    }
}

fn construct_ordering(axis: &Vec3, points: &[Vec3], order: &mut [usize]) {
    let count = points.len();
    let mut dps = [0.0f32; 16];

    for (i, p) in points.iter().enumerate() {
        dps[i] = p.dot(axis);
        order[i] = i;
    }

    for i in 0..count {
        let mut j = i;
        while j > 0 && dps[j] < dps[j - 1] {
            dps.swap(j, j - 1);
            order.swap(j, j - 1);
            j -= 1;
        }
    }
}

fn cluster_fit_4(context: &mut ColorFitContext, max_iterations: usize, block: &mut [u8; 8]) {
    let set = context.color_set;
    let count = set.count;
    let metric = context.metric;
    let cmetric = metric * metric;

    let (covariance, _) = compute_weighted_covariance3(&set.points[..count], &set.weights[..count]);
    let mut axis = estimate_principle_component(&covariance);

    let mut order = [0usize; 16];
    let mut points_weights = [Vec4::splat(0.0); 16];

    let mut best_start = Vec3::splat(0.0);
    let mut best_end = Vec3::splat(0.0);
    let mut best_error = f32::MAX;

    let weight1 = Vec4::new(1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0, 1.0 / 9.0);
    let weight2 = Vec4::new(2.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0, 4.0 / 9.0);
    let two_nineths = 2.0 / 9.0;

    for _ in 0..max_iterations.clamp(1, 15) {
        construct_ordering(&axis, &set.points[..count], &mut order);

        let mut xsum_wsum = Vec4::splat(0.0);
        for i in 0..count {
            let unweighted = set.points[order[i]];
            let w = set.weights[order[i]];
            let pw = Vec4::new(unweighted.x * w, unweighted.y * w, unweighted.z * w, w);
            points_weights[i] = pw;
            xsum_wsum += pw;
        }

        let mut iteration_improved = false;
        let mut part0 = Vec4::splat(0.0);

        for i in 0..count {
            let mut part1 = Vec4::splat(0.0);
            let mut j = i;
            while j <= count {
                let mut part2 = if j == 0 {
                    points_weights[0]
                } else {
                    Vec4::splat(0.0)
                };
                let mut k = if j == 0 { 1 } else { j };

                while k <= count {
                    let part3 = xsum_wsum - part2 - part1 - part0;

                    let alphax_sum = part0 + part1 * weight2 + part2 * weight1;
                    let betax_sum = part3 + part2 * weight2 + part1 * weight1;

                    let alpha2_sum = alphax_sum.w;
                    let beta2_sum = betax_sum.w;
                    let alphabeta_sum = two_nineths * (part1.w + part2.w);

                    let denom = alpha2_sum * beta2_sum - alphabeta_sum * alphabeta_sum;
                    if denom.abs() > f32::EPSILON {
                        let factor = 1.0 / denom;
                        let a = (alphax_sum.to_vec3() * beta2_sum
                            - betax_sum.to_vec3() * alphabeta_sum)
                            * factor;
                        let b = (betax_sum.to_vec3() * alpha2_sum
                            - alphax_sum.to_vec3() * alphabeta_sum)
                            * factor;

                        let a_snap = Q565::snap_to_lattice(&a);
                        let b_snap = Q565::snap_to_lattice(&b);

                        let e1 = a_snap * a_snap * alpha2_sum + b_snap * b_snap * beta2_sum;
                        let e2 = a_snap * b_snap * alphabeta_sum - a_snap * alphax_sum.to_vec3();
                        let e3 = e2 - b_snap * betax_sum.to_vec3();
                        let e4 = e3 * 2.0 + e1;

                        let err = (e4 * cmetric).dot(&Vec3::splat(1.0));

                        if err < best_error {
                            best_error = err;
                            best_start = a_snap;
                            best_end = b_snap;
                            iteration_improved = true;
                        }
                    }

                    if k == count {
                        break;
                    }
                    part2 += points_weights[k];
                    k += 1;
                }

                if j == count {
                    break;
                }
                part1 += points_weights[j];
                j += 1;
            }

            part0 += points_weights[i];
        }

        if !iteration_improved {
            break;
        }

        axis = best_end - best_start;
        if axis.length_squared() < f32::EPSILON {
            break;
        }
    }

    if best_error < f32::MAX {
        let codes = [
            context.metric * best_start,
            context.metric * best_end,
            context.metric * ((2.0 / 3.0) * best_start + (1.0 / 3.0) * best_end),
            context.metric * ((1.0 / 3.0) * best_start + (2.0 / 3.0) * best_end),
        ];

        let mut closest = [0u8; 16];
        let mut true_error = 0.0f32;

        for (i, slot) in closest[..count].iter_mut().enumerate() {
            let value = context.metric * set.points[i];
            let d0 = (value - codes[0]).length_squared();
            let d1 = (value - codes[1]).length_squared();
            let d2 = (value - codes[2]).length_squared();
            let d3 = (value - codes[3]).length_squared();

            let mut min_dist = d0;
            let mut idx = 0u8;
            if d1 < min_dist {
                min_dist = d1;
                idx = 1;
            }
            if d2 < min_dist {
                min_dist = d2;
                idx = 2;
            }
            if d3 < min_dist {
                min_dist = d3;
                idx = 3;
            }

            *slot = idx;
            true_error += min_dist * set.weights[i];
        }

        if true_error < context.best_error {
            context.best_error = true_error;
            let remapped = context.remap_indices(&closest);
            write_color_block_4(&best_start, &best_end, &remapped, block);
        }
    }
}

fn cluster_fit_3(context: &mut ColorFitContext, max_iterations: usize, block: &mut [u8; 8]) {
    let set = context.color_set;
    let count = set.count;
    let metric = context.metric;
    let cmetric = metric * metric;

    let (covariance, _) = compute_weighted_covariance3(&set.points[..count], &set.weights[..count]);
    let mut axis = estimate_principle_component(&covariance);

    let mut order = [0usize; 16];
    let mut points_weights = [Vec4::splat(0.0); 16];

    let mut best_start = Vec3::splat(0.0);
    let mut best_end = Vec3::splat(0.0);
    let mut best_error = f32::MAX;

    let half_weight = Vec4::new(0.5, 0.5, 0.5, 0.25);

    for _ in 0..max_iterations.clamp(1, 15) {
        construct_ordering(&axis, &set.points[..count], &mut order);

        let mut xsum_wsum = Vec4::splat(0.0);
        for i in 0..count {
            let unweighted = set.points[order[i]];
            let w = set.weights[order[i]];
            let pw = Vec4::new(unweighted.x * w, unweighted.y * w, unweighted.z * w, w);
            points_weights[i] = pw;
            xsum_wsum += pw;
        }

        let mut iteration_improved = false;
        let mut part0 = Vec4::splat(0.0);

        for i in 0..count {
            let mut part1 = if i == 0 {
                points_weights[0]
            } else {
                Vec4::splat(0.0)
            };
            let mut j = if i == 0 { 1 } else { i };

            while j <= count {
                let part2 = xsum_wsum - part1 - part0;

                let alphax_sum = part0 + part1 * half_weight;
                let betax_sum = part2 + part1 * half_weight;

                let alpha2_sum = alphax_sum.w;
                let beta2_sum = betax_sum.w;
                let alphabeta_sum = 0.25 * part1.w;

                let denom = alpha2_sum * beta2_sum - alphabeta_sum * alphabeta_sum;
                if denom.abs() > f32::EPSILON {
                    let factor = 1.0 / denom;
                    let a = (alphax_sum.to_vec3() * beta2_sum
                        - betax_sum.to_vec3() * alphabeta_sum)
                        * factor;
                    let b = (betax_sum.to_vec3() * alpha2_sum
                        - alphax_sum.to_vec3() * alphabeta_sum)
                        * factor;

                    let a_snap = Q565::snap_to_lattice(&a);
                    let b_snap = Q565::snap_to_lattice(&b);

                    let e1 = a_snap * a_snap * alpha2_sum + b_snap * b_snap * beta2_sum;
                    let e2 = a_snap * b_snap * alphabeta_sum - a_snap * alphax_sum.to_vec3();
                    let e3 = e2 - b_snap * betax_sum.to_vec3();
                    let e4 = e3 * 2.0 + e1;

                    let err = (e4 * cmetric).dot(&Vec3::splat(1.0));

                    if err < best_error {
                        best_error = err;
                        best_start = a_snap;
                        best_end = b_snap;
                        iteration_improved = true;
                    }
                }

                if j == count {
                    break;
                }
                part1 += points_weights[j];
                j += 1;
            }

            part0 += points_weights[i];
        }

        if !iteration_improved {
            break;
        }

        axis = best_end - best_start;
        if axis.length_squared() < f32::EPSILON {
            break;
        }
    }

    if best_error < f32::MAX {
        let codes = [
            context.metric * best_start,
            context.metric * best_end,
            context.metric * (0.5 * best_start + 0.5 * best_end),
        ];

        let mut closest = [0u8; 16];
        let mut true_error = 0.0f32;

        for (i, slot) in closest[..count].iter_mut().enumerate() {
            let value = context.metric * set.points[i];
            let d0 = (value - codes[0]).length_squared();
            let d1 = (value - codes[1]).length_squared();
            let d2 = (value - codes[2]).length_squared();

            let mut min_dist = d0;
            let mut idx = 0u8;
            if d1 < min_dist {
                min_dist = d1;
                idx = 1;
            }
            if d2 < min_dist {
                min_dist = d2;
                idx = 2;
            }

            *slot = idx;
            true_error += min_dist * set.weights[i];
        }

        if true_error < context.best_error {
            context.best_error = true_error;
            let remapped = context.remap_indices(&closest);
            write_color_block_3(&best_start, &best_end, &remapped, block);
        }
    }
}

fn compress_normal_fit(context: &mut ColorFitContext, block: &mut [u8; 8]) {
    if context.color_set.transparent {
        compress_normal_fit_3(context, block);
    } else {
        compress_normal_fit_4(context, block);
    }
}

fn compress_normal_fit_3(context: &mut ColorFitContext, block: &mut [u8; 8]) {
    let set = context.color_set;
    let count = set.count;

    let (cov, centroid) = compute_weighted_covariance3(&set.points[..count], &set.weights[..count]);
    let principle = estimate_principle_component(&cov);
    let (s_proj, e_proj) =
        get_principle_projection_vec3(&principle, &centroid, &set.points[..count]);

    let mut start = Q565::snap_to_lattice(&s_proj);
    let mut end = Q565::snap_to_lattice(&e_proj);

    for _ in 0..4 {
        let codes = codebook_3_normal(&start, &end);
        let mut means = [Vec3::splat(0.0); 3];
        let mut mean_weights = [0.0f32; 3];

        for i in 0..count {
            let normal = unorm_to_snorm(set.points[i]).normalize();
            let (_, idx) = min_deviance_3(&normal, &codes);
            means[idx] += normal * set.weights[i];
            mean_weights[idx] += set.weights[i];
        }

        if mean_weights[0] > f32::EPSILON {
            start = Q565::snap_to_lattice(&snorm_to_unorm(means[0].normalize()));
        }
        if mean_weights[1] > f32::EPSILON {
            end = Q565::snap_to_lattice(&snorm_to_unorm(means[1].normalize()));
        }
    }

    let codes = codebook_3_normal(&start, &end);
    let mut closest = [0u8; 16];
    let mut error = DEVIANCE_BASE;

    for (i, slot) in closest[..count].iter_mut().enumerate() {
        let normal = unorm_to_snorm(set.points[i]).normalize();
        let (max_dot, idx) = min_deviance_3(&normal, &codes);
        add_deviance(max_dot, &mut error, set.weights[i]);
        *slot = idx as u8;
    }

    if error < context.best_error {
        let remapped = context.remap_indices(&closest);
        context.try_update_result(error, start, end, remapped);
        write_color_block_3(&start, &end, &remapped, block);
    }
}

fn compress_normal_fit_4(context: &mut ColorFitContext, block: &mut [u8; 8]) {
    let set = context.color_set;
    let count = set.count;

    let (cov, centroid) = compute_weighted_covariance3(&set.points[..count], &set.weights[..count]);
    let principle = estimate_principle_component(&cov);
    let (s_proj, e_proj) =
        get_principle_projection_vec3(&principle, &centroid, &set.points[..count]);

    let mut start = Q565::snap_to_lattice(&s_proj);
    let mut end = Q565::snap_to_lattice(&e_proj);

    for _ in 0..4 {
        let codes = codebook_4_normal(&start, &end);
        let mut means = [Vec3::splat(0.0); 4];
        let mut mean_weights = [0.0f32; 4];

        for i in 0..count {
            let normal = unorm_to_snorm(set.points[i]).normalize();
            let (_, idx) = min_deviance_4(&normal, &codes);
            means[idx] += normal * set.weights[i];
            mean_weights[idx] += set.weights[i];
        }

        if mean_weights[0] > f32::EPSILON {
            start = Q565::snap_to_lattice(&snorm_to_unorm(means[0].normalize()));
        }
        if mean_weights[1] > f32::EPSILON {
            end = Q565::snap_to_lattice(&snorm_to_unorm(means[1].normalize()));
        }
    }

    let codes = codebook_4_normal(&start, &end);
    let mut closest = [0u8; 16];
    let mut error = DEVIANCE_BASE;

    for (i, slot) in closest[..count].iter_mut().enumerate() {
        let normal = unorm_to_snorm(set.points[i]).normalize();
        let (max_dot, idx) = min_deviance_4(&normal, &codes);
        add_deviance(max_dot, &mut error, set.weights[i]);
        *slot = idx as u8;
    }

    if error < context.best_error {
        let remapped = context.remap_indices(&closest);
        context.try_update_result(error, start, end, remapped);
        write_color_block_4(&start, &end, &remapped, block);
    }
}

// =============================================================================
// Top-Level BC1 Block Compression Entry Point
// =============================================================================

pub fn compress_bc1_block(
    pixels: &[[u8; 4]; 16],
    mask: u16,
    options: CompressionOptions,
    block: &mut [u8; 8],
) {
    let is_1bit_alpha = options.is_1bit_alpha;
    let color_set = ColorSet::new(pixels, mask, options.weight_by_alpha, is_1bit_alpha);
    let mut context = ColorFitContext::new(&color_set, options.metric);

    if color_set.count == 0 {
        // All pixels are 1-bit alpha transparent.
        // In DXT1 3-color mode: color0 = 0, color1 = 0 (c0 <= c1), all indices = 3 (11b = transparent).
        block[0] = 0;
        block[1] = 0;
        block[2] = 0;
        block[3] = 0;
        block[4] = 0xFF;
        block[5] = 0xFF;
        block[6] = 0xFF;
        block[7] = 0xFF;
        return;
    }

    if color_set.count == 1 {
        compress_single_fit(&mut context, block);
    } else if options.is_normal_map {
        compress_normal_fit(&mut context, block);
    } else {
        match options.strategy {
            FitStrategy::FastRange => {
                compress_range_fit(&mut context, block);
            }
            FitStrategy::Cluster(iterations) => {
                compress_range_fit(&mut context, block);
                compress_cluster_fit(&mut context, iterations, block);
            }
        }
    }
}
