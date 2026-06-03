use std::ops::{Add, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Square(u8);

impl Square {
    pub const SQUARE_MASK: u8 = 0b111111;
    
    #[inline(always)]
    pub fn new(index: u8) -> Option<Self> {
        if index > 64 {
            return None;
        }
        
        Some(Self(index))
    }
    
    #[inline(always)]
    pub const fn new_unchecked(index: u8) -> Self {
        Self(index)
    }
    
    #[inline(always)]
    pub fn to_bitboard_mask(&self) -> u64 {
        1 << self.0
    }

    #[inline(always)]
    pub fn col(&self) -> u8 {
        self.0 & 7
    }

    #[inline(always)]
    pub fn row(&self) -> u8 {
        self.0 >> 3
    }

    #[inline(always)]
    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

impl Add<u8> for Square {
    type Output = Self;
    
    fn add(self, rhs: u8) -> Self::Output {
        Self(self.0.wrapping_add(rhs))
    }
}

impl Sub<u8> for Square {
    type Output = Self;

    fn sub(self, rhs: u8) -> Self::Output {
        Self(self.0.wrapping_sub(rhs))
    }
}