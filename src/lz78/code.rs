use std::ops::{Add, AddAssign, Sub};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Hash)]
pub(crate) struct Code(pub(crate) u16);

impl Code {
    pub fn min_bits(self) -> usize {
        let mut value = self.0;
        let mut bits = 0;

        let mut bit_test = 8;
        while bit_test > 0 {
            if value >> bit_test != 0 {
                bits += bit_test;
                value >>= bit_test;
            }
            bit_test >>= 1;
        }

        bits + value as usize
    }
}

impl AddAssign<u16> for Code {
    fn add_assign(&mut self, rhs: u16) {
        self.0 += rhs;
    }
}

impl Add<u16> for Code {
    type Output = Self;

    fn add(self, rhs: u16) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Sub<u16> for Code {
    type Output = Self;

    fn sub(self, rhs: u16) -> Self::Output {
        Self(self.0 - rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero() {
        assert_eq!(0, Code(0).min_bits());
    }

    #[test]
    fn one() {
        assert_eq!(1, Code(1).min_bits());
    }

    #[test]
    fn eight_bits() {
        assert_eq!(8, Code(255).min_bits());
        assert_eq!(8, Code(128).min_bits());
        assert_eq!(8, Code(192).min_bits());
    }
}
