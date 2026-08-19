use super::encode::{
    encode_bc7_block_mode6, expand_quantized, fix_bc7_anchor_indices, quantize_endpoint,
    write_bc7_block,
};
use super::tables::{
    BC7_MODE_INFO, BC7_TABLES, BC7E_2SUBSET_CHECKERBOARD_PARTITION_INDEX, PB_WEIGHT, PR_WEIGHT,
};
use crate::flags::QualityLevel;
use crate::math::pca::{
    compute_weighted_covariance3, compute_weighted_covariance4, estimate_principle_component,
    estimate_principle_component_vec4, get_principle_projection_vec3,
    get_principle_projection_vec4,
};
use crate::math::vector::{
    Vec3, Vec4, find_closest_code_4, find_closest_code_4_rgb, find_closest_code_8,
    find_closest_code_8_rgb, find_closest_code_16,
};
use crate::tables::WEIGHTS_F32;
use crate::tables::bc7_masks::{get_subset_2, get_subset_3};

#[inline(always)]
pub fn compute_color_distance_rgb(
    e1: [u8; 4],
    e2: [u8; 4],
    perceptual: bool,
    weights: &[u32; 4],
) -> u64 {
    if perceptual {
        let l1 = e1[0] as f32 * 0.2126 + e1[1] as f32 * 0.7152 + e1[2] as f32 * 0.0722;
        let cr1 = e1[0] as f32 - l1;
        let cb1 = e1[2] as f32 - l1;

        let l2 = e2[0] as f32 * 0.2126 + e2[1] as f32 * 0.7152 + e2[2] as f32 * 0.0722;
        let cr2 = e2[0] as f32 - l2;
        let cb2 = e2[2] as f32 - l2;

        let dl = l1 - l2;
        let dcr = cr1 - cr2;
        let dcb = cb1 - cb2;

        (weights[0] as f32 * (dl * dl)
            + weights[1] as f32 * PR_WEIGHT * (dcr * dcr)
            + weights[2] as f32 * PB_WEIGHT * (dcb * dcb)) as u64
    } else {
        let dr = e1[0] as f32 - e2[0] as f32;
        let dg = e1[1] as f32 - e2[1] as f32;
        let db = e1[2] as f32 - e2[2] as f32;
        (weights[0] as f32 * dr * dr + weights[1] as f32 * dg * dg + weights[2] as f32 * db * db)
            as u64
    }
}

#[inline(always)]
pub fn compute_color_distance_rgba(
    e1: [u8; 4],
    e2: [u8; 4],
    perceptual: bool,
    weights: &[u32; 4],
) -> u64 {
    let da = e1[3] as f32 - e2[3] as f32;
    let a_err = weights[3] as f32 * (da * da);
    compute_color_distance_rgb(e1, e2, perceptual, weights) + a_err as u64
}

fn color_cell_compression_est(
    num_selector_weights: usize,
    weights: &[u32; 4],
    num_pixels: usize,
    pixels: &[[u8; 4]],
) -> u64 {
    if num_pixels == 0 {
        return 0;
    }

    let mut lr = 255.0f32;
    let mut lg = 255.0f32;
    let mut lb = 255.0f32;
    let mut hr = 0.0f32;
    let mut hg = 0.0f32;
    let mut hb = 0.0f32;

    for p in &pixels[..num_pixels] {
        lr = lr.min(p[0] as f32);
        lg = lg.min(p[1] as f32);
        lb = lb.min(p[2] as f32);
        hr = hr.max(p[0] as f32);
        hg = hg.max(p[1] as f32);
        hb = hb.max(p[2] as f32);
    }

    let dir = hr - lr;
    let dig = hg - lg;
    let dib = hb - lb;

    let low = dir * lr + dig * lg + dib * lb;
    let high = dir * hr + dig * hg + dib * hb;

    let denom = high - low;
    let scale = if denom > 1e-4 {
        ((num_selector_weights - 1) as f32) / denom
    } else {
        0.0
    };
    let inv_n = 1.0 / ((num_selector_weights - 1) as f32);

    let wr = weights[0] as f32;
    let wg = weights[1] as f32;
    let wb = weights[2] as f32;

    let mut total_errf = 0.0f32;

    for p in &pixels[..num_pixels] {
        let pr = p[0] as f32;
        let pg = p[1] as f32;
        let pb = p[2] as f32;

        let d = dir * pr + dig * pg + dib * pb;
        let s = (((d - low) * scale + 0.5).floor() * inv_n).clamp(0.0, 1.0);

        let itr = lr + dir * s;
        let itg = lg + dig * s;
        let itb = lb + dib * s;

        let dr = itr - pr;
        let dg = itg - pg;
        let db = itb - pb;

        total_errf += wr * dr * dr + wg * dg * dg + wb * db * db;
    }

    total_errf as u64
}

