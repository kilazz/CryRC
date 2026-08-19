use super::types::{
    LODGenParams, LODSequenceOutput, Move, Poly, TakenMove, Vertex, ZSpan, ZSpanData,
};
use cry_core::math::{AABB, Matrix34, Vec3};
use rayon::prelude::*;
use std::f32::consts::PI;

pub struct VisualChangeCalculatorView {
    pub spans: Vec<ZSpan>,
    pub error: Vec<f32>,
    pub trans_vtx: Vec<Vec3>,
    pub mtx: Matrix34,
    pub width: usize,
    pub height: usize,
    pub far_plane: f32,
}

impl VisualChangeCalculatorView {
    pub fn new(
        moves: &[Move],
        polys: &[Poly],
        vertices: &[Vertex],
        view_dir: Vec3,
        meters_per_pixel: f32,
        silhouette_weight: f32,
    ) -> Self {
        let mut bb = AABB::default();
        bb.reset();
        for v in vertices {
            bb.add_point(v.pos);
        }

        let mut right_len = 0.0f32;
        let mut up_len = 0.0f32;
        let mtx =
            Self::create_view_matrix(view_dir, &bb, &mut right_len, &mut up_len, meters_per_pixel);
        let far_plane = 2.0
            * bb.get_size().x.max(bb.get_size().y).max(bb.get_size().z)
            * 0.5
            * (1.0 + silhouette_weight)
            / meters_per_pixel;

        let width = (right_len.ceil() as usize).max(1);
        let height = (up_len.ceil() as usize).max(1);

        let trans_vtx: Vec<Vec3> = vertices
            .iter()
            .map(|v| mtx.transform_point(&v.pos))
            .collect();
        let error = vec![0.0f32; moves.len()];
        let spans = vec![ZSpan::default(); width * height];

        let mut view = Self {
            spans,
            error,
            trans_vtx,
            mtx,
            width,
            height,
            far_plane,
        };
        view.full_render(polys, moves, vertices);
        view
    }

    fn create_view_matrix(
        direction: Vec3,
        bb: &AABB,
        right_len: &mut f32,
        up_len: &mut f32,
        distance_per_pixel: f32,
    ) -> Matrix34 {
        let mut up = Vec3::new(0.0, 0.0, 1.0);
        let right_test = Vec3::new(1.0, 0.0, 0.0);

        let dot_up = direction.dot(up);
        let dot_right = direction.dot(right_test);

        if dot_up.abs() > dot_right.abs() {
            up = right_test;
        }

        let mut right = up.cross(direction);
        let r_len = right.len().max(1e-6);
        right.x /= r_len;
        right.y /= r_len;
        right.z /= r_len;

        up = direction.cross(right);
        let u_len = up.len().max(1e-6);
        up.x /= u_len;
        up.y /= u_len;
        up.z /= u_len;

        let center = Vec3::new(
            (bb.min.x + bb.max.x) * 0.5,
            (bb.min.y + bb.max.y) * 0.5,
            (bb.min.z + bb.max.z) * 0.5,
        );
        let radius = (bb.max.x - bb.min.x)
            .max(bb.max.y - bb.min.y)
            .max(bb.max.z - bb.min.z)
            * 0.5;

        let mut r_max = 0.0f32;
        let mut u_max = 0.0f32;

        for i in 0..4 {
            let bx = if (i & 1) != 0 {
                bb.max.x - bb.min.x
            } else {
                bb.min.x - bb.max.x
            };
            let by = if (i & 2) != 0 {
                bb.max.y - bb.min.y
            } else {
                bb.min.y - bb.max.y
            };
            let bz = bb.max.z - bb.min.z;

            let dot_r = (right.x * bx + right.y * by + right.z * bz).abs();
            let dot_u = (up.x * bx + up.y * by + up.z * bz).abs();

            r_max = r_max.max(dot_r);
            u_max = u_max.max(dot_u);
        }

        *right_len = r_max / distance_per_pixel;
        *up_len = u_max / distance_per_pixel;

        let mut mtx = Matrix34::IDENTITY;
        mtx.m[0] = [
            right.x * (*right_len),
            right.y * (*right_len),
            right.z * (*right_len),
            -(center.x * right.x + center.y * right.y + center.z * right.z),
        ];
        mtx.m[1] = [
            up.x * (*up_len),
            up.y * (*up_len),
            up.z * (*up_len),
            -(center.x * up.x + center.y * up.y + center.z * up.z),
        ];
        mtx.m[2] = [
            direction.x / distance_per_pixel,
            direction.y / distance_per_pixel,
            direction.z / distance_per_pixel,
            radius / distance_per_pixel,
        ];

        mtx
    }

