use core::ops::Not;

use crate::bitboard::{Bitboard, RANK_1, RANK_3, RANK_6, RANK_8};
use crate::square::Square;

/// Bit layout:
/// `[4 free][1 color][3 kind]`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Piece(u8);

impl Piece {
    #[inline(always)]
    pub fn new(color: Color, kind: Kind) -> Self {
        Self(((color as u8) << 3) | (kind as u8))
    }

    #[inline(always)]
    pub fn color(&self) -> Color {
        Color::ALL_COLORS[((self.0 >> 3) & 1) as usize]
    }

    #[inline(always)]
    pub fn kind(&self) -> Kind {
        unsafe { *Kind::ALL_KINDS.get_unchecked((self.0 & Kind::KIND_MASK) as usize) }
    }

    #[inline(always)]
    pub fn to_index(&self) -> usize {
        (self.color() as usize * 6) + (self.kind() as usize)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl Kind {
    pub const NONE_VALUE: u8 = 6;

    pub const ALL_KINDS: [Self; 6] = [
        Self::Pawn,
        Self::Knight,
        Self::Bishop,
        Self::Rook,
        Self::Queen,
        Self::King,
    ];

    pub const PROMOTION_KINDS: [Self; 4] = [Self::Knight, Self::Bishop, Self::Rook, Self::Queen];

    const START_ROW_MASKS: [u64; 6] = [
        0b1111_1111,
        0b0100_0010,
        0b0010_0100,
        0b1000_0001,
        0b0000_1000,
        0b0001_0000,
    ];

    pub const KIND_MASK: u8 = 0b111;

    #[inline(always)]
    pub fn from_index(index: u8) -> Option<Self> {
        Self::ALL_KINDS.get(usize::from(index)).copied()
    }

    #[inline(always)]
    pub fn start_row_mask(&self) -> u64 {
        Self::START_ROW_MASKS[*self as usize]
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub const ALL_COLORS: [Self; 2] = [Self::White, Self::Black];

    pub const fn shift(&self) -> u8 {
        (8 * 7) * (*self as u8)
    }
}

impl Not for Color {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
}

pub trait PawnDirections {
    const COLOR: Color;
    const ONCE_PUSH_RANK: Bitboard;
    const PROMOTION_RANK: Bitboard;
    const LEFT_ATTACKS_OFFSET: u8;
    const RIGHT_ATTACKS_OFFSET: u8;

    fn shift_bitboard(bitboard: Bitboard, value: u8) -> Bitboard;
    fn offest_square(square: Square, offset: u8) -> Square;
}

pub struct WhitePawnDirections;

impl PawnDirections for WhitePawnDirections {
    const COLOR: Color = Color::White;
    const ONCE_PUSH_RANK: Bitboard = RANK_3;
    const PROMOTION_RANK: Bitboard = RANK_8;
    const LEFT_ATTACKS_OFFSET: u8 = 7;
    const RIGHT_ATTACKS_OFFSET: u8 = 9;

    #[inline(always)]
    fn shift_bitboard(bitboard: Bitboard, value: u8) -> Bitboard {
        bitboard << value
    }

    #[inline(always)]
    fn offest_square(square: Square, offset: u8) -> Square {
        square - offset
    }
}
pub struct BlackPawnDirections;

impl PawnDirections for BlackPawnDirections {
    const COLOR: Color = Color::Black;
    const ONCE_PUSH_RANK: Bitboard = RANK_6;
    const PROMOTION_RANK: Bitboard = RANK_1;
    const LEFT_ATTACKS_OFFSET: u8 = 9;
    const RIGHT_ATTACKS_OFFSET: u8 = 7;

    #[inline(always)]
    fn shift_bitboard(bitboard: Bitboard, value: u8) -> Bitboard {
        bitboard >> value
    }

    #[inline(always)]
    fn offest_square(square: Square, offset: u8) -> Square {
        square + offset
    }
}
