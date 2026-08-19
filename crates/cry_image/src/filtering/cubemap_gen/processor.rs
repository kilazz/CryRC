use super::cubemap_topology::CubeMapTopology;
use super::edge_fixup::EdgeFixup;
use super::image_surface::ImageSurface;
use super::importance_sampling::ImportanceSampling;
use crate::math::vector::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CubemapFilterType {
    Cosine,
    GGX,
}

pub struct CubeMapProcessor {
    pub input_surfaces: [ImageSurface; 6],
    pub output_mips: Vec<[ImageSurface; 6]>,
    pub num_channels: usize,
}

impl CubeMapProcessor {
    pub fn new(input_size: usize, num_channels: usize) -> Self {
        let input_surfaces =
            core::array::from_fn(|_| ImageSurface::new(input_size, input_size, num_channels));
        Self {
            input_surfaces,
            output_mips: Vec::new(),
            num_channels,
        }
    }

    pub fn filter_cubemap_mipchain(
        &mut self,
        output_size: usize,
        num_mips: usize,
        _filter_type: CubemapFilterType,
        sample_count_ggx: usize,
        fixup_width: usize,
    ) {
        let mut cur_size = output_size;
        for mip_idx in 0..num_mips {
            let mut mip_surfaces =
                core::array::from_fn(|_| ImageSurface::new(cur_size, cur_size, self.num_channels));
            let roughness = (mip_idx as f32 / (num_mips.max(2) - 1) as f32).powi(2);

            for (face_idx, dst_surface) in mip_surfaces.iter_mut().enumerate() {
                for v in 0..cur_size {
                    for u in 0..cur_size {
                        let normal = CubeMapTopology::texel_coord_to_vect(
                            face_idx, u as f32, v as f32, cur_size,
                        );
                        let mut accum = [0.0f32; 4];
                        let mut total_w = 0.0f32;

                        for i in 0..sample_count_ggx {
                            let xi = ImportanceSampling::hammersley_sequence(i, sample_count_ggx);
                            let h =
                                ImportanceSampling::importance_sample_ggx(xi, roughness, normal);
                            let v_dot_h = normal.dot(&h);
                            let l = Vec3::new(
                                2.0 * v_dot_h * h.x - normal.x,
                                2.0 * v_dot_h * h.y - normal.y,
                                2.0 * v_dot_h * h.z - normal.z,
                            );

                            let n_dot_l = normal.dot(&l).max(0.0);
                            if n_dot_l > 0.0 {
                                let (s_face, su, sv) = CubeMapTopology::vect_to_texel_coord(
                                    l,
                                    self.input_surfaces[0].width,
                                );
                                let pix = self.input_surfaces[s_face].get_pixel(su, sv);
                                for (acc, &p) in accum[..self.num_channels]
                                    .iter_mut()
                                    .zip(&pix[..self.num_channels])
                                {
                                    *acc += p * n_dot_l;
                                }
                                total_w += n_dot_l;
                            }
                        }

                        let off = (v * cur_size + u) * self.num_channels;
                        if total_w > 0.0 {
                            for (c, &acc) in accum[..self.num_channels].iter().enumerate() {
                                dst_surface.data[off + c] = acc / total_w;
                            }
                        }
                    }
                }
            }

            EdgeFixup::fixup_cube_edges(&mut mip_surfaces, fixup_width);
            self.output_mips.push(mip_surfaces);
            cur_size = (cur_size / 2).max(1);
        }
    }
}
