// Divide and yield remainder.
pub fn div_rem(a: u64, b: u64) -> (u64, u64) {
    (a / b, a % b)
}

// Divide, rounding up.
pub fn div_ceil(a: u64, b: u64) -> u64 {
    let (d, m) = div_rem(a, b);
    if m == 0 {
        d
    } else {
        d + 1
    }
}

// Get the smallest number that, added to `a`, yields a multiple of `b`.
pub fn padding_to_multiple(a: u64, b: u64) -> usize {
    let m = a % b;
    let r = if m == 0 {
        m
    } else {
        b - m
    };
    r as usize
}

// Check if `a` is a multiple of `b`.
pub fn is_multiple_of(a: u64, b: u64) -> bool {
    a % b == 0
}
