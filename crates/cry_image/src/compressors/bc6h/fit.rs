use super::encode::write_bc6h_block;
use super::quant::quantize_and_clamp_endpoints;
use super::tables::BC6H_CONFIGS;
use crate::math::pca::{
    compute_weighted_covariance3, estimate_principle_component, get_principle_projection_vec3,
};
use crate::math::vector::Vec3;
use crate::tables::WEIGHTS_F32;
use crate::tables::bc7_masks::{ANCHOR_INDEX_2_SUBSET_1, get_subset_2};

#[derive(Debug, Clone, Default)]
pub struct HDRSet {
    pub num_subsets: usize,
    pub partition: usize,
    pub count: [usize; 2],
    pub points: [[Vec3; 16]; 2],
    pub weights: [[f32; 16]; 2],
    pub remap: [[Option<u8>; 16]; 2],
}

impl HDRSet {
    pub fn new(rgb: &[[f32; 3]; 16], mask: u16, num_subsets: usize, partition: usize) -> Self {
        let mut set = Self {
            num_subsets,
            partition,
            ..Default::default()
        };
        for s in 0..num_subsets {
            let mut count = 0;
            for (i, &rgb_px) in rgb.iter().enumerate() {
                if (mask & (1 << i)) == 0 {
                    set.remap[s][i] = None;
                    continue;
                }
                if num_subsets == 2 && get_subset_2(partition, i) != s {
                    continue;
                }

                let pt = Vec3::new(rgb_px[0], rgb_px[1], rgb_px[2]);
                let mut found = None;
                for j in 0..count {
                    if (set.points[s][j] - pt).length_squared() < 1e-6 {
                        found = Some(j);
                        break;
                    }
                }

                match found {
                    Some(idx) => {
                        set.remap[s][i] = Some(idx as u8);
                        set.weights[s][idx] += 1.0;
                    }
                    None => {
                        set.remap[s][i] = Some(count as u8);
                        set.points[s][count] = pt;
                        set.weights[s][count] = 1.0;
                        count += 1;
                    }
                }
            }
            set.count[s] = count;
        }
        set
    }

    pub fn from_initial(initial: &HDRSet, partition: usize) -> Self {
        let mut set = Self {
            num_subsets: 2,
            partition,
            ..Default::default()
        };

        for s in 0..2 {
            let mut count = 0;
            let mut gotcha = [None; 16];

            for i in 0..16 {
                if get_subset_2(partition, i) != s {
                    continue;
                }
                if let Some(uidx_raw) = initial.remap[0][i] {
                    let uidx = uidx_raw as usize;
                    let pt = initial.points[0][uidx];
                    let w = initial.weights[0][uidx];

                    if let Some(new_idx) = gotcha[uidx] {
                        set.remap[s][i] = Some(new_idx);
                        set.weights[s][new_idx as usize] += w;
                    } else {
                        let new_idx = count as u8;
                        gotcha[uidx] = Some(new_idx);
                        set.remap[s][i] = Some(new_idx);
                        set.points[s][count] = pt;
                        set.weights[s][count] = w;
                        count += 1;
                    }
                }
            }
            set.count[s] = count;
        }
        set
    }

    pub fn remap_indices(&self, source: &[u8], target: &mut [u8; 16], subset: usize) {
        for (i, target_slot) in target.iter_mut().enumerate() {
            if let Some(idx) = self.remap[subset][i] {
                *target_slot = source[idx as usize];
            }
        }
    }
}

pub struct HDRRangeFit {
    pub start: [Vec3; 2],
    pub end: [Vec3; 2],
}

impl HDRRangeFit {
    pub fn new(set: &HDRSet) -> Self {
        let mut start = [Vec3::splat(0.0); 2];
        let mut end = [Vec3::splat(0.0); 2];

        for s in 0..set.num_subsets {
            let count = set.count[s];
            if count == 0 {
                continue;
            }
            if count == 1 {
                start[s] = set.points[s][0];
                end[s] = set.points[s][0];
                continue;
            }

            let pts = &set.points[s][..count];
            let wts = &set.weights[s][..count];
            let (cov, center) = compute_weighted_covariance3(pts, wts);
            let principle = estimate_principle_component(&cov);
            let (s_proj, e_proj) = get_principle_projection_vec3(&principle, &center, pts);

            start[s] = s_proj;
            end[s] = e_proj;
        }

        Self { start, end }
    }

