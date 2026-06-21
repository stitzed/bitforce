use std::str::FromStr;

use crate::bitboard::Bitboard;
use crate::errors::CastlingParseError;
use crate::piece::Color;

#[derive(Clone, Copy)]
pub struct CastlingXorMasks {
    pub rook_mask: Bitboard,
    pub king_mask: Bitboard
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum CastlingType {
    Kingside = 1,
    Queenside = 2
}

impl CastlingType {
    pub const ALL_CASTLING_TYPES: [Self; 2] = [Self::Kingside, Self::Queenside];
    const PATH_MASKS: [Bitboard; 2] = [Bitboard::new(0b0110_0000), Bitboard::new(0b1110)];
    const XOR_MASKS: [CastlingXorMasks; 2] = [
        CastlingXorMasks {king_mask: Bitboard::new(0b0101_0000), rook_mask: Bitboard::new(0b1010_0000)}, 
        CastlingXorMasks {king_mask: Bitboard::new(0b0001_0100), rook_mask: Bitboard::new(0b1001),}
        ];
    
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
}

/// Bit layout:
/// [4 free][1 q][1 k][1 Q][1 K]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct CastlingFlags(u8);

impl CastlingFlags {
    #[inline(always)]
    pub fn new(
        white_kingside: bool, 
        white_queenside: bool, 
        black_kingside: bool, 
        black_queenside: bool
    ) -> Self {
        let mut bits: u8 = (black_queenside as u8) << 3;
        bits |= (black_kingside as u8) << 2;
        bits |= (white_queenside as u8) << 1;
        bits |= white_kingside as u8;

        Self(bits)
    }

    #[inline(always)]
    fn castle_to_mask(color: Color, castling_type: CastlingType) -> u8 {
        (castling_type as u8) << (color.to_index() * 2)
    }

    #[inline(always)]
    pub fn set_flag(&mut self, color: Color, castling_type: CastlingType) {
        self.0 |= Self::castle_to_mask(color, castling_type);
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
                _ => {return Err(CastlingParseError::InvalidChar(c));}
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