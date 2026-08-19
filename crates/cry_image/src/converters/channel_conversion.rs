use crate::color_types::ColorRGBA8;

pub struct ChannelConverter;

impl ChannelConverter {
    pub fn swizzle_pixels(pixels: &mut [ColorRGBA8], pattern: &str) {
        if pattern.is_empty() || pattern.eq_ignore_ascii_case("rgba") {
            return;
        }
        let chars: Vec<char> = pattern.to_ascii_lowercase().chars().collect();
        if chars.len() < 4 {
            return;
        }

        for p in pixels.iter_mut() {
            let orig = *p;
            let map_channel = |c: char| -> u8 {
                match c {
                    'r' => orig.r,
                    'g' => orig.g,
                    'b' => orig.b,
                    'a' => orig.a,
                    '0' => 0,
                    '1' => 255,
                    _ => 0,
                }
            };
            p.r = map_channel(chars[0]);
            p.g = map_channel(chars[1]);
            p.b = map_channel(chars[2]);
            p.a = map_channel(chars[3]);
        }
    }

    pub fn convert_l8_to_rgba8(l8_bytes: &[u8]) -> Vec<ColorRGBA8> {
        l8_bytes
            .iter()
            .map(|&l| ColorRGBA8::new(l, l, l, 255))
            .collect()
    }

    pub fn convert_rgba8_to_l8(pixels: &[ColorRGBA8]) -> Vec<u8> {
        pixels
            .iter()
            .map(|p| ((p.r as u32 * 39 + p.g as u32 * 50 + p.b as u32 * 11 + 50) / 100) as u8)
            .collect()
    }

    pub fn convert_a8l8_to_rgba8(a8l8_bytes: &[u8]) -> Vec<ColorRGBA8> {
        a8l8_bytes
            .chunks_exact(2)
            .map(|c| ColorRGBA8::new(c[0], c[0], c[0], c[1]))
            .collect()
    }
}
