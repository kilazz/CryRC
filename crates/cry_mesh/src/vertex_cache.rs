const CACHE_SIZE_MAX: usize = 32;
const VALENCE_BOOST_POWER: f32 = 0.5;
const VALENCE_BOOST_SCALE: f32 = 2.0;

#[derive(Clone, Copy, Default)]
struct VertexScoreData {
    cache_position: i32,
    score: f32,
    num_faces_active: u32,
}

pub struct ForsythOptimizer;

impl ForsythOptimizer {
    #[inline]
    fn calculate_vertex_score(cache_pos: i32, active_faces: u32) -> f32 {
        if active_faces == 0 {
            return -1.0;
        }

        let mut score = 0.0;
        if cache_pos >= 0 {
            if cache_pos < 3 {
                score = 0.75;
            } else if cache_pos < CACHE_SIZE_MAX as i32 {
                let scaler = 1.0 / ((CACHE_SIZE_MAX - 3) as f32);
                score = 1.0 - ((cache_pos - 3) as f32) * scaler;
                score = score.powf(1.5);
            }
        }

        let valence_boost = (active_faces as f32).powf(-VALENCE_BOOST_POWER);
        score += VALENCE_BOOST_SCALE * valence_boost;
        score
    }

    pub fn optimize_indices(indices: &[u32], vertex_count: usize) -> Vec<u32> {
        let num_faces = indices.len() / 3;
        let mut face_alive = vec![true; num_faces];
        let mut vertex_data = vec![VertexScoreData::default(); vertex_count];

        let mut vertex_face_map = vec![Vec::new(); vertex_count];
        for (f_idx, chunk) in indices.chunks_exact(3).enumerate() {
            for &v_idx in chunk {
                let v = v_idx as usize;
                if v < vertex_count {
                    vertex_face_map[v].push(f_idx as u32);
                    vertex_data[v].num_faces_active += 1;
                }
            }
        }

        for v_data in vertex_data[..vertex_count].iter_mut() {
            v_data.cache_position = -1;
            v_data.score = Self::calculate_vertex_score(-1, v_data.num_faces_active);
        }

        let mut lru_cache: Vec<u32> = Vec::with_capacity(CACHE_SIZE_MAX + 3);
        let mut optimized_indices = Vec::with_capacity(indices.len());
        let mut best_face_idx = 0usize;

        while optimized_indices.len() < indices.len() {
            let mut found_face = None;
            let mut best_score = -1.0f32;

            for &cached_v in &lru_cache {
                let v = cached_v as usize;
                if v < vertex_count {
                    for &f in &vertex_face_map[v] {
                        let f_idx = f as usize;
                        if face_alive[f_idx] {
                            let (i0, i1, i2) = (
                                indices[f_idx * 3] as usize,
                                indices[f_idx * 3 + 1] as usize,
                                indices[f_idx * 3 + 2] as usize,
                            );
                            let score = vertex_data[i0].score
                                + vertex_data[i1].score
                                + vertex_data[i2].score;
                            if score > best_score {
                                best_score = score;
                                found_face = Some(f_idx);
                            }
                        }
                    }
                }
            }

            if found_face.is_none() {
                while best_face_idx < num_faces {
                    if face_alive[best_face_idx] {
                        found_face = Some(best_face_idx);
                        break;
                    }
                    best_face_idx += 1;
                }
            }

            let next_face = match found_face {
                Some(f) => f,
                None => break,
            };

            face_alive[next_face] = false;
            let f_verts = [
                indices[next_face * 3],
                indices[next_face * 3 + 1],
                indices[next_face * 3 + 2],
            ];
            optimized_indices.extend_from_slice(&f_verts);

            for &v_idx in &f_verts {
                let v = v_idx as usize;
                if v < vertex_count {
                    vertex_data[v].num_faces_active =
                        vertex_data[v].num_faces_active.saturating_sub(1);
                }
            }

            for &v_idx in &f_verts {
                lru_cache.retain(|&x| x != v_idx);
                lru_cache.insert(0, v_idx);
            }

            lru_cache.truncate(CACHE_SIZE_MAX);
            for (pos, &v_idx) in lru_cache.iter().enumerate() {
                let v = v_idx as usize;
                if v < vertex_count {
                    vertex_data[v].cache_position = pos as i32;
                    vertex_data[v].score =
                        Self::calculate_vertex_score(pos as i32, vertex_data[v].num_faces_active);
                }
            }
        }

        optimized_indices
    }
}
