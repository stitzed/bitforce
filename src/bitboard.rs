use std::fmt::Display;
use std::ops::*;

use crate::square::Square;

const fn generate_masks(offsets: &[(i8, i8)]) -> [Bitboard; 64] {
    let mut masks: [Bitboard; 64] = [Bitboard::new(0); 64];

    let mut i: usize = 0;

    while i < 64 {
        let mut mask: u64 = 0;

        let sq: Square = Square::new(i as u8);
        let (row, col) = (sq.row(), sq.col());

        let mut arr_ptr: usize = 0;

        while arr_ptr < offsets.len() {
            let piece_offset: (i8, i8) = offsets[arr_ptr];
            let (delta_row, delta_col) = (row as i8 + piece_offset.0, col as i8 + piece_offset.1);

            if (delta_row >= 0 && delta_row < 8) && (delta_col >= 0 && delta_col < 8) {
                mask |= Square::from_coords(delta_row as u8, delta_col as u8).to_bitboard_mask()
            }

            arr_ptr += 1;
        }

        masks[i] = Bitboard::new(mask);

        i += 1;
    }

    masks
}

const KNIGHT_OFFSETS: [(i8, i8); 8] = [(-2, -1), (-2, 1), (-1, -2), (-1, 2), (1, -2), (1, 2), (2, -1), (2, 1)];

const KING_OFFSETS: [(i8, i8); 8] = [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)];

pub const KNIGHT_MASKS: [Bitboard; 64] = generate_masks(&KNIGHT_OFFSETS);
pub const KING_MASKS: [Bitboard; 64] = generate_masks(&KING_OFFSETS);

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Bitboard(u64);

impl Bitboard {
    #[inline(always)]
    pub const fn new(bitboard: u64) -> Self {
        Self(bitboard)
    }

    #[inline(always)]
    pub fn set_bit(&mut self, square: Square) {
        self.0 |= square.to_bitboard_mask()
    }

    #[inline(always)]
    pub fn clear_bit(&mut self, square: Square) {
        self.0 &= !(square.to_bitboard_mask())
    }

    #[inline(always)]
    pub fn is_bit_setted(&self, square: Square) -> bool {
        self.0 & (square.to_bitboard_mask()) != 0
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    #[inline(always)]
    pub fn first_square_unchecked(&self) -> Square {
        Square::new(self.0.trailing_zeros() as u8)
    }
}

impl BitAnd for Bitboard {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for Bitboard {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Bitboard {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0
    }
}

impl Not for Bitboard {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl Shl<u8> for Bitboard {
    type Output = Self;

    fn shl(self, rhs: u8) -> Self::Output {
        Self(self.0 << (rhs & 63))
    }
}

impl Shr<u8> for Bitboard {
    type Output = Self;

    fn shr(self, rhs: u8) -> Self::Output {
        Self(self.0 >> (rhs & 63))
    }
}

impl BitXor for Bitboard {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for Bitboard {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0
    }
}

impl Display for Bitboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for row in (0..8).rev() {
            for col in 0..8 {
                let index: u8 = row * 8 + col;
                let square: Square = Square::new(index);

                if self.is_bit_setted(square) {
                    write!(f, "X ")?;
                } else {
                    write!(f, ". ")?;
                }
            }

            writeln!(f)?;
        }

        Ok(())
    }
}

pub struct BitboardIterator(Bitboard);

impl Iterator for BitboardIterator {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.0.is_empty() {
            let square: Square = self.0.first_square_unchecked();
            self.0.0 &= self.0.0 - 1;

            Some(square)
        } else {
            None
        }
    }
}

impl IntoIterator for Bitboard {
    type IntoIter = BitboardIterator;
    type Item = Square;

    fn into_iter(self) -> Self::IntoIter {
        BitboardIterator(self)
    }
}