pub fn estimate_partition_list(
    mode: usize,
    pixels: &[[u8; 4]; 16],
    weights: &[u32; 4],
    max_solutions: usize,
    out_solutions: &mut [(usize, u64)],
) -> usize {
    let info = &BC7_MODE_INFO[mode];
    let total_subsets = info.num_subsets;
    let total_partitions = 1 << info.partition_bits;

    if total_partitions <= 1 {
        out_solutions[0] = (0, 0);
        return 1;
    }

    let num_weights = 1 << info.index_bits;
    let mut num_solutions = 0;

    for partition in 0..total_partitions {
        let mut subset_colors = [[[0u8; 4]; 16]; 3];
        let mut subset_counts = [0usize; 3];

        for (i, &px) in pixels.iter().enumerate() {
            let p = if total_subsets == 3 {
                get_subset_3(partition, i)
            } else {
                get_subset_2(partition, i)
            };
            subset_colors[p][subset_counts[p]] = px;
            subset_counts[p] += 1;
        }

        let mut total_err = 0u64;
        for s in 0..total_subsets {
            total_err += color_cell_compression_est(
                num_weights,
                weights,
                subset_counts[s],
                &subset_colors[s],
            );
        }

        let mut insert_pos = num_solutions;
        for (i, sol) in out_solutions.iter().enumerate().take(num_solutions) {
            if total_err < sol.1 {
                insert_pos = i;
                break;
            }
        }

        if insert_pos < max_solutions {
            let move_count = (max_solutions - 1).min(num_solutions) - insert_pos;
            for j in (0..move_count).rev() {
                out_solutions[insert_pos + j + 1] = out_solutions[insert_pos + j];
            }
            out_solutions[insert_pos] = (partition, total_err);
            if num_solutions < max_solutions {
                num_solutions += 1;
            }
        }

        if total_subsets == 2
            && partition == BC7E_2SUBSET_CHECKERBOARD_PARTITION_INDEX
            && insert_pos >= 4
        {
            break;
        }
    }

    num_solutions
}

#[derive(Debug, Clone, Default)]
pub struct PaletteSet {
    pub num_subsets: usize,
    pub rotation: usize,
    pub partition: usize,
    pub separate_alpha: bool,
    pub merged_alpha: bool,
    pub transparent: bool,
    pub count: [usize; 4],
    pub points: [[Vec4; 16]; 4],
    pub weights: [[f32; 16]; 4],
    pub remap: [[Option<u8>; 16]; 4],
}

impl PaletteSet {
    pub fn new(rgba: &[[u8; 4]; 16], mask: u16, mode: usize, part_or_rot: usize) -> Self {
        let mut set = Self::default();
        set.configure_mode(mode, part_or_rot);
        set.build_set(rgba, mask);
        set
    }

    pub fn from_initial(initial: &PaletteSet, mask: u16, mode: usize, partition: usize) -> Self {
        let mut set = Self::default();
        set.configure_mode(mode, partition);
        set.permute_set(initial, mask);
        set
    }

