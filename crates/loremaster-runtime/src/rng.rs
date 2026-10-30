/// SplitMix64: tiny, fast, and identical on every platform, so seeded
/// playthroughs replay exactly.
pub(crate) fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

pub(crate) fn below(state: &mut u64, n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    (next(state) % n as u64) as usize
}
