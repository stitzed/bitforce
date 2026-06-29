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
        assert!(index < 64, "index must be in 0..64 range");

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