    fn configure_mode(&mut self, mode: usize, part_or_rot: usize) {
        match mode {
            0 => {
                self.num_subsets = 3;
                self.partition = part_or_rot.min(15);
                self.merged_alpha = false;
                self.separate_alpha = false;
            }
            1 | 3 => {
                self.num_subsets = 2;
                self.partition = part_or_rot.min(63);
                self.merged_alpha = false;
                self.separate_alpha = false;
            }
            2 => {
                self.num_subsets = 3;
                self.partition = part_or_rot.min(63);
                self.merged_alpha = false;
                self.separate_alpha = false;
            }
            4 | 5 => {
                self.num_subsets = 1;
                self.rotation = part_or_rot.min(3);
                self.separate_alpha = true;
                self.merged_alpha = false;
            }
            6 => {
                self.num_subsets = 1;
                self.partition = 0;
                self.merged_alpha = true;
                self.separate_alpha = false;
            }
            7 => {
                self.num_subsets = 2;
                self.partition = part_or_rot.min(63);
                self.merged_alpha = true;
                self.separate_alpha = false;
            }
            _ => panic!("Unsupported BC7 mode: {}", mode),
        }
    }

    fn build_set(&mut self, rgba: &[[u8; 4]; 16], mask: u16) {
        let mut transformed_rgba = [Vec4::splat(0.0); 16];

        for i in 0..16 {
            let mut r = rgba[i][0];
            let mut g = rgba[i][1];
            let mut b = rgba[i][2];
            let mut a = rgba[i][3];

            if a < 255 {
                self.transparent = true;
            }

            match self.rotation {
                1 => std::mem::swap(&mut r, &mut a),
                2 => std::mem::swap(&mut g, &mut a),
                3 => std::mem::swap(&mut b, &mut a),
                _ => {}
            }

            transformed_rgba[i] = Vec4::new(
                r as f32 / 255.0,
                g as f32 / 255.0,
                b as f32 / 255.0,
                if self.separate_alpha {
                    0.0
                } else {
                    a as f32 / 255.0
                },
            );
        }

        for s in 0..self.num_subsets {
            self.build_subset(s, &transformed_rgba, mask);
        }

        if self.separate_alpha {
            let alpha_subset_idx = self.num_subsets;
            let mut alpha_rgba = [Vec4::splat(0.0); 16];

            for i in 0..16 {
                let a = match self.rotation {
                    1 => rgba[i][0],
                    2 => rgba[i][1],
                    3 => rgba[i][2],
                    _ => rgba[i][3],
                };
                alpha_rgba[i] = Vec4::splat(a as f32 / 255.0);
            }

            self.build_subset(alpha_subset_idx, &alpha_rgba, mask);
        }

        let total_subsets = self.num_subsets + if self.separate_alpha { 1 } else { 0 };
        for s in 0..total_subsets {
            for i in 0..self.count[s] {
                self.weights[s][i] = self.weights[s][i].sqrt();
            }
        }
    }

    fn build_subset(&mut self, subset_idx: usize, transformed_points: &[Vec4; 16], mask: u16) {
        let mut count = 0;

        for (i, remap_slot) in self.remap[subset_idx].iter_mut().enumerate() {
            if (mask & (1 << i)) == 0 {
                *remap_slot = None;
                continue;
            }

            let belongs = if subset_idx >= self.num_subsets {
                true
            } else {
                match self.num_subsets {
                    1 => true,
                    2 => get_subset_2(self.partition, i) == subset_idx,
                    3 => get_subset_3(self.partition, i) == subset_idx,
                    _ => false,
                }
            };

            if !belongs {
                continue;
            }

            let pt = transformed_points[i];
            let w = 1.0f32;

            let mut found = None;
            for j in 0..count {
                if (self.points[subset_idx][j] - pt).length_squared() < 1e-7 {
                    found = Some(j);
                    break;
                }
            }

            match found {
                Some(idx) => {
                    *remap_slot = Some(idx as u8);
                    self.weights[subset_idx][idx] += w;
                }
                None => {
                    *remap_slot = Some(count as u8);
                    self.points[subset_idx][count] = pt;
                    self.weights[subset_idx][count] = w;
                    count += 1;
                }
            }
        }

        self.count[subset_idx] = count;
    }

