use super::pixel_formats::EPixelFormat;
use crate::math::vector::Vec4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ECubemap {
    No,
    Yes,
    UnknownYet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EAlphaContent {
    Absent,
    OnlyWhite,
    OnlyBlack,
    OnlyBlackAndWhite,
    Greyscale,
}

pub const EIF_CUBEMAP: u32 = 1 << 0;
pub const EIF_VOLUMETEXTURE: u32 = 1 << 1;
pub const EIF_DECAL: u32 = 1 << 2;
pub const EIF_SRGBREAD: u32 = 1 << 3;
pub const EIF_FILESINGLE: u32 = 1 << 4;
pub const EIF_ATTACHEDALPHA: u32 = 1 << 5;
pub const EIF_SPLITTED: u32 = 1 << 16;
pub const EIF_RENORMALIZEDTEXTURE: u32 = 1 << 18;

#[derive(Debug, Clone)]
pub struct MipLevel {
    pub width: usize,
    pub height: usize,
    pub row_count: usize,
    pub pitch: usize,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ImageObject {
    pub pixel_format: EPixelFormat,
    pub cubemap: ECubemap,
    pub mips: Vec<MipLevel>,
    pub min_color: Vec4,
    pub max_color: Vec4,
    pub average_brightness: f32,
    pub image_flags: u32,
    pub num_persistent_mips: usize,
    pub compressed_block_width: usize,
    pub compressed_block_height: usize,
    pub attached_image: Option<Box<ImageObject>>,
}

impl ImageObject {
    pub fn new(
        _width: usize,
        _height: usize,
        _max_mips: usize,
        pixel_format: EPixelFormat,
        cubemap: ECubemap,
    ) -> Self {
        Self {
            pixel_format,
            cubemap,
            mips: Vec::new(),
            min_color: Vec4::new(0.0, 0.0, 0.0, 0.0),
            max_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            average_brightness: 0.0,
            image_flags: if cubemap == ECubemap::Yes {
                EIF_CUBEMAP
            } else {
                0
            },
            num_persistent_mips: 0,
            compressed_block_width: 4,
            compressed_block_height: 4,
            attached_image: None,
        }
    }

    pub fn get_extent(&self) -> (usize, usize, usize) {
        if self.mips.is_empty() {
            (0, 0, 0)
        } else {
            (self.mips[0].width, self.mips[0].height, self.mips.len())
        }
    }
}
