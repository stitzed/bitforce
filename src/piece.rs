use std::ops::Not;

const COLORS: [Color; 2] = [Color::White, Color::Black];
const KINDS: [Kind; 6] = [Kind::Pawn, Kind::Knight, Kind::Bishop, Kind::Rook, Kind::Queen, Kind::King];

/// Bit layout: 
/// `[4 free][1 color][3 kind]`
pub struct Piece(u8);

impl Piece {
    const KIND_MASK: u8 = 0b111;
    
    #[inline(always)]  
    pub fn new(color: Color, kind: Kind) -> Self {
        Self(((color as u8) << 3) | ((kind as u8) - 1))
    }
    
    #[inline(always)]
    pub fn color(&self) -> Color {
        COLORS[(self.0 >> 3) as usize]
    }
    
    #[inline(always)]
    pub fn kind(&self) -> Kind {
        KINDS[(self.0 & Self::KIND_MASK) as usize]
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Kind {
    Pawn = 1,
    Knight = 2,
    Bishop = 3,
    Rook = 4,
    Queen = 5,
    King = 6
}

impl Kind {
    #[inline(always)]
    pub fn to_index(&self) -> usize {
        *self as usize - 1
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Color {
    White,
    Black
}

impl Not for Color {
    type Output = Self;
    
    fn not(self) -> Self::Output {
        match (self as u8) ^ 1 {
            0 => Color::White,
            _ => Color::Black
        }
    }
}