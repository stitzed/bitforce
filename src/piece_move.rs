use core::fmt::Debug;
use core::fmt::Display;

use crate::castle::CastlingType;
use crate::piece::Kind;
use crate::square::Square;

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
        castling_type: Option<CastlingType>,
    ) -> Self {
        let mut bits: u32 = (castling_type.map_or(0, |c| c as u32)) << 19;

        bits |= (is_en_passant as u32) << 18;
        bits |= captured_type.map_or(Kind::NONE_VALUE as u32, |k| k as u32) << 15;
        bits |= promotion_type.map_or(Kind::NONE_VALUE as u32, |k| k as u32) << 12;
        bits |= (u32::from(from_square)) << 6;
        bits |= u32::from(to_square);

        Self(bits)
    }

    #[inline(always)]
    pub fn from_square(&self) -> Square {
        unsafe { Square::new_unchecked(((self.0 >> 6) & Square::SQUARE_MASK as u32) as u8) }
    }

    #[inline(always)]
    pub fn to_square(&self) -> Square {
        unsafe { Square::new_unchecked((self.0 & Square::SQUARE_MASK as u32) as u8) }
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

impl Display for Move {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.from_square())?;
        write!(f, "{}", self.to_square())?;

        if let Some(promotion_type) = self.promotion_type() {
            match promotion_type {
                Kind::Pawn => write!(f, "p")?,
                Kind::Knight => write!(f, "n")?,
                Kind::Bishop => write!(f, "b")?,
                Kind::Rook => write!(f, "r")?,
                Kind::Queen => write!(f, "q")?,
                Kind::King => write!(f, "k")?,
            }
        }

        Ok(())
    }
}

impl Debug for Move {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

pub struct MoveBuilder(u32);

impl MoveBuilder {
    #[inline(always)]
    pub fn new(from_square: Square, to_square: Square) -> Self {
        let mut bits: u32 = 0b0011_0110_0000_0000_0000;
        bits |= (u32::from(from_square)) << 6;
        bits |= u32::from(to_square);
        Self(bits)
    }

    #[inline(always)]
    pub fn with_promotion(mut self, kind: Kind) -> Self {
        self.0 &= !((Kind::KIND_MASK as u32) << 12);
        self.0 |= (kind as u32) << 12;
        self
    }

    #[inline(always)]
    pub fn with_capture(mut self, kind: Kind) -> Self {
        self.0 &= !((Kind::KIND_MASK as u32) << 15);
        self.0 |= (kind as u32) << 15;
        self
    }

    #[inline(always)]
    pub fn with_optional_capture(mut self, kind: Option<Kind>) -> Self {
        let val: u32 = kind.map_or(Kind::NONE_VALUE as u32, |k| k as u32);
        self.0 &= !((Kind::KIND_MASK as u32) << 15);
        self.0 |= val << 15;
        self
    }

    #[inline(always)]
    pub fn with_en_passant(mut self) -> Self {
        self.0 |= 1 << 18;
        self
    }

    #[inline(always)]
    pub fn with_castling(mut self, castling: CastlingType) -> Self {
        self.0 &= !(0b11 << 19);
        self.0 |= (castling as u32) << 19;
        self
    }

    #[inline(always)]
    pub fn build(self) -> Move {
        Move(self.0)
    }
}