    pub fn full_render(&mut self, polys: &[Poly], moves: &[Move], vertices: &[Vertex]) {
        for span in &mut self.spans {
            span.data.push(ZSpanData {
                height: self.far_plane,
                move_id: 0,
                poly_id: 0,
            });
            span.reference = self.far_plane;
            span.error = 0.0;
        }

        for (i, p) in polys.iter().enumerate() {
            self.render_polygon(
                self.trans_vtx[p.v[0] as usize],
                self.trans_vtx[p.v[1] as usize],
                self.trans_vtx[p.v[2] as usize],
                i as u32,
                0,
            );
        }

        for (m_idx, m) in moves.iter().enumerate() {
            if m.from == m.to {
                continue;
            }
            let v = &vertices[m.from as usize];
            for &p_idx in &v.polys {
                let p = &polys[p_idx as usize];
                let v0 = if p.v[0] == m.from { m.to } else { p.v[0] };
                let v1 = if p.v[1] == m.from { m.to } else { p.v[1] };
                let v2 = if p.v[2] == m.from { m.to } else { p.v[2] };

                self.render_polygon(
                    self.trans_vtx[v0 as usize],
                    self.trans_vtx[v1 as usize],
                    self.trans_vtx[v2 as usize],
                    p_idx,
                    m_idx as u32,
                );
            }
        }

        for span in &mut self.spans {
            let mut h = 0.0;
            for d in &span.data {
                if d.move_id == 0 {
                    h = d.height;
                    break;
                }
            }
            span.reference = h;
            span.error = 0.0;
        }
    }

    fn render_polygon(&mut self, a: Vec3, b: Vec3, c: Vec3, poly_id: u32, move_id: u32) {
        let ab = Vec3::new(b.x - a.x, b.y - a.y, b.z - a.z);
        let ac = Vec3::new(c.x - a.x, c.y - a.y, c.z - a.z);
        let n = ab.cross(ac);

        if n.z < 0.0 {
            let min_x =
                (a.x.min(b.x.min(c.x)).floor() as isize).clamp(0, self.width as isize - 1) as usize;
            let max_x =
                (a.x.max(b.x.max(c.x)).ceil() as isize).clamp(0, self.width as isize - 1) as usize;
            let min_y = (a.y.min(b.y.min(c.y)).floor() as isize).clamp(0, self.height as isize - 1)
                as usize;
            let max_y =
                (a.y.max(b.y.max(c.y)).ceil() as isize).clamp(0, self.height as isize - 1) as usize;

            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let mut h = 0.0f32;
                    if Self::ray_hit(a, b, c, n, x as f32, y as f32, &mut h) {
                        let span = &mut self.spans[y * self.width + x];
                        Self::insert_into_span(span, h, move_id, poly_id);
                    }
                }
            }
        }
    }

    #[inline]
    fn ray_hit(a: Vec3, b: Vec3, c: Vec3, n: Vec3, x: f32, y: f32, out_h: &mut f32) -> bool {
        let px = x;
        let py = y;
        if (b.x - a.x) * (py - a.y) - (b.y - a.y) * (px - a.x) >= 0.0 {
            return false;
        }
        if (c.x - b.x) * (py - b.y) - (c.y - b.y) * (px - b.x) >= 0.0 {
            return false;
        }
        if (a.x - c.x) * (py - c.y) - (a.y - c.y) * (px - c.x) >= 0.0 {
            return false;
        }
        *out_h = (n.x * a.x + n.y * a.y + n.z * a.z - px * n.x - py * n.y) / n.z;
        *out_h >= 0.0
    }

    fn insert_into_span(span: &mut ZSpan, height: f32, move_id: u32, poly_id: u32) {
        let mut insert_pos = span.data.len();
        for (i, d) in span.data.iter().enumerate() {
            if move_id != 0 && d.move_id == move_id && d.height <= height {
                return;
            }
            if d.height >= height {
                insert_pos = i;
                break;
            }
        }

        span.data.insert(
            insert_pos,
            ZSpanData {
                height,
                move_id,
                poly_id,
            },
        );
        if move_id != 0 {
            span.data
                .retain(|d| !(d.move_id == move_id && d.height > height));
        }
    }
}

pub struct VisualChangeCalculator {
    pub params: LODGenParams,
    pub vertices: Vec<Vertex>,
    pub polys: Vec<Poly>,
    pub moves: Vec<Move>,
    pub original_indices: Vec<u32>,
    pub move_list: Vec<TakenMove>,
    pub views: Vec<VisualChangeCalculatorView>,
}

impl VisualChangeCalculator {
    pub fn new(params: LODGenParams) -> Self {
        Self {
            params,
            vertices: Vec::new(),
            polys: Vec::new(),
            moves: Vec::new(),
            original_indices: Vec::new(),
            move_list: Vec::new(),
            views: Vec::new(),
        }
    }