    fn permute_set(&mut self, initial_1subset: &PaletteSet, mask: u16) {
        self.transparent = initial_1subset.transparent;

        for s in 0..self.num_subsets {
            let mut count = 0;
            let mut gotcha = [None; 16];

            for (i, remap_slot) in self.remap[s].iter_mut().enumerate() {
                if (mask & (1 << i)) == 0 {
                    *remap_slot = None;
                    continue;
                }

                let belongs = match self.num_subsets {
                    2 => get_subset_2(self.partition, i) == s,
                    3 => get_subset_3(self.partition, i) == s,
                    _ => true,
                };

                if !belongs {
                    continue;
                }

                if let Some(src_uindex) = initial_1subset.remap[0][i] {
                    let uidx = src_uindex as usize;
                    let pt = initial_1subset.points[0][uidx];
                    let w = initial_1subset.weights[0][uidx];

                    if let Some(new_idx) = gotcha[uidx] {
                        *remap_slot = Some(new_idx);
                        self.weights[s][new_idx as usize] += w;
                    } else {
                        let new_idx = count as u8;
                        gotcha[uidx] = Some(new_idx);
                        *remap_slot = Some(new_idx);
                        self.points[s][count] = pt;
                        self.weights[s][count] = w;
                        count += 1;
                    }
                }
            }

            self.count[s] = count;
        }
    }

    pub fn remap_indices(&self, source: &[u8], target: &mut [u8; 16], subset: usize) {
        for (i, target_slot) in target.iter_mut().enumerate() {
            if let Some(idx) = self.remap[subset][i] {
                *target_slot = source[idx as usize];
            }
        }
    }
}

pub struct PaletteRangeFit {
    pub start: [Vec4; 4],
    pub end: [Vec4; 4],
}

impl PaletteRangeFit {
    pub fn new(palette: &PaletteSet) -> Self {
        let mut start = [Vec4::splat(0.0); 4];
        let mut end = [Vec4::splat(0.0); 4];

        for s in 0..palette.num_subsets {
            let count = palette.count[s];
            if count == 0 {
                continue;
            }
            if count == 1 {
                start[s] = palette.points[s][0];
                end[s] = palette.points[s][0];
                continue;
            }
            if count == 2 {
                start[s] = palette.points[s][0];
                end[s] = palette.points[s][1];
                continue;
            }

            let pts = &palette.points[s][..count];
            let wts = &palette.weights[s][..count];

            if palette.merged_alpha {
                let (cov, centroid) = compute_weighted_covariance4(pts, wts);
                let principle = estimate_principle_component_vec4(&cov);
                let (s_proj, e_proj) = get_principle_projection_vec4(&principle, &centroid, pts);
                start[s] = s_proj;
                end[s] = e_proj;
            } else {
                let mut pts3 = [Vec3::splat(0.0); 16];
                for i in 0..count {
                    pts3[i] = pts[i].to_vec3();
                }
                let (cov, centroid) = compute_weighted_covariance3(&pts3[..count], wts);
                let principle = estimate_principle_component(&cov);
                let (s_proj, e_proj) =
                    get_principle_projection_vec3(&principle, &centroid, &pts3[..count]);
                start[s] = s_proj.extend(1.0);
                end[s] = e_proj.extend(1.0);
            }
        }

        if palette.separate_alpha {
            let a_idx = palette.num_subsets;
            let count = palette.count[a_idx];
            if count > 0 {
                let mut min_a = 1.0f32;
                let mut max_a = 0.0f32;
                for i in 0..count {
                    let a = palette.points[a_idx][i].x;
                    min_a = min_a.min(a);
                    max_a = max_a.max(a);
                }
                start[a_idx] = Vec4::splat(min_a);
                end[a_idx] = Vec4::splat(max_a);
            }
        }

        Self { start, end }
    }

