// Copyright 2011-2026 Morten S. Mikkelsen / Crytek GmbH. All rights reserved.

use cry_core::math::Vec3;
use std::f32::consts::PI;

pub trait MikkTSpaceMesh {
    fn get_num_faces(&self) -> usize;
    fn get_num_vertices_of_face(&self, face: usize) -> usize;
    fn get_position(&self, face: usize, vert: usize) -> Vec3;
    fn get_normal(&self, face: usize, vert: usize) -> Vec3;
    fn get_tex_coord(&self, face: usize, vert: usize) -> [f32; 2];
    #[allow(clippy::too_many_arguments)]
    fn set_tspace(
        &mut self,
        tangent: Vec3,
        bitangent: Vec3,
        mag_s: f32,
        mag_t: f32,
        is_orientation_preserving: bool,
        face: usize,
        vert: usize,
    );
}

const MARK_DEGENERATE: u32 = 1 << 0;
const GROUP_WITH_ANY: u32 = 1 << 2;
const ORIENT_PRESERVING: u32 = 1 << 3;

#[derive(Debug, Clone, Default)]
struct STriInfo {
    assigned_group: [Option<usize>; 3],
    v_os: Vec3,
    v_ot: Vec3,
    mag_s: f32,
    mag_t: f32,
    org_face_number: usize,
    flag: u32,
    tspaces_offset: usize,
    vert_num: [usize; 4],
}

#[derive(Debug, Clone, Default)]
struct STSpace {
    v_os: Vec3,
    mag_s: f32,
    v_ot: Vec3,
    mag_t: f32,
    orient: bool,
}

#[derive(Debug, Clone, Default)]
struct SGroup {
    face_indices: Vec<usize>,
    orient_preserving: bool,
}

pub struct MikkTSpaceGenerator;

impl MikkTSpaceGenerator {
    pub fn gen_tang_space_default<M: MikkTSpaceMesh>(mesh: &mut M) -> bool {
        Self::gen_tang_space(mesh, 180.0)
    }

    pub fn gen_tang_space<M: MikkTSpaceMesh>(mesh: &mut M, angular_threshold_deg: f32) -> bool {
        let num_faces = mesh.get_num_faces();
        if num_faces == 0 {
            return false;
        }

        let mut num_triangles = 0;
        for f in 0..num_faces {
            let verts = mesh.get_num_vertices_of_face(f);
            if verts == 3 {
                num_triangles += 1;
            } else if verts == 4 {
                num_triangles += 2;
            }
        }
        if num_triangles == 0 {
            return false;
        }

        let mut tri_infos = vec![STriInfo::default(); num_triangles];
        let mut tri_list = vec![0usize; num_triangles * 3];

        let num_tspaces = Self::generate_initial_index_list(&mut tri_infos, &mut tri_list, mesh);
        Self::generate_shared_vertices_index_list(&mut tri_list, mesh, num_triangles);

        let tot_tris = num_triangles;
        let mut degen_triangles = 0;

        for t in 0..tot_tris {
            let p0 = mesh.get_position(tri_list[t * 3] >> 2, tri_list[t * 3] & 3);
            let p1 = mesh.get_position(tri_list[t * 3 + 1] >> 2, tri_list[t * 3 + 1] & 3);
            let p2 = mesh.get_position(tri_list[t * 3 + 2] >> 2, tri_list[t * 3 + 2] & 3);

            if veq(p0, p1) || veq(p0, p2) || veq(p1, p2) {
                tri_infos[t].flag |= MARK_DEGENERATE;
                degen_triangles += 1;
            }
        }

        let valid_triangles = tot_tris - degen_triangles;
        Self::degen_prologue(&mut tri_infos, &mut tri_list, valid_triangles, tot_tris);
        Self::init_tri_info(&mut tri_infos, &tri_list, mesh, valid_triangles);

        let mut groups = Vec::new();
        Self::build_4rule_groups(&mut tri_infos, &mut groups, valid_triangles);

        let mut tspaces = vec![
            STSpace {
                v_os: Vec3::new(1.0, 0.0, 0.0),
                mag_s: 1.0,
                v_ot: Vec3::new(0.0, 1.0, 0.0),
                mag_t: 1.0,
                orient: true,
            };
            num_tspaces
        ];

        let thres_cos = (angular_threshold_deg * PI / 180.0).cos();
        Self::generate_tspaces(
            &mut tspaces,
            &tri_infos,
            &groups,
            &tri_list,
            thres_cos,
            mesh,
        );
        Self::degen_epilogue(
            &mut tspaces,
            &tri_infos,
            &tri_list,
            mesh,
            valid_triangles,
            tot_tris,
        );

        let mut index = 0;
        for f in 0..num_faces {
            let verts = mesh.get_num_vertices_of_face(f);
            if verts != 3 && verts != 4 {
                continue;
            }

            for i in 0..verts {
                let ts = &tspaces[index];
                mesh.set_tspace(ts.v_os, ts.v_ot, ts.mag_s, ts.mag_t, ts.orient, f, i);
                index += 1;
            }
        }

        true
    }

