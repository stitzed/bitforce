use core::fmt::Display;
use core::ops::{Add, Sub};
use core::str::FromStr;

use derive_more::Into;

use crate::errors::SquareParseError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Into)]
#[into(u8, u32, usize)]
#[repr(transparent)]
pub struct Square(u8);

impl Square {
    pub const SQUARE_MASK: u8 = 0b111111;

    #[inline(always)]
    pub const fn new(index: u8) -> Option<Self> {
        if index < 64 { Some(Self(index)) } else { None }
    }

    /// # Safety
    /// `index` must be in 0..64 range
    #[inline(always)]
    pub const unsafe fn new_unchecked(index: u8) -> Self {
        Self(index)
    }

    #[inline(always)]
    pub const fn from_coords(row: u8, col: u8) -> Option<Self> {
        if row >= 8 || col >= 8 {
            None
        } else {
            Some(Self((row * 8) + col))
        }
    }

    /// # Safety
    /// `row` and `col` must be in 0..8 range
    #[inline(always)]
    pub const unsafe fn from_coords_unchecked(row: u8, col: u8) -> Self {
        Self((row * 8) + col)
    }

    #[inline(always)]
    pub const fn bitboard_mask(&self) -> u64 {
        1 << self.0
    }

    #[inline(always)]
    pub const fn col(&self) -> u8 {
        self.0 & 7
    }

    #[inline(always)]
    pub const fn row(&self) -> u8 {
        self.0 >> 3
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

impl Display for Square {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let col_char: char = char::from_u32(('a' as u32) + self.col() as u32).unwrap_or('?');
        write!(f, "{col_char}")?;
        write!(f, "{}", self.row() + 1)?;
        Ok(())
    }
}

impl FromStr for Square {
    type Err = SquareParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();

        let col: char = chars.next().ok_or(SquareParseError::InvalidLength(0))?;
        let row: char = chars.next().ok_or(SquareParseError::InvalidLength(1))?;

        if chars.next().is_some() {
            return Err(SquareParseError::InvalidLength(s.chars().count()));
        }

        if !('a'..='h').contains(&col) {
            return Err(SquareParseError::InvalidColumnChar(col));
        }

        let col_index: u8 = (col as u8) - b'a';

        if !('1'..='8').contains(&row) {
            return Err(SquareParseError::InvalidRowChar(row));
        }

        let row_index: u8 = (row as u8) - b'1';

        Ok(unsafe { Square::from_coords_unchecked(row_index, col_index) })
    }
}

impl Square {
    pub const A1: Self = Self(0);
    pub const B1: Self = Self(1);
    pub const C1: Self = Self(2);
    pub const D1: Self = Self(3);
    pub const E1: Self = Self(4);
    pub const F1: Self = Self(5);
    pub const G1: Self = Self(6);
    pub const H1: Self = Self(7);
    pub const A2: Self = Self(8);
    pub const B2: Self = Self(9);
    pub const C2: Self = Self(10);
    pub const D2: Self = Self(11);
    pub const E2: Self = Self(12);
    pub const F2: Self = Self(13);
    pub const G2: Self = Self(14);
    pub const H2: Self = Self(15);
    pub const A3: Self = Self(16);
    pub const B3: Self = Self(17);
    pub const C3: Self = Self(18);
    pub const D3: Self = Self(19);
    pub const E3: Self = Self(20);
    pub const F3: Self = Self(21);
    pub const G3: Self = Self(22);
    pub const H3: Self = Self(23);
    pub const A4: Self = Self(24);
    pub const B4: Self = Self(25);
    pub const C4: Self = Self(26);
    pub const D4: Self = Self(27);
    pub const E4: Self = Self(28);
    pub const F4: Self = Self(29);
    pub const G4: Self = Self(30);
    pub const H4: Self = Self(31);
    pub const A5: Self = Self(32);
    pub const B5: Self = Self(33);
    pub const C5: Self = Self(34);
    pub const D5: Self = Self(35);
    pub const E5: Self = Self(36);
    pub const F5: Self = Self(37);
    pub const G5: Self = Self(38);
    pub const H5: Self = Self(39);
    pub const A6: Self = Self(40);
    pub const B6: Self = Self(41);
    pub const C6: Self = Self(42);
    pub const D6: Self = Self(43);
    pub const E6: Self = Self(44);
    pub const F6: Self = Self(45);
    pub const G6: Self = Self(46);
    pub const H6: Self = Self(47);
    pub const A7: Self = Self(48);
    pub const B7: Self = Self(49);
    pub const C7: Self = Self(50);
    pub const D7: Self = Self(51);
    pub const E7: Self = Self(52);
    pub const F7: Self = Self(53);
    pub const G7: Self = Self(54);
    pub const H7: Self = Self(55);
    pub const A8: Self = Self(56);
    pub const B8: Self = Self(57);
    pub const C8: Self = Self(58);
    pub const D8: Self = Self(59);
    pub const E8: Self = Self(60);
    pub const F8: Self = Self(61);
    pub const G8: Self = Self(62);
    pub const H8: Self = Self(63);
}
