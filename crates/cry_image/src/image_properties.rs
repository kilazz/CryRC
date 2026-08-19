use super::pixel_formats::EPixelFormat;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EColorModel {
    RGB,
    CIE,
    YCbCr,
    YFbFr,
    IRB,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EInputColorSpace {
    Linear,
    Srgb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EOutputColorSpace {
    Linear,
    Srgb,
    Auto,
}

#[derive(Debug, Clone)]
pub struct ReduceItem {
    pub platform_name: String,
    pub platform_index: i32,
    pub value: i32,
}

#[derive(Debug, Clone)]
pub struct ImageProperties {
    pub preset: String,
    pub pixel_format: Option<EPixelFormat>,
    pub input_color_space: EInputColorSpace,
    pub output_color_space: EOutputColorSpace,
    pub maintain_alpha_coverage: bool,
    pub mip_renormalize: bool,
    pub generate_mips: bool,
    pub gloss_from_normals: bool,
    pub discard_alpha: bool,
    pub normalize_range: bool,
    pub min_texture_size: usize,
    pub max_texture_size: usize,
    pub reduce_map: HashMap<String, ReduceItem>,
}

impl Default for ImageProperties {
    fn default() -> Self {
        Self {
            preset: "Albedo".to_string(),
            pixel_format: None,
            input_color_space: EInputColorSpace::Linear,
            output_color_space: EOutputColorSpace::Linear,
            maintain_alpha_coverage: false,
            mip_renormalize: false,
            generate_mips: true,
            gloss_from_normals: false,
            discard_alpha: false,
            normalize_range: false,
            min_texture_size: 0,
            max_texture_size: 0,
            reduce_map: HashMap::new(),
        }
    }
}

impl ImageProperties {
    pub fn parse_reduce_string(&mut self, text: &str) {
        self.reduce_map.clear();
        if text.contains(':') {
            for pair in text.split(',') {
                let parts: Vec<&str> = pair.split(':').collect();
                if parts.len() == 2 {
                    let platform = parts[0].trim().to_ascii_lowercase();
                    let val: i32 = parts[1].trim().parse().unwrap_or(0).clamp(-2, 5);
                    self.reduce_map.insert(
                        platform.clone(),
                        ReduceItem {
                            platform_name: platform,
                            platform_index: -1,
                            value: val,
                        },
                    );
                }
            }
        } else {
            let val: i32 = text.trim().parse().unwrap_or(0).clamp(-2, 5);
            self.reduce_map.insert(
                "pc".to_string(),
                ReduceItem {
                    platform_name: "pc".to_string(),
                    platform_index: 0,
                    value: val,
                },
            );
        }
    }

    pub fn get_resolution_reduce_for_platform(&self, platform: &str) -> usize {
        self.reduce_map
            .get(&platform.to_ascii_lowercase())
            .map(|r| r.value.max(0) as usize)
            .unwrap_or(0)
    }
}
