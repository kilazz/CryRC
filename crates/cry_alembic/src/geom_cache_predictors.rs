use super::geom_cache_file::{
    MESH_PREDICTOR_LOOK_BACK_MAX_DIST, Position, STemporalPredictorControl,
};
use std::collections::{HashMap, HashSet};

pub fn calculate_entropy(data: &[u8]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let mut counts = [0usize; 256];
    for &b in data {
        counts[b as usize] += 1;
    }
    let total = data.len() as f32;
    let mut sum = 0.0f32;
    for &cnt in &counts {
        if cnt > 0 {
            let p = cnt as f32 / total;
            sum += p * p.log2();
        }
    }
    -sum
}

pub fn parallelogram_predict_positions(
    positions: &[Position],
    predictor_indices: &[u16],
) -> Vec<Position> {
    let mut predicted = Vec::with_capacity(positions.len());
    let mut pred_idx = 0;

    for i in 0..positions.len() {
        if pred_idx + 2 < predictor_indices.len() && predictor_indices[pred_idx] != 0xFFFF {
            let u_dist = predictor_indices[pred_idx] as usize;
            let v_dist = predictor_indices[pred_idx + 1] as usize;
            let w_dist = predictor_indices[pred_idx + 2] as usize;
            pred_idx += 3;

            let u = positions[i - u_dist];
            let v = positions[i - v_dist];
            let w = positions[i - w_dist];

            let pred_x = (u.x as i32 + v.x as i32 - w.x as i32) as u16;
            let pred_y = (u.y as i32 + v.y as i32 - w.y as i32) as u16;
            let pred_z = (u.z as i32 + v.z as i32 - w.z as i32) as u16;

            predicted.push(Position {
                x: positions[i].x.wrapping_sub(pred_x),
                y: positions[i].y.wrapping_sub(pred_y),
                z: positions[i].z.wrapping_sub(pred_z),
            });
        } else {
            if pred_idx < predictor_indices.len() && predictor_indices[pred_idx] == 0xFFFF {
                pred_idx += 1;
            }
            predicted.push(positions[i]);
        }
    }
    predicted
}

pub fn optimize_mesh_for_compression(
    positions: &[Position],
    indices_map: &mut HashMap<u16, Vec<u32>>,
    use_mesh_prediction: bool,
) -> (Vec<Position>, Vec<u16>) {
    let mut reorder_map: HashMap<u32, u32> = HashMap::new();
    let mut current_new_idx = 0u32;

    for indices in indices_map.values_mut() {
        for idx in indices.iter_mut() {
            let old = *idx;
            let new_idx = *reorder_map.entry(old).or_insert_with(|| {
                let n = current_new_idx;
                current_new_idx += 1;
                n
            });
            *idx = new_idx;
        }
    }

    let mut reordered_positions = vec![Position::default(); positions.len()];
    for (old, &new) in &reorder_map {
        if (*old as usize) < positions.len() {
            reordered_positions[new as usize] = positions[*old as usize];
        }
    }

    if !use_mesh_prediction {
        return (reordered_positions, Vec::new());
    }

    let mut neighbor_map: HashMap<u32, HashSet<u32>> = HashMap::new();
    for indices in indices_map.values() {
        for chunk in indices.chunks_exact(3) {
            let (i1, i2, i3) = (chunk[0], chunk[1], chunk[2]);
            neighbor_map.entry(i1).or_default().insert(i2);
            neighbor_map.entry(i1).or_default().insert(i3);
            neighbor_map.entry(i2).or_default().insert(i1);
            neighbor_map.entry(i2).or_default().insert(i3);
            neighbor_map.entry(i3).or_default().insert(i1);
            neighbor_map.entry(i3).or_default().insert(i2);
        }
    }

    let mut predictor_data = Vec::new();
    let num_positions = reordered_positions.len();

    for curr_idx in 0..num_positions as u32 {
        let mut found_triangle = false;
        if let Some(neighbors) = neighbor_map.get(&curr_idx) {
            let mut sorted_neighbors: Vec<u32> = neighbors.iter().copied().collect();
            sorted_neighbors.sort();

            for &n_idx in &sorted_neighbors {
                if n_idx < curr_idx
                    && (curr_idx - n_idx) <= MESH_PREDICTOR_LOOK_BACK_MAX_DIST
                    && let Some(n_neighbors) = neighbor_map.get(&n_idx)
                {
                    for &nn_idx in n_neighbors {
                        if nn_idx < curr_idx
                            && nn_idx != n_idx
                            && (curr_idx - nn_idx) <= MESH_PREDICTOR_LOOK_BACK_MAX_DIST
                            && neighbors.contains(&nn_idx)
                        {
                            let u_dist = (curr_idx - n_idx) as u16;
                            let v_dist = (curr_idx - nn_idx) as u16;
                            let w_dist = (curr_idx.saturating_sub(n_idx.min(nn_idx))) as u16;

                            predictor_data.push(u_dist);
                            predictor_data.push(v_dist);
                            predictor_data.push(w_dist);
                            found_triangle = true;
                            break;
                        }
                    }
                }
                if found_triangle {
                    break;
                }
            }
        }
        if !found_triangle {
            predictor_data.push(0xFFFF);
        }
    }

    (reordered_positions, predictor_data)
}