    pub fn compress(
        &self,
        palette: &PaletteSet,
        mode: usize,
        metric: &Vec4,
        out_block: &mut [u8; 16],
    ) -> f32 {
        let info = &BC7_MODE_INFO[mode];
        let index_bits = info.index_bits as usize;
        let ccs = 1 << index_bits;

        let mut total_error = 0.0f32;
        let mut snapped_start = [[0u8; 4]; 3];
        let mut snapped_end = [[0u8; 4]; 3];
        let mut p_bits = [0u8; 6];
        let mut indices = [0u8; 16];
        let mut alpha_indices = [0u8; 16];

        for s in 0..palette.num_subsets {
            let count = palette.count[s];
            if count == 0 {
                continue;
            }

            let mut best_subset_err = f32::MAX;
            let mut best_s_u = [0u8; 4];
            let mut best_e_u = [0u8; 4];
            let mut best_p = [0u8; 2];
            let mut best_sub_indices = [0u8; 16];

            if info.endpoint_pbits > 0 {
                for p0 in 0..=1u32 {
                    for p1 in 0..=1u32 {
                        let (s_u, s_norm) = quantize_endpoint(
                            self.start[s],
                            info.color_bits,
                            info.alpha_bits,
                            Some(p0),
                        );
                        let (e_u, e_norm) = quantize_endpoint(
                            self.end[s],
                            info.color_bits,
                            info.alpha_bits,
                            Some(p1),
                        );

                        let mut codes = [Vec4::splat(0.0); 16];
                        for i in 0..ccs {
                            let w = WEIGHTS_F32[index_bits][i];
                            codes[i] = *metric * (s_norm * (1.0 - w) + e_norm * w);
                        }

                        let mut sub_ind = [0u8; 16];
                        let mut err = 0.0f32;
                        for (i, slot) in sub_ind[..count].iter_mut().enumerate() {
                            let pt = *metric * palette.points[s][i];
                            let (min_d, best_idx) = match index_bits {
                                2 => find_closest_code_4(pt, (&codes[0..4]).try_into().unwrap()),
                                3 => find_closest_code_8(pt, (&codes[0..8]).try_into().unwrap()),
                                _ => find_closest_code_16(pt, &codes),
                            };
                            *slot = best_idx;
                            err += min_d * palette.weights[s][i];
                        }

                        if err < best_subset_err {
                            best_subset_err = err;
                            best_s_u = s_u;
                            best_e_u = e_u;
                            best_p = [p0 as u8, p1 as u8];
                            best_sub_indices = sub_ind;
                        }
                    }
                }
                p_bits[s * 2] = best_p[0];
                p_bits[s * 2 + 1] = best_p[1];
            } else if info.shared_pbits > 0 {
                for p in 0..=1u32 {
                    let (s_u, s_norm) =
                        quantize_endpoint(self.start[s], info.color_bits, info.alpha_bits, Some(p));
                    let (e_u, e_norm) =
                        quantize_endpoint(self.end[s], info.color_bits, info.alpha_bits, Some(p));

                    let mut codes = [Vec4::splat(0.0); 16];
                    for i in 0..ccs {
                        let w = WEIGHTS_F32[index_bits][i];
                        codes[i] = *metric * (s_norm * (1.0 - w) + e_norm * w);
                    }

                    let mut sub_ind = [0u8; 16];
                    let mut err = 0.0f32;
                    for (i, slot) in sub_ind[..count].iter_mut().enumerate() {
                        let pt = *metric * palette.points[s][i];
                        let (min_d, best_idx) = match index_bits {
                            2 => find_closest_code_4(pt, (&codes[0..4]).try_into().unwrap()),
                            3 => find_closest_code_8(pt, (&codes[0..8]).try_into().unwrap()),
                            _ => find_closest_code_16(pt, &codes),
                        };
                        *slot = best_idx;
                        err += min_d * palette.weights[s][i];
                    }

                    if err < best_subset_err {
                        best_subset_err = err;
                        best_s_u = s_u;
                        best_e_u = e_u;
                        best_p = [p as u8, 0];
                        best_sub_indices = sub_ind;
                    }
                }
                p_bits[s] = best_p[0];
            } else {
                let (s_u, s_norm) =
                    quantize_endpoint(self.start[s], info.color_bits, info.alpha_bits, None);
                let (e_u, e_norm) =
                    quantize_endpoint(self.end[s], info.color_bits, info.alpha_bits, None);

                let mut sub_ind = [0u8; 16];
                let mut err = 0.0f32;

                if palette.separate_alpha {
                    let metric_rgb = metric.to_vec3();
                    let mut codes_rgb = [Vec3::splat(0.0); 16];
                    for i in 0..ccs {
                        let w = WEIGHTS_F32[index_bits][i];
                        codes_rgb[i] =
                            metric_rgb * (s_norm.to_vec3() * (1.0 - w) + e_norm.to_vec3() * w);
                    }
                    for (i, slot) in sub_ind[..count].iter_mut().enumerate() {
                        let pt_rgb = metric_rgb * palette.points[s][i].to_vec3();
                        let (min_d, best_idx) = match index_bits {
                            2 => find_closest_code_4_rgb(
                                pt_rgb,
                                (&codes_rgb[0..4]).try_into().unwrap(),
                            ),
                            3 => find_closest_code_8_rgb(
                                pt_rgb,
                                (&codes_rgb[0..8]).try_into().unwrap(),
                            ),
                            _ => find_closest_code_4_rgb(
                                pt_rgb,
                                (&codes_rgb[0..4]).try_into().unwrap(),
                            ),
                        };
                        *slot = best_idx;
                        err += min_d * palette.weights[s][i];
                    }
                } else {
                    let mut codes = [Vec4::splat(0.0); 16];
                    for i in 0..ccs {
                        let w = WEIGHTS_F32[index_bits][i];
                        codes[i] = *metric * (s_norm * (1.0 - w) + e_norm * w);
                    }
                    for (i, slot) in sub_ind[..count].iter_mut().enumerate() {
                        let pt = *metric * palette.points[s][i];
                        let (min_d, best_idx) = match index_bits {
                            2 => find_closest_code_4(pt, (&codes[0..4]).try_into().unwrap()),
                            3 => find_closest_code_8(pt, (&codes[0..8]).try_into().unwrap()),
                            _ => find_closest_code_16(pt, &codes),
                        };
                        *slot = best_idx;
                        err += min_d * palette.weights[s][i];
                    }
                }

                best_subset_err = err;
                best_s_u = s_u;
                best_e_u = e_u;
                best_sub_indices = sub_ind;
            }

            snapped_start[s] = best_s_u;
            snapped_end[s] = best_e_u;
            total_error += best_subset_err;

            palette.remap_indices(&best_sub_indices, &mut indices, s);
        }

        if palette.separate_alpha {
            let a_idx = palette.num_subsets;
            let count = palette.count[a_idx];

            if count > 0 {
                let s_a = self.start[a_idx].x;
                let e_a = self.end[a_idx].x;

                let max_a = ((1 << info.alpha_bits) - 1) as f32;
                let q_s = (s_a.clamp(0.0, 1.0) * max_a).round() as u32;
                let q_e = (e_a.clamp(0.0, 1.0) * max_a).round() as u32;

                let a_start = expand_quantized(q_s, info.alpha_bits, None);
                let a_end = expand_quantized(q_e, info.alpha_bits, None);

                snapped_start[0][3] = a_start;
                snapped_end[0][3] = a_end;

                let a_s_norm = a_start as f32 / 255.0;
                let a_e_norm = a_end as f32 / 255.0;

                let a_bits = info.secondary_index_bits as usize;
                let a_ccs = 1 << a_bits;

                let mut a_codes = [0.0f32; 16];
                for i in 0..a_ccs {
                    let w = WEIGHTS_F32[a_bits][i];
                    a_codes[i] = metric.w * (a_s_norm * (1.0 - w) + a_e_norm * w);
                }

                let mut subset_a_indices = [0u8; 16];
                for (i, slot) in subset_a_indices[..count].iter_mut().enumerate() {
                    let val = metric.w * palette.points[a_idx][i].x;
                    let mut min_dist = f32::MAX;
                    let mut best_idx = 0u8;
                    for (j, &a_code) in a_codes[..a_ccs].iter().enumerate() {
                        let diff = val - a_code;
                        let dist = diff * diff;
                        if dist < min_dist {
                            min_dist = dist;
                            best_idx = j as u8;
                        }
                    }
                    *slot = best_idx;
                    total_error += min_dist * palette.weights[a_idx][i];
                }
                palette.remap_indices(&subset_a_indices, &mut alpha_indices, a_idx);
            }
        }

        fix_bc7_anchor_indices(
            mode,
            palette.partition,
            &mut snapped_start,
            &mut snapped_end,
            &mut p_bits,
            &mut indices,
            &mut alpha_indices,
        );
        write_bc7_block(
            mode,
            palette.partition,
            palette.rotation,
            0,
            &snapped_start,
            &snapped_end,
            &p_bits,
            &indices,
            &alpha_indices,
            out_block,
        );

        total_error
    }
}