    fn generate_initial_index_list<M: MikkTSpaceMesh>(
        tri_infos: &mut [STriInfo],
        tri_list: &mut [usize],
        mesh: &M,
    ) -> usize {
        let mut tspaces_offset = 0;
        let mut dst_tri = 0;

        for f in 0..mesh.get_num_faces() {
            let verts = mesh.get_num_vertices_of_face(f);
            if verts != 3 && verts != 4 {
                continue;
            }

            tri_infos[dst_tri].org_face_number = f;
            tri_infos[dst_tri].tspaces_offset = tspaces_offset;

            if verts == 3 {
                tri_infos[dst_tri].vert_num = [0, 1, 2, 0];
                tri_list[dst_tri * 3] = f << 2;
                tri_list[dst_tri * 3 + 1] = (f << 2) | 1;
                tri_list[dst_tri * 3 + 2] = (f << 2) | 2;
                dst_tri += 1;
            } else {
                tri_infos[dst_tri + 1].org_face_number = f;
                tri_infos[dst_tri + 1].tspaces_offset = tspaces_offset;

                let t0 = mesh.get_tex_coord(f, 0);
                let t1 = mesh.get_tex_coord(f, 1);
                let t2 = mesh.get_tex_coord(f, 2);
                let t3 = mesh.get_tex_coord(f, 3);

                let dist_02 = (t2[0] - t0[0]).powi(2) + (t2[1] - t0[1]).powi(2);
                let dist_13 = (t3[0] - t1[0]).powi(2) + (t3[1] - t1[1]).powi(2);

                if dist_02 < dist_13 {
                    tri_infos[dst_tri].vert_num = [0, 1, 2, 0];
                    tri_list[dst_tri * 3] = f << 2;
                    tri_list[dst_tri * 3 + 1] = (f << 2) | 1;
                    tri_list[dst_tri * 3 + 2] = (f << 2) | 2;
                    dst_tri += 1;

                    tri_infos[dst_tri].vert_num = [0, 2, 3, 0];
                    tri_list[dst_tri * 3] = f << 2;
                    tri_list[dst_tri * 3 + 1] = (f << 2) | 2;
                    tri_list[dst_tri * 3 + 2] = (f << 2) | 3;
                    dst_tri += 1;
                } else {
                    tri_infos[dst_tri].vert_num = [0, 1, 3, 0];
                    tri_list[dst_tri * 3] = f << 2;
                    tri_list[dst_tri * 3 + 1] = (f << 2) | 1;
                    tri_list[dst_tri * 3 + 2] = (f << 2) | 3;
                    dst_tri += 1;

                    tri_infos[dst_tri].vert_num = [1, 2, 3, 0];
                    tri_list[dst_tri * 3] = (f << 2) | 1;
                    tri_list[dst_tri * 3 + 1] = (f << 2) | 2;
                    tri_list[dst_tri * 3 + 2] = (f << 2) | 3;
                    dst_tri += 1;
                }
            }

            tspaces_offset += verts;
        }

        tspaces_offset
    }

