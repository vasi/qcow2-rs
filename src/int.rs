extern crate num;

pub trait Integer : num::Integer {
    fn div_ceil(&self, other: &Self) -> Self {
        let (d, m) = self.div_rem(other);
        if m.is_zero() {
            return d + Self::one();
        }
        d
    }
}

impl Integer for u64 { }