    pub fn compress(
        &self,
        set: &HDRSet,
        mode: usize,
        metric: &Vec3,
        is_signed: bool,
        out_block: &mut [u8; 16],
    ) -> f32 {
        let cfg = &BC6H_CONFIGS[mode - 1];
        let is_two_subsets = cfg.num_subsets == 2;
        let index_bits = cfg.index_bits as usize;
        let ccs = 1 << index_bits;

        let mut current_start = self.start;
        let mut current_end = self.end;

        for s in 0..cfg.num_subsets {
            let anchor_px = if s == 0 {
                0
            } else {
                ANCHOR_INDEX_2_SUBSET_1[set.partition]
            };
            if let Some(uidx) = set.remap[s][anchor_px] {
                let anchor_pt = set.points[s][uidx as usize];
                if (anchor_pt - current_start[s]).length_squared()
                    > (anchor_pt - current_end[s]).length_squared()
                {
                    std::mem::swap(&mut current_start[s], &mut current_end[s]);
                }
            }
        }

        let (endpoints_q, unq) =
            quantize_and_clamp_endpoints(&current_start, &current_end, cfg, is_signed);

        let mut total_error = 0.0f32;
        let mut indices = [0u8; 16];

        for (s, unq_sub) in unq[..cfg.num_subsets].iter().enumerate() {
            let count = set.count[s];
            if count == 0 {
                continue;
            }

            let s_unq = unq_sub[0];
            let e_unq = unq_sub[1];

            let mut codes = [Vec3::splat(0.0); 16];
            for i in 0..ccs {
                let w = WEIGHTS_F32[index_bits][i];
                codes[i] = *metric * (s_unq * (1.0 - w) + e_unq * w);
            }

            let mut subset_indices = [0u8; 16];
            for (i, slot) in subset_indices[..count].iter_mut().enumerate() {
                let pt = *metric * set.points[s][i];
                let mut min_dist = f32::MAX;
                let mut best_idx = 0u8;

                for (j, code) in codes[..ccs].iter().enumerate() {
                    let dist = (pt - *code).length_squared();
                    if dist < min_dist {
                        min_dist = dist;
                        best_idx = j as u8;
                    }
                }

                *slot = best_idx;
                total_error += min_dist * set.weights[s][i];
            }

            set.remap_indices(&subset_indices, &mut indices, s);
        }

        let mut final_endpoints_q = endpoints_q;

        if indices[0] >= (ccs / 2) as u8 {
            indices[0] = (ccs / 2) as u8 - 1;
        }

        if is_two_subsets {
            let anchor1 = ANCHOR_INDEX_2_SUBSET_1[set.partition];
            if indices[anchor1] >= (ccs / 2) as u8 {
                final_endpoints_q[1].swap(0, 1);
                for (i, idx) in indices.iter_mut().enumerate() {
                    if get_subset_2(set.partition, i) == 1 {
                        *idx = ((ccs - 1) as u8) - *idx;
                    }
                }
            }
        }

        write_bc6h_block(
            mode,
            set.partition,
            &final_endpoints_q,
            &indices,
            is_signed,
            out_block,
        );
        total_error
    }
}

pub struct HDRClusterFit {
    pub max_iterations: usize,
}

impl HDRClusterFit {
    pub fn new(max_iterations: usize) -> Self {
        Self {
            max_iterations: max_iterations.clamp(1, 15),
        }
    }