#[rustfmt::skip]
#[macro_export]
macro_rules! bitboard {
    (
        $r7_0:tt $r7_1:tt $r7_2:tt $r7_3:tt $r7_4:tt $r7_5:tt $r7_6:tt $r7_7:tt ;
        $r6_0:tt $r6_1:tt $r6_2:tt $r6_3:tt $r6_4:tt $r6_5:tt $r6_6:tt $r6_7:tt ;
        $r5_0:tt $r5_1:tt $r5_2:tt $r5_3:tt $r5_4:tt $r5_5:tt $r5_6:tt $r5_7:tt ;
        $r4_0:tt $r4_1:tt $r4_2:tt $r4_3:tt $r4_4:tt $r4_5:tt $r4_6:tt $r4_7:tt ;
        $r3_0:tt $r3_1:tt $r3_2:tt $r3_3:tt $r3_4:tt $r3_5:tt $r3_6:tt $r3_7:tt ;
        $r2_0:tt $r2_1:tt $r2_2:tt $r2_3:tt $r2_4:tt $r2_5:tt $r2_6:tt $r2_7:tt ;
        $r1_0:tt $r1_1:tt $r1_2:tt $r1_3:tt $r1_4:tt $r1_5:tt $r1_6:tt $r1_7:tt ;
        $r0_0:tt $r0_1:tt $r0_2:tt $r0_3:tt $r0_4:tt $r0_5:tt $r0_6:tt $r0_7:tt ;
    ) => {{
        let mut value: u64 = 0;

        macro_rules! parse_bit {
            (X) => { 1 };
            (.) => { 0 };
        }

        value |= (parse_bit!($r7_0) << 56) | (parse_bit!($r7_1) << 57) | (parse_bit!($r7_2) << 58) | (parse_bit!($r7_3) << 59) | (parse_bit!($r7_4) << 60) | (parse_bit!($r7_5) << 61) | (parse_bit!($r7_6) << 62) | (parse_bit!($r7_7) << 63);
        value |= (parse_bit!($r6_0) << 48) | (parse_bit!($r6_1) << 49) | (parse_bit!($r6_2) << 50) | (parse_bit!($r6_3) << 51) | (parse_bit!($r6_4) << 52) | (parse_bit!($r6_5) << 53) | (parse_bit!($r6_6) << 54) | (parse_bit!($r6_7) << 55);
        value |= (parse_bit!($r5_0) << 40) | (parse_bit!($r5_1) << 41) | (parse_bit!($r5_2) << 42) | (parse_bit!($r5_3) << 43) | (parse_bit!($r5_4) << 44) | (parse_bit!($r5_5) << 45) | (parse_bit!($r5_6) << 46) | (parse_bit!($r5_7) << 47);
        value |= (parse_bit!($r4_0) << 32) | (parse_bit!($r4_1) << 33) | (parse_bit!($r4_2) << 34) | (parse_bit!($r4_3) << 35) | (parse_bit!($r4_4) << 36) | (parse_bit!($r4_5) << 37) | (parse_bit!($r4_6) << 38) | (parse_bit!($r4_7) << 39);
        value |= (parse_bit!($r3_0) << 24) | (parse_bit!($r3_1) << 25) | (parse_bit!($r3_2) << 26) | (parse_bit!($r3_3) << 27) | (parse_bit!($r3_4) << 28) | (parse_bit!($r3_5) << 29) | (parse_bit!($r3_6) << 30) | (parse_bit!($r3_7) << 31);
        value |= (parse_bit!($r2_0) << 16) | (parse_bit!($r2_1) << 17) | (parse_bit!($r2_2) << 18) | (parse_bit!($r2_3) << 19) | (parse_bit!($r2_4) << 20) | (parse_bit!($r2_5) << 21) | (parse_bit!($r2_6) << 22) | (parse_bit!($r2_7) << 23);
        value |= (parse_bit!($r1_0) << 8)  | (parse_bit!($r1_1) << 9)  | (parse_bit!($r1_2) << 10) | (parse_bit!($r1_3) << 11) | (parse_bit!($r1_4) << 12) | (parse_bit!($r1_5) << 13) | (parse_bit!($r1_6) << 14) | (parse_bit!($r1_7) << 15);
        value |= (parse_bit!($r0_0) << 0)  | (parse_bit!($r0_1) << 1)  | (parse_bit!($r0_2) << 2)  | (parse_bit!($r0_3) << 3)  | (parse_bit!($r0_4) << 4)  | (parse_bit!($r0_5) << 5)  | (parse_bit!($r0_6) << 6)  | (parse_bit!($r0_7) << 7);

        Bitboard::new(value)
    }};
}

pub const FILE_A: Bitboard = bitboard![
    X . . . . . . .;
    X . . . . . . .;
    X . . . . . . .;
    X . . . . . . .;
    X . . . . . . .;
    X . . . . . . .;
    X . . . . . . .;
    X . . . . . . .;
];

pub const FILE_H: Bitboard = bitboard![
    . . . . . . . X;
    . . . . . . . X;
    . . . . . . . X;
    . . . . . . . X;
    . . . . . . . X;
    . . . . . . . X;
    . . . . . . . X;
    . . . . . . . X;
];

pub const RANK_3: Bitboard = bitboard![
    . . . . . . . .;
    . . . . . . . .;
    . . . . . . . .;
    . . . . . . . .;
    . . . . . . . .;
    X X X X X X X X;
    . . . . . . . .;
    . . . . . . . .;
];

pub const RANK_6: Bitboard = bitboard![
    . . . . . . . .;
    . . . . . . . .;
    X X X X X X X X;
    . . . . . . . .;
    . . . . . . . .;
    . . . . . . . .;
    . . . . . . . .;
    . . . . . . . .;
];
