use std::ops::Not;

/// Bit layout: 
/// `[4 free][1 color][3 kind]`
pub struct Piece(u8);

impl Piece {
    #[inline(always)]  
    pub fn new(color: Color, kind: Kind) -> Self {
        Self(((color as u8) << 3) | ((kind as u8) - 1))
    }
    
    #[inline(always)]
    pub fn color(&self) -> Color {
        Color::ALL_COLORS[(self.0 >> 3) as usize]
    }
    
    #[inline(always)]
    pub fn kind(&self) -> Kind {
        Kind::ALL_KINDS[(self.0 & Kind::KIND_MASK) as usize]
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
    pub const ALL_KINDS: [Self; 6] = [
        Self::Pawn, Self::Knight, Self::Bishop, 
        Self::Rook, Self::Queen, Self::King
        ];
    
    pub const KIND_MASK: u8 = 0b111;
    
    #[inline(always)]
    pub fn to_index(&self) -> usize {
        *self as usize - 1
    }

    #[inline(always)]
    pub fn from_index(index: u8) -> Option<Self> {
        let idx: u8 = index.checked_sub(1)?;
        
        Self::ALL_KINDS.get(usize::from(idx)).copied()
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Color {
    White,
    Black
}

impl Color {
    pub const ALL_COLORS: [Self; 2] = [Self::White, Self::Black];
}

impl Not for Color {
    type Output = Self;
    
    fn not(self) -> Self::Output {
        Self::ALL_COLORS[((self as u8) ^ 1) as usize]
    }
}