    pub fn compress(
        &self,
        set: &HDRSet,
        mode: usize,
        metric: &Vec3,
        is_signed: bool,
        out_block: &mut [u8; 16],
    ) -> f32 {
        let cfg = &BC6H_CONFIGS[mode - 1];
        let is_two_subsets = cfg.num_subsets == 2;
        let index_bits = cfg.index_bits as usize;
        let ccs = 1 << index_bits;

        let rf = HDRRangeFit::new(set);
        let mut current_start = rf.start;
        let mut current_end = rf.end;

        for s in 0..cfg.num_subsets {
            let anchor_px = if s == 0 {
                0
            } else {
                ANCHOR_INDEX_2_SUBSET_1[set.partition]
            };
            if let Some(uidx) = set.remap[s][anchor_px] {
                let anchor_pt = set.points[s][uidx as usize];
                if (anchor_pt - current_start[s]).length_squared()
                    > (anchor_pt - current_end[s]).length_squared()
                {
                    std::mem::swap(&mut current_start[s], &mut current_end[s]);
                }
            }
        }

        for s in 0..cfg.num_subsets {
            let count = set.count[s];
            if count <= 1 {
                continue;
            }

            let anchor_px = if s == 0 {
                0
            } else {
                ANCHOR_INDEX_2_SUBSET_1[set.partition]
            };
            let anchor_pt = if let Some(uidx) = set.remap[s][anchor_px] {
                set.points[s][uidx as usize]
            } else {
                set.points[s][0]
            };

            let pts = &set.points[s][..count];
            let wts = &set.weights[s][..count];

            let mut best_start = current_start[s];
            let mut best_end = current_end[s];

            for _ in 0..self.max_iterations {
                if (anchor_pt - best_start).length_squared()
                    > (anchor_pt - best_end).length_squared()
                {
                    std::mem::swap(&mut best_start, &mut best_end);
                }

                let s_unq = best_start;
                let e_unq = best_end;

                let mut codes = [Vec3::splat(0.0); 16];
                for k in 0..ccs {
                    let w = WEIGHTS_F32[index_bits][k];
                    codes[k] = s_unq * (1.0 - w) + e_unq * w;
                }

                let mut alpha2 = 0.0f32;
                let mut beta2 = 0.0f32;
                let mut alphabeta = 0.0f32;
                let mut alphax = Vec3::splat(0.0);
                let mut betax = Vec3::splat(0.0);

                for (k, pt) in pts.iter().enumerate() {
                    let pt_m = *metric * *pt;
                    let mut min_d = f32::MAX;
                    let mut best_k = 0;
                    for (l, code) in codes[..ccs].iter().enumerate() {
                        let d = (pt_m - *metric * *code).length_squared();
                        if d < min_d {
                            min_d = d;
                            best_k = l;
                        }
                    }

                    let w = WEIGHTS_F32[index_bits][best_k];
                    let alpha = 1.0 - w;
                    let beta = w;
                    let pt_weight = wts[k];

                    alpha2 += alpha * alpha * pt_weight;
                    beta2 += beta * beta * pt_weight;
                    alphabeta += alpha * beta * pt_weight;
                    alphax += *pt * (alpha * pt_weight);
                    betax += *pt * (beta * pt_weight);
                }

                let denom = alpha2 * beta2 - alphabeta * alphabeta;
                if denom.abs() > f32::EPSILON {
                    let inv_denom = 1.0 / denom;
                    let mut next_s = (alphax * beta2 - betax * alphabeta) * inv_denom;
                    let mut next_e = (betax * alpha2 - alphax * alphabeta) * inv_denom;

                    if (anchor_pt - next_s).length_squared() > (anchor_pt - next_e).length_squared()
                    {
                        std::mem::swap(&mut next_s, &mut next_e);
                    }

                    best_start = next_s;
                    best_end = next_e;
                }
            }

            current_start[s] = best_start;
            current_end[s] = best_end;
        }

        let (endpoints_q, unq) =
            quantize_and_clamp_endpoints(&current_start, &current_end, cfg, is_signed);

        let mut total_error = 0.0f32;
        let mut indices = [0u8; 16];

        for (s, unq_sub) in unq[..cfg.num_subsets].iter().enumerate() {
            let count = set.count[s];
            if count == 0 {
                continue;
            }

            let s_unq = unq_sub[0];
            let e_unq = unq_sub[1];

            let mut codes = [Vec3::splat(0.0); 16];
            for i in 0..ccs {
                let w = WEIGHTS_F32[index_bits][i];
                codes[i] = *metric * (s_unq * (1.0 - w) + e_unq * w);
            }

            let mut subset_indices = [0u8; 16];
            for (i, slot) in subset_indices[..count].iter_mut().enumerate() {
                let pt = *metric * set.points[s][i];
                let mut min_dist = f32::MAX;
                let mut best_idx = 0u8;

                for (j, code) in codes[..ccs].iter().enumerate() {
                    let dist = (pt - *code).length_squared();
                    if dist < min_dist {
                        min_dist = dist;
                        best_idx = j as u8;
                    }
                }

                *slot = best_idx;
                total_error += min_dist * set.weights[s][i];
            }

            set.remap_indices(&subset_indices, &mut indices, s);
        }

        let mut final_endpoints_q = endpoints_q;

        if indices[0] >= (ccs / 2) as u8 {
            indices[0] = (ccs / 2) as u8 - 1;
        }

        if is_two_subsets {
            let anchor1 = ANCHOR_INDEX_2_SUBSET_1[set.partition];
            if indices[anchor1] >= (ccs / 2) as u8 {
                final_endpoints_q[1].swap(0, 1);
                for (i, idx) in indices.iter_mut().enumerate() {
                    if get_subset_2(set.partition, i) == 1 {
                        *idx = ((ccs - 1) as u8) - *idx;
                    }
                }
            }
        }

        write_bc6h_block(
            mode,
            set.partition,
            &final_endpoints_q,
            &indices,
            is_signed,
            out_block,
        );
        total_error
    }
}
