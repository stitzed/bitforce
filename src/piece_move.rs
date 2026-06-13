use std::fmt::Debug;

use crate::castle::CastlingType;
use crate::square::Square;
use crate::piece::Kind;

/// [11 free][2 castling_type][1 is_en_passant][3 captured_type][3 promotion_type][6 from][6 to]
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Move(u32);

impl Move {
    #[inline(always)]
    pub fn new(
        from_square: Square, 
        to_square: Square, 
        promotion_type: Option<Kind>, 
        captured_type: Option<Kind>, 
        is_en_passant: bool, 
        castling_type: Option<CastlingType>
    ) -> Self {
        let mut bits: u32 = (castling_type.map_or(0, |c| c as u32)) << 19;

        bits |= (is_en_passant as u32) << 18;
        bits |= captured_type.map_or(0, |k| k as u32) << 15;
        bits |= promotion_type.map_or(0, |k| k as u32) << 12;
        bits |= (u32::from(from_square)) << 6;
        bits |= u32::from(to_square);

        Self(bits)
    }

    #[inline(always)]
    pub fn from_square(&self) -> Square {
        Square::new(((self.0 >> 6) & Square::SQUARE_MASK as u32) as u8)
    }
    
    #[inline(always)]
    pub fn to_square(&self) -> Square {
        Square::new((self.0 & Square::SQUARE_MASK as u32) as u8)
    }

    #[inline(always)]
    pub fn promotion_type(&self) -> Option<Kind> {
        Kind::from_index(((self.0 >> 12) & Kind::KIND_MASK as u32) as u8)
    }

    #[inline(always)]
    pub fn captured_type(&self) -> Option<Kind> {
        Kind::from_index(((self.0 >> 15) & Kind::KIND_MASK as u32) as u8)
    }

    #[inline(always)]
    pub fn is_en_passant(&self) -> bool {
        ((self.0 >> 18) & 1) == 1
    }

    #[inline(always)]
    pub fn castling_type(&self) -> Option<CastlingType> {
        CastlingType::from_index((self.0 >> 19) as u8)
    }
}

impl Debug for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Move")
            .field("from_square", &format_args!("{}", self.from_square()))
            .field("to_square", &format_args!("{}", self.to_square()))
            .field("promotion_type", &self.promotion_type())
            .field("captured_type", &self.captured_type())
            .field("is_en_passant", &self.is_en_passant())
            .field("castling_type", &self.castling_type())
            .finish()
    }
}