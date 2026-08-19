use std::sync::LazyLock;

pub const BC7E_2SUBSET_CHECKERBOARD_PARTITION_INDEX: usize = 34;

pub const G_BC7_WEIGHTS2: [u32; 4] = [0, 21, 43, 64];
pub const G_BC7_WEIGHTS3: [u32; 8] = [0, 9, 18, 27, 37, 46, 55, 64];
pub const G_BC7_WEIGHTS4: [u32; 16] = [0, 4, 9, 13, 17, 21, 26, 30, 34, 38, 43, 47, 51, 55, 60, 64];

pub const PR_WEIGHT: f32 = (0.5 / (1.0 - 0.2126)) * (0.5 / (1.0 - 0.2126));
pub const PB_WEIGHT: f32 = (0.5 / (1.0 - 0.0722)) * (0.5 / (1.0 - 0.0722));

#[derive(Debug, Clone, Copy)]
pub struct BC7ModeInfo {
    pub num_subsets: usize,
    pub partition_bits: u32,
    pub rotation_bits: u32,
    pub index_selection_bits: u32,
    pub color_bits: u32,
    pub alpha_bits: u32,
    pub endpoint_pbits: u32,
    pub shared_pbits: u32,
    pub index_bits: u32,
    pub secondary_index_bits: u32,
}

pub const BC7_MODE_INFO: [BC7ModeInfo; 8] = [
    BC7ModeInfo {
        num_subsets: 3,
        partition_bits: 4,
        rotation_bits: 0,
        index_selection_bits: 0,
        color_bits: 4,
        alpha_bits: 0,
        endpoint_pbits: 1,
        shared_pbits: 0,
        index_bits: 3,
        secondary_index_bits: 0,
    },
    BC7ModeInfo {
        num_subsets: 2,
        partition_bits: 6,
        rotation_bits: 0,
        index_selection_bits: 0,
        color_bits: 6,
        alpha_bits: 0,
        endpoint_pbits: 0,
        shared_pbits: 1,
        index_bits: 3,
        secondary_index_bits: 0,
    },
    BC7ModeInfo {
        num_subsets: 3,
        partition_bits: 6,
        rotation_bits: 0,
        index_selection_bits: 0,
        color_bits: 5,
        alpha_bits: 0,
        endpoint_pbits: 0,
        shared_pbits: 0,
        index_bits: 2,
        secondary_index_bits: 0,
    },
    BC7ModeInfo {
        num_subsets: 2,
        partition_bits: 6,
        rotation_bits: 0,
        index_selection_bits: 0,
        color_bits: 7,
        alpha_bits: 0,
        endpoint_pbits: 1,
        shared_pbits: 0,
        index_bits: 2,
        secondary_index_bits: 0,
    },
    BC7ModeInfo {
        num_subsets: 1,
        partition_bits: 0,
        rotation_bits: 2,
        index_selection_bits: 1,
        color_bits: 5,
        alpha_bits: 6,
        endpoint_pbits: 0,
        shared_pbits: 0,
        index_bits: 2,
        secondary_index_bits: 3,
    },
    BC7ModeInfo {
        num_subsets: 1,
        partition_bits: 0,
        rotation_bits: 2,
        index_selection_bits: 0,
        color_bits: 7,
        alpha_bits: 8,
        endpoint_pbits: 0,
        shared_pbits: 0,
        index_bits: 2,
        secondary_index_bits: 2,
    },
    BC7ModeInfo {
        num_subsets: 1,
        partition_bits: 0,
        rotation_bits: 0,
        index_selection_bits: 0,
        color_bits: 7,
        alpha_bits: 7,
        endpoint_pbits: 1,
        shared_pbits: 0,
        index_bits: 4,
        secondary_index_bits: 0,
    },
    BC7ModeInfo {
        num_subsets: 2,
        partition_bits: 6,
        rotation_bits: 0,
        index_selection_bits: 0,
        color_bits: 5,
        alpha_bits: 5,
        endpoint_pbits: 1,
        shared_pbits: 0,
        index_bits: 2,
        secondary_index_bits: 0,
    },
];

#[derive(Clone, Copy, Default)]
pub struct EndpointErr {
    pub error: u16,
    pub lo: u8,
    pub hi: u8,
}

pub struct Bc7Tables {
    pub mode_6: [[[EndpointErr; 2]; 2]; 256],
}

impl Bc7Tables {
    #[allow(clippy::needless_range_loop)]
    pub fn init() -> Self {
        let mut mode_6 = [[[EndpointErr::default(); 2]; 2]; 256];

        for c in 0..256usize {
            for hp in 0..2usize {
                for lp in 0..2usize {
                    let mut best = EndpointErr {
                        error: u16::MAX,
                        lo: 0,
                        hi: 0,
                    };
                    for l in 0..128usize {
                        let low = (l << 1) | lp;
                        for h in 0..128usize {
                            let high = (h << 1) | hp;
                            let k = (low * (64 - G_BC7_WEIGHTS4[5] as usize)
                                + high * G_BC7_WEIGHTS4[5] as usize
                                + 32)
                                >> 6;
                            let err = (k as i32 - c as i32) * (k as i32 - c as i32);
                            if err < best.error as i32 {
                                best.error = err as u16;
                                best.lo = l as u8;
                                best.hi = h as u8;
                            }
                        }
                    }
                    mode_6[c][hp][lp] = best;
                }
            }
        }

        Self { mode_6 }
    }
}

pub static BC7_TABLES: LazyLock<Bc7Tables> = LazyLock::new(Bc7Tables::init);
