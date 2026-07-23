use std::fmt::Display;
use std::str::FromStr;

use crate::bitboard::Bitboard;
use crate::errors::CastlingParseError;
use crate::piece::Color;
use crate::square::Square;

#[derive(Clone, Copy)]
pub struct CastlingXorMasks {
    pub rook_mask: Bitboard,
    pub king_mask: Bitboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CastlingType {
    Kingside = 1,
    Queenside = 2,
}

impl CastlingType {
    pub const ALL_CASTLING_TYPES: [Self; 2] = [Self::Kingside, Self::Queenside];

    const PATH_MASKS: [Bitboard; 2] = [Bitboard::new(0b0110_0000), Bitboard::new(0b1110)];

    const XOR_MASKS: [CastlingXorMasks; 2] = [
        CastlingXorMasks {
            king_mask: Bitboard::new(0b0101_0000),
            rook_mask: Bitboard::new(0b1010_0000),
        },
        CastlingXorMasks {
            king_mask: Bitboard::new(0b0001_0100),
            rook_mask: Bitboard::new(0b1001),
        },
    ];

    const PASSED_SQUARES: [Square; 2] = [Square::F1, Square::D1];

    #[inline(always)]
    pub fn from_index(index: u8) -> Option<Self> {
        let idx: u8 = index.checked_sub(1)?;

        Self::ALL_CASTLING_TYPES.get(usize::from(idx)).copied()
    }

    #[inline(always)]
    pub fn path_mask(&self) -> Bitboard {
        Self::PATH_MASKS[*self as usize - 1]
    }

    #[inline(always)]
    pub fn xor_mask(&self) -> CastlingXorMasks {
        Self::XOR_MASKS[*self as usize - 1]
    }

    #[inline(always)]
    pub fn passed_square(&self) -> Square {
        Self::PASSED_SQUARES[*self as usize - 1]
    }

    #[inline(always)]
    pub fn rook_start_square(&self) -> Square {
        (Bitboard::new(self.passed_square().to_bitboard_mask()) ^ self.xor_mask().rook_mask).first_square_unchecked()
    }
}

/// Bit layout:
/// [4 free][1 q][1 k][1 Q][1 K]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct CastlingFlags(u8);

impl CastlingFlags {
    #[inline(always)]
    pub fn new(white_kingside: bool, white_queenside: bool, black_kingside: bool, black_queenside: bool) -> Self {
        let mut bits: u8 = (black_queenside as u8) << 3;
        bits |= (black_kingside as u8) << 2;
        bits |= (white_queenside as u8) << 1;
        bits |= white_kingside as u8;

        Self(bits)
    }

    #[inline(always)]
    fn castle_to_mask(color: Color, castling_type: CastlingType) -> u8 {
        (castling_type as u8) << (color as u8 * 2)
    }

    #[inline(always)]
    pub fn set_flag(&mut self, color: Color, castling_type: CastlingType) {
        self.0 |= Self::castle_to_mask(color, castling_type);
    }

    #[inline(always)]
    pub fn update_castling(&mut self, color: Color, castling_type: CastlingType, can_castle: bool) {
        let mask: u8 = Self::castle_to_mask(color, castling_type);
        self.0 = (self.0 & !mask) | (mask * can_castle as u8);
    }

    #[inline(always)]
    pub fn unset_flag(&mut self, color: Color, castling_type: CastlingType) {
        self.0 &= !(Self::castle_to_mask(color, castling_type));
    }

    #[inline(always)]
    pub fn can_castle(&self, color: Color, castling_type: CastlingType) -> bool {
        (self.0 & Self::castle_to_mask(color, castling_type)) != 0
    }
}

impl Display for CastlingFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 == 0 {
            write!(f, "-")?;
            return Ok(());
        }

        for color in Color::ALL_COLORS {
            for castle in CastlingType::ALL_CASTLING_TYPES {
                if !self.can_castle(color, castle) {
                    continue;
                }

                let char: char = match (color, castle) {
                    (Color::White, CastlingType::Kingside) => 'K',
                    (Color::White, CastlingType::Queenside) => 'Q',
                    (Color::Black, CastlingType::Kingside) => 'k',
                    (Color::Black, CastlingType::Queenside) => 'q',
                };

                write!(f, "{char}")?;
            }
        }

        Ok(())
    }
}

impl FromStr for CastlingFlags {
    type Err = CastlingParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut castling_flags: Self = Self(0);

        if s == "-" {
            return Ok(castling_flags);
        }

        let mut current_rank: u8 = 0;

        for c in s.chars() {
            let (castling_rank, color, castling_type) = match c {
                'K' => (1, Color::White, CastlingType::Kingside),
                'Q' => (2, Color::White, CastlingType::Queenside),
                'k' => (3, Color::Black, CastlingType::Kingside),
                'q' => (4, Color::Black, CastlingType::Queenside),
                _ => {
                    return Err(CastlingParseError::InvalidChar(c));
                }
            };

            if castling_rank <= current_rank {
                return Err(CastlingParseError::InvalidFormat);
            }

            castling_flags.set_flag(color, castling_type);

            current_rank = castling_rank;
        }

        Ok(castling_flags)
    }
}