    pub fn load_mesh(&mut self, positions: &[Vec3], indices: &[u32]) {
        self.vertices.clear();
        self.polys.clear();
        self.moves.clear();
        self.move_list.clear();
        self.original_indices = indices.to_vec();

        let weld_sq = self.params.vertex_welding_distance.powi(2);

        let mut vert_remap = Vec::with_capacity(positions.len());
        for pos in positions {
            let mut found_idx = None;
            for (idx, v) in self.vertices.iter().enumerate() {
                let d_sq = (pos.x - v.pos.x).powi(2)
                    + (pos.y - v.pos.y).powi(2)
                    + (pos.z - v.pos.z).powi(2);
                if d_sq <= weld_sq {
                    found_idx = Some(idx as u32);
                    break;
                }
            }

            match found_idx {
                Some(idx) => vert_remap.push(idx),
                None => {
                    let idx = self.vertices.len() as u32;
                    self.vertices.push(Vertex {
                        pos: *pos,
                        polys: Vec::new(),
                    });
                    vert_remap.push(idx);
                }
            }
        }

        for chunk in indices.chunks(3) {
            if chunk.len() == 3 {
                let v0 = vert_remap[chunk[0] as usize];
                let v1 = vert_remap[chunk[1] as usize];
                let v2 = vert_remap[chunk[2] as usize];

                if v0 != v1 && v0 != v2 && v1 != v2 {
                    let poly_idx = self.polys.len() as u32;
                    self.vertices[v0 as usize].polys.push(poly_idx);
                    self.vertices[v1 as usize].polys.push(poly_idx);
                    self.vertices[v2 as usize].polys.push(poly_idx);

                    self.polys.push(Poly {
                        v: [v0, v1, v2],
                        moves: Vec::new(),
                    });
                }
            }
        }

        for (v_idx, vert) in self.vertices.iter().enumerate() {
            for &p_idx in &vert.polys {
                let poly = &self.polys[p_idx as usize];
                for &target in &poly.v {
                    if target != v_idx as u32 {
                        self.moves.push(Move {
                            from: v_idx as u32,
                            to: target,
                        });
                    }
                }
            }
        }
    }

    pub fn is_patch_nice(&self, from_idx: u32, to_idx: u32) -> bool {
        if !self.params.check_topology {
            return true;
        }
        let p_from = &self.vertices[from_idx as usize];
        let p_to = &self.vertices[to_idx as usize];

        let mut shared_neighbors = 0;
        for &p1 in &p_from.polys {
            for &p2 in &p_to.polys {
                if p1 == p2 {
                    shared_neighbors += 1;
                }
            }
        }
        shared_neighbors <= 2
    }

    pub fn setup_views(&mut self) {
        let num_elevations = if !self.params.object_has_base {
            self.params.view_elevations.div_ceil(2)
        } else {
            self.params.view_elevations
        };
        let num_angles = self.params.views_around;
        let phase_step = (2.0 * PI) / (num_angles as f32);
        let elevation_angle = PI / ((num_elevations + 1) as f32);

        let mut directions = Vec::new();
        for k in 0..num_elevations {
            let phase_change = (k as f32) * phase_step / (num_elevations as f32);
            let z_angle = ((k + 1) as f32) * elevation_angle;
            let sz = z_angle.sin();
            let cz = -z_angle.cos();

            for i in 0..num_angles {
                let phase = phase_change + (i as f32) * phase_step;
                directions.push(Vec3::new(sz * phase.sin(), sz * phase.cos(), cz));
            }
        }
        directions.push(Vec3::new(0.0, 0.0, -1.0));
        if self.params.object_has_base {
            directions.push(Vec3::new(0.0, 0.0, 1.0));
        }

        self.views = directions
            .into_par_iter()
            .map(|dir| {
                VisualChangeCalculatorView::new(
                    &self.moves,
                    &self.polys,
                    &self.vertices,
                    dir,
                    self.params.view_resolution,
                    self.params.silhouette_weight,
                )
            })
            .collect();
    }

    pub fn process(&mut self) -> LODSequenceOutput {
        self.setup_views();
        let mut active_moves = self.moves.clone();

        while let Some((best_idx, min_error)) = self.find_best_move(&active_moves) {
            let m = active_moves[best_idx];
            self.move_list.push(TakenMove {
                from: m.from,
                to: m.to,
                error: min_error,
            });

            active_moves[best_idx].from = active_moves[best_idx].to;
            for mv in &mut active_moves {
                if mv.from == m.from {
                    mv.from = m.to;
                }
                if mv.to == m.from {
                    mv.to = m.to;
                }
            }
        }

        LODSequenceOutput {
            positions: self.vertices.iter().map(|v| v.pos).collect(),
            indices: self.original_indices.clone(),
            move_list: self.move_list.clone(),
        }
    }

    fn find_best_move(&self, active_moves: &[Move]) -> Option<(usize, f32)> {
        active_moves
            .par_iter()
            .enumerate()
            .filter(|(_, m)| m.from != m.to && self.is_patch_nice(m.from, m.to))
            .map(|(idx, m)| {
                let p0 = self.vertices[m.from as usize].pos;
                let p1 = self.vertices[m.to as usize].pos;
                let edge_len =
                    ((p1.x - p0.x).powi(2) + (p1.y - p0.y).powi(2) + (p1.z - p0.z).powi(2)).sqrt();
                (idx, edge_len)
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    }
}
