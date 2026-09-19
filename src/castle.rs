use core::fmt::Display;
use core::str::FromStr;

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
    Kingside = 0b01,
    Queenside = 0b10,
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
        (Bitboard::from(self.passed_square()) ^ self.xor_mask().rook_mask).first_square_unchecked()
    }
}

/// Bit layout:
/// [4 free][1 q][1 k][1 Q][1 K]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct CastlingFlags(u8);

impl CastlingFlags {
    const RIGHTS_MASK: [u8; 64] = {
        let mut table: [u8; 64] = [0b1111; 64];

        let white_kingside: u8 = Self::castle_to_mask(Color::White, CastlingType::Kingside);
        let white_queenside: u8 = Self::castle_to_mask(Color::White, CastlingType::Queenside);
        let black_kingside: u8 = Self::castle_to_mask(Color::Black, CastlingType::Kingside);
        let black_queenside: u8 = Self::castle_to_mask(Color::Black, CastlingType::Queenside);

        table[4] = !(white_kingside | white_queenside); // E1
        table[60] = !(black_kingside | black_queenside); // E8

        table[7] = !white_kingside; // H1
        table[0] = !white_queenside; // A1
        table[63] = !black_kingside; // H8
        table[56] = !black_queenside; // A8

        table
    };

    #[inline(always)]
    pub fn new(white_kingside: bool, white_queenside: bool, black_kingside: bool, black_queenside: bool) -> Self {
        let mut bits: u8 = (black_queenside as u8) << 3;
        bits |= (black_kingside as u8) << 2;
        bits |= (white_queenside as u8) << 1;
        bits |= white_kingside as u8;

        Self(bits)
    }

    #[inline(always)]
    const fn castle_to_mask(color: Color, castling_type: CastlingType) -> u8 {
        (castling_type as u8) << (color as u8 * 2)
    }

    pub fn clear_rights_for_move(&mut self, from_square: Square, to_square: Square) {
        self.0 &= Self::RIGHTS_MASK[usize::from(from_square)] & Self::RIGHTS_MASK[usize::from(to_square)]
    }

    #[inline(always)]
    pub fn can_castle(&self, color: Color, castling_type: CastlingType) -> bool {
        (self.0 & Self::castle_to_mask(color, castling_type)) != 0
    }
}

impl Display for CastlingFlags {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

            castling_flags.0 |= Self::castle_to_mask(color, castling_type);

            current_rank = castling_rank;
        }

        Ok(castling_flags)
    }
}
