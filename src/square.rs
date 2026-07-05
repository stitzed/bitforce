use std::fmt::Display;
use std::ops::{Add, Sub};

use derive_more::Into;

use crate::errors::SquareParseError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Into)]
#[into(u8, u32, usize)]
#[repr(transparent)]
pub struct Square(u8);

impl Square {
    pub const SQUARE_MASK: u8 = 0b111111;

    #[inline(always)]
    pub const fn new(index: u8) -> Self {
        debug_assert!(index < 64, "index must be in 0..64 range");

        Self(index)
    }

    #[inline(always)]
    pub const fn from_coords(row: u8, col: u8) -> Self {
        assert!(row < 8 && col < 8, "coords must be in 0..8 range");

        Self((row * 8) + col)
    }

    #[inline(always)]
    pub const fn to_bitboard_mask(&self) -> u64 {
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let col_char: char = char::from_u32(('a' as u32) + self.col() as u32).unwrap_or('?');
        f.write_str(&format!("{}{}", col_char, self.row() + 1))
    }
}

impl std::str::FromStr for Square {
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

        Ok(Square::from_coords(row_index, col_index))
    }
}

impl Square {
    pub const A1: Self = Self::new(0);
    pub const B1: Self = Self::new(1);
    pub const C1: Self = Self::new(2);
    pub const D1: Self = Self::new(3);
    pub const E1: Self = Self::new(4);
    pub const F1: Self = Self::new(5);
    pub const G1: Self = Self::new(6);
    pub const H1: Self = Self::new(7);
    pub const A2: Self = Self::new(8);
    pub const B2: Self = Self::new(9);
    pub const C2: Self = Self::new(10);
    pub const D2: Self = Self::new(11);
    pub const E2: Self = Self::new(12);
    pub const F2: Self = Self::new(13);
    pub const G2: Self = Self::new(14);
    pub const H2: Self = Self::new(15);
    pub const A3: Self = Self::new(16);
    pub const B3: Self = Self::new(17);
    pub const C3: Self = Self::new(18);
    pub const D3: Self = Self::new(19);
    pub const E3: Self = Self::new(20);
    pub const F3: Self = Self::new(21);
    pub const G3: Self = Self::new(22);
    pub const H3: Self = Self::new(23);
    pub const A4: Self = Self::new(24);
    pub const B4: Self = Self::new(25);
    pub const C4: Self = Self::new(26);
    pub const D4: Self = Self::new(27);
    pub const E4: Self = Self::new(28);
    pub const F4: Self = Self::new(29);
    pub const G4: Self = Self::new(30);
    pub const H4: Self = Self::new(31);
    pub const A5: Self = Self::new(32);
    pub const B5: Self = Self::new(33);
    pub const C5: Self = Self::new(34);
    pub const D5: Self = Self::new(35);
    pub const E5: Self = Self::new(36);
    pub const F5: Self = Self::new(37);
    pub const G5: Self = Self::new(38);
    pub const H5: Self = Self::new(39);
    pub const A6: Self = Self::new(40);
    pub const B6: Self = Self::new(41);
    pub const C6: Self = Self::new(42);
    pub const D6: Self = Self::new(43);
    pub const E6: Self = Self::new(44);
    pub const F6: Self = Self::new(45);
    pub const G6: Self = Self::new(46);
    pub const H6: Self = Self::new(47);
    pub const A7: Self = Self::new(48);
    pub const B7: Self = Self::new(49);
    pub const C7: Self = Self::new(50);
    pub const D7: Self = Self::new(51);
    pub const E7: Self = Self::new(52);
    pub const F7: Self = Self::new(53);
    pub const G7: Self = Self::new(54);
    pub const H7: Self = Self::new(55);
    pub const A8: Self = Self::new(56);
    pub const B8: Self = Self::new(57);
    pub const C8: Self = Self::new(58);
    pub const D8: Self = Self::new(59);
    pub const E8: Self = Self::new(60);
    pub const F8: Self = Self::new(61);
    pub const G8: Self = Self::new(62);
    pub const H8: Self = Self::new(63);
}