pub struct PaletteClusterFit {
    pub max_iterations: usize,
}

impl PaletteClusterFit {
    pub fn new(max_iterations: usize) -> Self {
        Self {
            max_iterations: max_iterations.clamp(1, 8),
        }
    }

    pub fn compress(
        &self,
        palette: &PaletteSet,
        mode: usize,
        metric: &Vec4,
        out_block: &mut [u8; 16],
    ) -> f32 {
        let rf = PaletteRangeFit::new(palette);
        rf.compress(palette, mode, metric, out_block)
    }
}

pub fn handle_opaque_block(
    pixels: &[[u8; 4]; 16],
    mask: u16,
    metric: &Vec4,
    quality: QualityLevel,
    out_block: &mut [u8; 16],
) {
    let initial_1subset = PaletteSet::new(pixels, mask, 6, 0);

    if initial_1subset.count[0] <= 1 {
        let c = if initial_1subset.count[0] == 1 {
            let p = initial_1subset.points[0][0];
            [
                (p.x * 255.0).round() as u8,
                (p.y * 255.0).round() as u8,
                (p.z * 255.0).round() as u8,
                255,
            ]
        } else {
            [0, 0, 0, 255]
        };

        let t = &BC7_TABLES;
        let mut best_p = 0;
        let mut best_err = u32::MAX;

        for p in 0..4 {
            let hi = p >> 1;
            let lo = p & 1;
            let err = t.mode_6[c[0] as usize][hi][lo].error as u32
                + t.mode_6[c[1] as usize][hi][lo].error as u32
                + t.mode_6[c[2] as usize][hi][lo].error as u32;
            if err < best_err {
                best_err = err;
                best_p = p;
            }
        }

        let hi = best_p >> 1;
        let lo = best_p & 1;
        let er = t.mode_6[c[0] as usize][hi][lo];
        let eg = t.mode_6[c[1] as usize][hi][lo];
        let eb = t.mode_6[c[2] as usize][hi][lo];
        let ea = t.mode_6[255][hi][lo];

        let low = [er.lo, eg.lo, eb.lo, ea.lo];
        let high = [er.hi, eg.hi, eb.hi, ea.hi];
        let pbits = [lo as u8, hi as u8];
        let selectors = [5u8; 16];

        encode_bc7_block_mode6(low, high, pbits, &selectors, out_block);
        return;
    }

    let mut best_error = f32::MAX;
    let mut best_block = [0u8; 16];

    // Mode 6 Evaluation
    {
        let palette = PaletteSet::new(pixels, mask, 6, 0);
        let rf = PaletteRangeFit::new(&palette);
        let mut block = [0u8; 16];
        let err = rf.compress(&palette, 6, metric, &mut block);
        if err < best_error {
            best_error = err;
            best_block = block;
        }
    }

    if quality == QualityLevel::Ultrafast {
        *out_block = best_block;
        return;
    }

    let weights_u32 = [
        (metric.x * 128.0).round() as u32,
        (metric.y * 64.0).round() as u32,
        (metric.z * 16.0).round() as u32,
        256,
    ];

    let max_solutions2 = match quality {
        QualityLevel::Ultrafast => 0,
        QualityLevel::Fast => 2,
        QualityLevel::Normal => 8,
        QualityLevel::Slow => 16,
        QualityLevel::Slowest => 32,
    };

    if max_solutions2 > 0 {
        let mut solutions2 = [(0usize, 0u64); 32];
        let num_solutions2 =
            estimate_partition_list(1, pixels, &weights_u32, max_solutions2, &mut solutions2);

        for sol in &solutions2[..num_solutions2] {
            let p = sol.0;
            for &mode in &[1, 3] {
                let subset_p = PaletteSet::from_initial(&initial_1subset, mask, mode, p);
                let rf = PaletteRangeFit::new(&subset_p);
                let mut block = [0u8; 16];
                let err = rf.compress(&subset_p, mode, metric, &mut block);

                if err < best_error {
                    best_error = err;
                    best_block = block;
                }
            }
        }
    }

    *out_block = best_block;
}

