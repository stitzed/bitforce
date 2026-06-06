use std::fmt::Display;

use crate::bitboard::Bitboard;
use crate::piece::{Color, Kind, Piece};
use crate::square::Square;

pub struct ChessBoard {
    pub current_turn: Color,
    board: [Option<Piece>; 64],
    /// `[WP, WN, WB, WR, WQ, WK | BP, BN, BB, BR, BQ, BK]`
    bitboards: [Bitboard; 12],
    all_white_bitboard: Bitboard,
    all_black_bitboard: Bitboard,
    all_pieces_bitboard: Bitboard
}

impl ChessBoard {
    pub fn new() -> Self {
        let mut board: [Option<Piece>; 64] = [None; 64];
        let mut bitboards: [Bitboard; 12] = [Bitboard::default(); 12];

        for color in Color::ALL_COLORS {
            for kind in Kind::ALL_KINDS {
                let shift: u32 = match (color, kind) {
                    (Color::White, Kind::Pawn) => 8,
                    (Color::Black, Kind::Pawn) => 8 * 6,
                    (Color::White, _) => 0,
                    (Color::Black, _) => 8 * 7
                };

                let piece: Piece = Piece::new(color, kind);
                let bitboard: Bitboard = Bitboard::new(kind.start_row_mask() << shift);

                bitboards[piece.to_index()] = bitboard;

                for sq in bitboard {
                    board[usize::from(sq)] = Some(piece);
                }
            }
        }

        let all_white_bitboard: Bitboard = Bitboard::new(0b1111_1111_1111_1111);
        let all_black_bitboard: Bitboard = Bitboard::new(0b1111_1111_1111_1111 << (8 * 6));
        
        Self {
            current_turn: Color::White,
            board,
            bitboards,
            all_white_bitboard,
            all_black_bitboard,
            all_pieces_bitboard: all_white_bitboard | all_black_bitboard
        }
    }
    
    #[inline(always)]
    pub fn get_piece_at(&self, square: Square) -> Option<Piece> {
        self.board[usize::from(square)]
    }
}

impl Display for ChessBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for row in (0..8).rev() {
            for col in 0..8 {
                let index: u8 = row * 8 + col;
                let square: Square = Square::new(index);

                let piece: Option<Piece> = self.get_piece_at(square);

                if let Some(p) = piece {
                    match (p.color(), p.kind()) {
                        (Color::White, Kind::Pawn) => write!(f, "P ")?,
                        (Color::White, Kind::Knight) => write!(f, "N ")?,
                        (Color::White, Kind::Bishop) => write!(f, "B ")?,
                        (Color::White, Kind::Rook) => write!(f, "R ")?,
                        (Color::White, Kind::Queen) => write!(f, "Q ")?,
                        (Color::White, Kind::King) => write!(f, "K ")?,
                        (Color::Black, Kind::Pawn) => write!(f, "p ")?,
                        (Color::Black, Kind::Knight) => write!(f, "n ")?,
                        (Color::Black, Kind::Bishop) => write!(f, "b ")?,
                        (Color::Black, Kind::Rook) => write!(f, "r ")?,
                        (Color::Black, Kind::Queen) => write!(f, "q ")?,
                        (Color::Black, Kind::King) => write!(f, "k ")?
                    }
                } else {
                    write!(f, ". ")?
                }
            }

            writeln!(f)?;
        }
        
        Ok(())
    }
}

impl Default for ChessBoard {
    fn default() -> Self {
        Self::new()
    }
}