use std::fmt::Display;

use crate::bitboard::Bitboard;
use crate::buffer::{Buffer, MOVE_HISTORY_BUFFER_LEN, MOVE_BUFFER_LEN};
use crate::piece::{Color, Kind, Piece};
use crate::piece_move::Move;
use crate::square::Square;

const KNIGHT_OFFSETS: [(i8, i8); 8] = [(-2, -1), (-2, 1), (-1, -2), (-1, 2), (1, -2), (1, 2), (2, -1), (2, 1)];

pub const KNIGHT_MASKS: [Bitboard; 64] = {
    let mut masks: [Bitboard; 64] = [Bitboard::new(0); 64];

    let mut i: usize = 0;

    while i < 64 {
        let mut mask: u64 = 0;
        
        let sq: Square = Square::new(i as u8);
        let (row, col) = (sq.row(), sq.col());
        
        let mut arr_ptr: usize = 0;

        while arr_ptr < KNIGHT_OFFSETS.len() {
            let knight_offset: (i8, i8) = KNIGHT_OFFSETS[arr_ptr];
            let (delta_row, delta_col) = (row as i8 + knight_offset.0, col as i8 + knight_offset.1);

            if (delta_row >= 0 && delta_row < 8) && (delta_col >= 0 && delta_col < 8) {
                mask |= Square::new((delta_row * 8 + delta_col) as u8).to_bitboard_mask()
            }
            

            arr_ptr += 1;
        }

        masks[i] = Bitboard::new(mask);
        
        i += 1;
    }

    masks
};

pub struct ChessBoard {
    pub current_turn: Color,
    history_of_moves: Buffer<Move, MOVE_HISTORY_BUFFER_LEN>,
    board: [Option<Piece>; 64],
    /// `[WP, WN, WB, WR, WQ, WK | BP, BN, BB, BR, BQ, BK]`
    bitboards: [Bitboard; 12],
    /// `[ALL_W, ALL_B]`
    side_bitboards: [Bitboard; 2],
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

        let side_bitboards: [Bitboard; 2] = [all_white_bitboard, all_black_bitboard];
        
        Self {
            current_turn: Color::White,
            history_of_moves: Buffer::new(),
            board,
            bitboards,
            side_bitboards,
            all_pieces_bitboard: all_white_bitboard | all_black_bitboard
        }
    }
    
    #[inline(always)]
    pub fn get_piece_at(&self, square: Square) -> Option<Piece> {
        self.board[usize::from(square)]
    }

    pub fn generate_pseudo_legal_moves<'a>(&self, color: Color, move_buffer: &'a mut Buffer<Move, MOVE_BUFFER_LEN>) -> &'a [Move] {
        for kind in Kind::ALL_KINDS {
            match kind {
                Kind::Pawn => {},
                Kind::Knight => self.generate_knight_moves(color, move_buffer),
                Kind::Bishop => {},
                Kind::Rook => {},
                Kind::Queen => {},
                Kind::King => {}
            }
        }
        
        move_buffer.as_slice()
    }
    
    fn generate_knight_moves(&self, color: Color, move_buffer: &mut Buffer<Move, MOVE_BUFFER_LEN>) {
        let knights_bitboard: Bitboard = self.bitboards[Piece::new(color, Kind::Knight).to_index()];

        for sq in knights_bitboard {
            let knight_mask: Bitboard = KNIGHT_MASKS[usize::from(sq)];

            let quiet_squares: Bitboard = knight_mask & !self.all_pieces_bitboard;

            for quiet_sq in quiet_squares {
                move_buffer.push(
                    Move::new(
                        sq, 
                        quiet_sq, 
                        None, 
                        None,
                        false, 
                        None
                    )
                );
            }

            let capture_squares: Bitboard = knight_mask & self.side_bitboards[(!color).to_index()];

            for cap_sq in capture_squares {
                let captured_type: Option<Kind> = self.get_piece_at(cap_sq).map(|p| p.kind());
                
                move_buffer.push(
                    Move::new(
                        sq, 
                        cap_sq,
                        None,
                        captured_type, 
                        false, 
                        None
                    )
                );
            }
        }
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