pub const IBL_ALPHA5: usize = 0;
pub const IBL_ALPHA7: usize = 1;
pub const IBL_COLOR3: usize = 2;
pub const IBL_COLOR4: usize = 3;

/// Index remapping tables when endpoints are inverted:
/// [0] Alpha 6-level (a > b)
/// [1] Alpha 8-level (a < b)
/// [2] Color 3-step (a > b)
/// [3] Color 4-step (a < b)
pub const LOOKUP_C34A57: [[u8; 8]; 4] = [
    [1, 0, 5, 4, 3, 2, 6, 7], // alpha5: (a > b)
    [1, 0, 7, 6, 5, 4, 3, 2], // alpha7: (a < b)
    [1, 0, 2, 3, 4, 5, 6, 7], // color3: (a > b)
    [1, 0, 3, 2, 5, 4, 7, 6], // color4: (a < b)
];

#[inline(always)]
pub fn remap_degenerate_index(lut_type: usize, index: u8) -> u8 {
    LOOKUP_C34A57[lut_type][(index & 0x07) as usize]
}
