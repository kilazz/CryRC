/// Precomputed least-squares initialization candidates for 4-cluster unweighted BC1 search.
pub const PART1_INITS_16: [[f32; 3]; 16] = [
    [0.25, 15.25, 0.25],
    [1.00, 15.00, 0.00],
    [2.00, 14.00, 0.00],
    [3.00, 13.00, 0.00],
    [4.00, 12.00, 0.00],
    [5.00, 11.00, 0.00],
    [6.00, 10.00, 0.00],
    [7.00, 9.00, 0.00],
    [8.00, 8.00, 0.00],
    [9.00, 7.00, 0.00],
    [10.00, 6.00, 0.00],
    [11.00, 5.00, 0.00],
    [12.00, 4.00, 0.00],
    [13.00, 3.00, 0.00],
    [14.00, 2.00, 0.00],
    [15.00, 1.00, 0.00],
];

pub const PART1_DELTA: [f32; 3] = [0.25, -0.75, 0.25];

/// Least-squares step deltas for 4-cluster search in BC7 / CTX1.
pub const PART2_DELTA: [f32; 3] = [1.0 / 9.0, -5.0 / 9.0, 2.0 / 9.0];

pub const PART2_INITS_COUNT: usize = 152;