pub fn optimize_temporal_predictor(
    in_positions: &[Position],
    floor_frame: &[Position],
    ceil_frame: &[Position],
) -> (STemporalPredictorControl, Vec<Position>) {
    let mut control = STemporalPredictorControl::default();
    let num_elements = in_positions.len();

    let mut best_lerp = 128u8;
    let mut min_entropy = f32::MAX;

    for lerp in (0..=255).step_by(8) {
        let factor = lerp as f32 / 255.0;
        let mut deltas_bytes = Vec::with_capacity(num_elements * 6);

        for i in 0..num_elements {
            let interp_x =
                (floor_frame[i].x as f32 * (1.0 - factor) + ceil_frame[i].x as f32 * factor) as u16;
            let interp_y =
                (floor_frame[i].y as f32 * (1.0 - factor) + ceil_frame[i].y as f32 * factor) as u16;
            let interp_z =
                (floor_frame[i].z as f32 * (1.0 - factor) + ceil_frame[i].z as f32 * factor) as u16;

            let dx = in_positions[i].x.wrapping_sub(interp_x);
            let dy = in_positions[i].y.wrapping_sub(interp_y);
            let dz = in_positions[i].z.wrapping_sub(interp_z);

            deltas_bytes.extend_from_slice(&dx.to_le_bytes());
            deltas_bytes.extend_from_slice(&dy.to_le_bytes());
            deltas_bytes.extend_from_slice(&dz.to_le_bytes());
        }

        let entropy = calculate_entropy(&deltas_bytes);
        if entropy < min_entropy {
            min_entropy = entropy;
            best_lerp = lerp;
        }
    }

    control.index_frame_lerp_factor = best_lerp;
    let factor = best_lerp as f32 / 255.0;
    let mut out_deltas = Vec::with_capacity(num_elements);

    for i in 0..num_elements {
        let interp_x =
            (floor_frame[i].x as f32 * (1.0 - factor) + ceil_frame[i].x as f32 * factor) as u16;
        let interp_y =
            (floor_frame[i].y as f32 * (1.0 - factor) + ceil_frame[i].y as f32 * factor) as u16;
        let interp_z =
            (floor_frame[i].z as f32 * (1.0 - factor) + ceil_frame[i].z as f32 * factor) as u16;

        out_deltas.push(Position {
            x: in_positions[i].x.wrapping_sub(interp_x),
            y: in_positions[i].y.wrapping_sub(interp_y),
            z: in_positions[i].z.wrapping_sub(interp_z),
        });
    }

    (control, out_deltas)
}
