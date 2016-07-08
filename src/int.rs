use num;

pub trait Integer: num::Integer + Copy {
    // Divide, rounding up.
    fn div_ceil(self, other: Self) -> Self {
        let (d, m) = self.div_rem(&other);
        if m.is_zero() {
            return d + Self::one();
        }
        d
    }

    // Get the next multiple of a number equal to or higher than self.
    fn to_multiple_of(self, other: Self) -> Self {
        let m = self % other;
        if m.is_zero() {
            return self;
        }
        self + (other - m)
    }

    // Get the amount to add to get a multiple of `other`.
    fn padding_to_multiple(self, other: Self) -> Self {
        let m = self % other;
        if m.is_zero() {
            return Self::zero();
        }
        other - m
    }
}

impl Integer for u64 {}
impl Integer for usize {}