    fn generate_shared_vertices_index_list<M: MikkTSpaceMesh>(
        tri_list: &mut [usize],
        mesh: &M,
        num_triangles: usize,
    ) {
        let total_indices = num_triangles * 3;
        for i in 0..total_indices {
            let idx_a = tri_list[i];
            let p_a = mesh.get_position(idx_a >> 2, idx_a & 3);
            let n_a = mesh.get_normal(idx_a >> 2, idx_a & 3);
            let uv_a = mesh.get_tex_coord(idx_a >> 2, idx_a & 3);

            for j in 0..i {
                let idx_b = tri_list[j];
                let p_b = mesh.get_position(idx_b >> 2, idx_b & 3);
                let n_b = mesh.get_normal(idx_b >> 2, idx_b & 3);
                let uv_b = mesh.get_tex_coord(idx_b >> 2, idx_b & 3);

                if veq(p_a, p_b) && veq(n_a, n_b) && uv_a == uv_b {
                    tri_list[i] = idx_b;
                    break;
                }
            }
        }
    }

    fn degen_prologue(
        tri_infos: &mut [STriInfo],
        tri_list: &mut [usize],
        valid_triangles: usize,
        tot_tris: usize,
    ) {
        let mut t0 = 0;
        let mut t1 = 1;

        while t0 < valid_triangles {
            if (tri_infos[t0].flag & MARK_DEGENERATE) != 0 {
                while t1 < tot_tris && (tri_infos[t1].flag & MARK_DEGENERATE) != 0 {
                    t1 += 1;
                }
                if t1 < tot_tris {
                    for i in 0..3 {
                        tri_list.swap(t0 * 3 + i, t1 * 3 + i);
                    }
                    tri_infos.swap(t0, t1);
                }
            }
            t0 += 1;
            t1 = t1.max(t0 + 1);
        }
    }

    fn init_tri_info<M: MikkTSpaceMesh>(
        tri_infos: &mut [STriInfo],
        tri_list: &[usize],
        mesh: &M,
        valid_triangles: usize,
    ) {
        for f in 0..valid_triangles {
            let p0 = mesh.get_position(tri_list[f * 3] >> 2, tri_list[f * 3] & 3);
            let p1 = mesh.get_position(tri_list[f * 3 + 1] >> 2, tri_list[f * 3 + 1] & 3);
            let p2 = mesh.get_position(tri_list[f * 3 + 2] >> 2, tri_list[f * 3 + 2] & 3);

            let uv0 = mesh.get_tex_coord(tri_list[f * 3] >> 2, tri_list[f * 3] & 3);
            let uv1 = mesh.get_tex_coord(tri_list[f * 3 + 1] >> 2, tri_list[f * 3 + 1] & 3);
            let uv2 = mesh.get_tex_coord(tri_list[f * 3 + 2] >> 2, tri_list[f * 3 + 2] & 3);

            let d1 = vsub(p1, p0);
            let d2 = vsub(p2, p0);
            let t21x = uv1[0] - uv0[0];
            let t21y = uv1[1] - uv0[1];
            let t31x = uv2[0] - uv0[0];
            let t31y = uv2[1] - uv0[1];

            let signed_area = t21x * t31y - t21y * t31x;
            let v_os = vsub(vscale(t31y, d1), vscale(t21y, d2));
            let v_ot = vadd(vscale(-t31x, d1), vscale(t21x, d2));

            if signed_area > 0.0 {
                tri_infos[f].flag |= ORIENT_PRESERVING;
            }

            let len_s = vlen(v_os);
            let len_t = vlen(v_ot);

            if signed_area.abs() > 1e-7 && len_s > 1e-7 && len_t > 1e-7 {
                let sign = if (tri_infos[f].flag & ORIENT_PRESERVING) == 0 {
                    -1.0
                } else {
                    1.0
                };
                tri_infos[f].v_os = vscale(sign / len_s, v_os);
                tri_infos[f].v_ot = vscale(sign / len_t, v_ot);
                tri_infos[f].mag_s = len_s / signed_area.abs();
                tri_infos[f].mag_t = len_t / signed_area.abs();
            } else {
                tri_infos[f].flag |= GROUP_WITH_ANY;
            }
        }
    }