pub fn handle_alpha_block(
    pixels: &[[u8; 4]; 16],
    mask: u16,
    metric: &Vec4,
    quality: QualityLevel,
    out_block: &mut [u8; 16],
) {
    let initial_1subset = PaletteSet::new(pixels, mask, 6, 0);

    let mut best_error = f32::MAX;
    let mut best_block = [0u8; 16];

    // Mode 6
    {
        let palette = PaletteSet::new(pixels, mask, 6, 0);
        let rf = PaletteRangeFit::new(&palette);
        let mut block = [0u8; 16];
        let err = rf.compress(&palette, 6, metric, &mut block);
        if err < best_error {
            best_error = err;
            best_block = block;
        }
    }

    // Mode 4 / 5
    let max_rot = if quality == QualityLevel::Ultrafast {
        1
    } else {
        4
    };
    for &mode in &[4, 5] {
        for rot in 0..max_rot {
            let palette = PaletteSet::new(pixels, mask, mode, rot);
            let rf = PaletteRangeFit::new(&palette);
            let mut block = [0u8; 16];
            let err = rf.compress(&palette, mode, metric, &mut block);
            if err < best_error {
                best_error = err;
                best_block = block;
            }
        }
    }

    // Mode 7
    let max_solutions7 = match quality {
        QualityLevel::Ultrafast => 0,
        QualityLevel::Fast => 2,
        QualityLevel::Normal => 8,
        _ => 16,
    };

    if max_solutions7 > 0 {
        let weights_u32 = [
            (metric.x * 128.0).round() as u32,
            (metric.y * 64.0).round() as u32,
            (metric.z * 16.0).round() as u32,
            256,
        ];
        let mut solutions7 = [(0usize, 0u64); 32];
        let num_solutions7 =
            estimate_partition_list(7, pixels, &weights_u32, max_solutions7, &mut solutions7);

        for sol in &solutions7[..num_solutions7] {
            let p = sol.0;
            let subset_p = PaletteSet::from_initial(&initial_1subset, mask, 7, p);
            let rf = PaletteRangeFit::new(&subset_p);
            let mut block = [0u8; 16];
            let err = rf.compress(&subset_p, 7, metric, &mut block);
            if err < best_error {
                best_error = err;
                best_block = block;
            }
        }
    }

    *out_block = best_block;
}