    fn build_4rule_groups(
        tri_infos: &mut [STriInfo],
        groups: &mut Vec<SGroup>,
        valid_triangles: usize,
    ) {
        for (f, tri_info) in tri_infos[..valid_triangles].iter_mut().enumerate() {
            for i in 0..3 {
                if (tri_info.flag & GROUP_WITH_ANY) == 0 && tri_info.assigned_group[i].is_none() {
                    let group_idx = groups.len();
                    tri_info.assigned_group[i] = Some(group_idx);

                    let grp = SGroup {
                        face_indices: vec![f],
                        orient_preserving: (tri_info.flag & ORIENT_PRESERVING) != 0,
                    };
                    groups.push(grp);
                }
            }
        }
    }

    fn generate_tspaces<M: MikkTSpaceMesh>(
        tspaces: &mut [STSpace],
        tri_infos: &[STriInfo],
        groups: &[SGroup],
        tri_list: &[usize],
        _thres_cos: f32,
        mesh: &M,
    ) {
        for (g_idx, group) in groups.iter().enumerate() {
            for &f in &group.face_indices {
                for i in 0..3 {
                    if tri_infos[f].assigned_group[i] == Some(g_idx) {
                        let offset = tri_infos[f].tspaces_offset;
                        let vert = tri_infos[f].vert_num[i];
                        let ts_idx = offset + vert;

                        let norm =
                            mesh.get_normal(tri_list[f * 3 + i] >> 2, tri_list[f * 3 + i] & 3);
                        let mut v_os = vsub(
                            tri_infos[f].v_os,
                            vscale(vdot(norm, tri_infos[f].v_os), norm),
                        );
                        let mut v_ot = vsub(
                            tri_infos[f].v_ot,
                            vscale(vdot(norm, tri_infos[f].v_ot), norm),
                        );

                        if vlen(v_os) > 1e-6 {
                            v_os = vnormalize(v_os);
                        }
                        if vlen(v_ot) > 1e-6 {
                            v_ot = vnormalize(v_ot);
                        }

                        tspaces[ts_idx].v_os = v_os;
                        tspaces[ts_idx].v_ot = v_ot;
                        tspaces[ts_idx].mag_s = tri_infos[f].mag_s;
                        tspaces[ts_idx].mag_t = tri_infos[f].mag_t;
                        tspaces[ts_idx].orient = group.orient_preserving;
                    }
                }
            }
        }
    }

    fn degen_epilogue<M: MikkTSpaceMesh>(
        tspaces: &mut [STSpace],
        tri_infos: &[STriInfo],
        tri_list: &[usize],
        _mesh: &M,
        valid_triangles: usize,
        tot_tris: usize,
    ) {
        if tot_tris > valid_triangles {
            for t in valid_triangles..tot_tris {
                for i in 0..3 {
                    let idx = tri_list[t * 3 + i];
                    for good_t in 0..valid_triangles {
                        for j in 0..3 {
                            if tri_list[good_t * 3 + j] == idx {
                                let src = tri_infos[good_t].tspaces_offset
                                    + tri_infos[good_t].vert_num[j];
                                let dst = tri_infos[t].tspaces_offset + tri_infos[t].vert_num[i];
                                tspaces[dst] = tspaces[src].clone();
                            }
                        }
                    }
                }
            }
        }
    }
}

#[inline]
fn veq(a: Vec3, b: Vec3) -> bool {
    (a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6 && (a.z - b.z).abs() < 1e-6
}
#[inline]
fn vadd(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
#[inline]
fn vsub(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
#[inline]
fn vscale(s: f32, v: Vec3) -> Vec3 {
    Vec3::new(v.x * s, v.y * s, v.z * s)
}
#[inline]
fn vdot(a: Vec3, b: Vec3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}
#[inline]
fn vlen(v: Vec3) -> f32 {
    (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
}
#[inline]
fn vnormalize(v: Vec3) -> Vec3 {
    let l = vlen(v);
    if l > 1e-6 {
        vscale(1.0 / l, v)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    }
}
