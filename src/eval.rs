use crate::bitboard::Bitboard;
use crate::board::ChessBoard;
use crate::piece::{Color, Kind, Piece};

#[rustfmt::skip]
const MG_PIECE_SQUARE_TABLE: [[i32; 64]; 6] = {
    let mut table: [[i32; 64]; 6] = [[0; 64]; 6];

    // pawn
    table[0] = [
        0, 0, 0, 0, 0, 0, 0, 0,
        5, 10, 10, -20, -20, 10, 10, 5,
        5, -5, -10, 0, 0, -10, -5, 5,
        0, 0, 0, 20, 20, 0, 0, 0,
        5, 5, 10, 25, 25, 10, 5, 5,
        10, 10, 20, 30, 30, 20, 10, 10,
        50, 50, 50, 50, 50, 50, 50, 50,
        0, 0, 0, 0, 0, 0, 0, 0
    ];

    // knight
    table[1] = [
        -50, -40, -30, -30, -30, -30, -40, -50,
        -40, -20, 0, 5, 5, 0, -20, -40,
        -30, 5, 10, 15, 15, 10, 5, -30,
        -30, 0, 15, 20, 20, 15, 0, -30,
        -30, 5, 15, 20, 20, 15, 5, -30,
        -30, 0, 10, 15, 15, 10, 0, -30,
        -40, -20, 0, 0, 0, 0, -20, -40,
        -50, -40, -30, -30, -30, -30, -40, -50
    ];

    // bishop
    table[2] = [
        -20, -10, -10, -10, -10, -10, -10, -20,
        -10, 5, 0, 0, 0, 0, 5, -10,
        -10, 10, 10, 10, 10, 10, 10, -10,
        -10, 0, 10, 10, 10, 10, 0, -10,
        -10, 5, 5, 10, 10, 5, 5, -10,
        -10, 0, 5, 10, 10, 5, 0, -10,
        -10, 0, 0, 0, 0, 0, 0, -10,
        -20, -10, -10, -10, -10, -10, -10, -20
    ];

    // rook
    table[3] = [
        0, 0, 0, 5, 5, 0, 0, 0,
        -5, 0, 0, 0, 0, 0, 0, -5,
        -5, 0, 0, 0, 0, 0, 0, -5,
        -5, 0, 0, 0, 0, 0, 0, -5,
        -5, 0, 0, 0, 0, 0, 0, -5,
        -5, 0, 0, 0, 0, 0, 0, -5,
        5, 10, 10, 10, 10, 10, 10, 5,
        0, 0, 0, 0, 0, 0, 0, 0
    ];

    // queen
    table[4] = [
        -20, -10, -10, -5, -5, -10, -10, -20,
        -10, 0, 5, 0, 0, 0, 0, -10,
        -10, 5, 5, 5, 5, 5, 0, -10,
        0, 0, 5, 5, 5, 5, 0, -5,
        -5, 0, 5, 5, 5, 5, 0, -5,
        -10, 0, 5, 5, 5, 5, 0, -10,
        -10, 0, 0, 0, 0, 0, 0, -10,
        -20, -10, -10, -5, -5, -10, -10, -20
    ];

    // king
    table[5] = [
        20, 30, 10, 0, 0, 10, 30, 20,
        20, 20, 0, 0, 0, 0, 20, 20,
        -10, -20, -20, -20, -20, -20, -20, -10,
        -20, -30, -30, -40, -40, -30, -30, -20,
        -30, -40, -40, -50, -50, -40, -40, -30,
        -30, -40, -40, -50, -50, -40, -40, -30,
        -30, -40, -40, -50, -50, -40, -40, -30,
        -30, -40, -40, -50, -50, -40, -40, -30
    ];

    table
};

pub trait Evaluable {
    fn eval(board: &ChessBoard) -> i32;
}

pub struct ClassicEval;

impl Evaluable for ClassicEval {
    fn eval(board: &ChessBoard) -> i32 {
        let mut score: i32 = 0;

        for color in Color::ALL_COLORS {
            for kind in Kind::ALL_KINDS {
                let piece_bitboard: Bitboard = board.bitboard(Piece::new(color, kind));
                let material_value: i32 = piece_bitboard.count_squares() as i32 * kind.value();

                match color {
                    Color::White => {
                        score += material_value;

                        for sq in piece_bitboard {
                            score += MG_PIECE_SQUARE_TABLE[kind as usize][usize::from(sq)];
                        }
                    }
                    Color::Black => {
                        score -= material_value;

                        for sq in piece_bitboard {
                            score -= MG_PIECE_SQUARE_TABLE[kind as usize][usize::from(sq) ^ 56];
                        }
                    }
                }
            }
        }

        match board.current_turn() {
            Color::White => score,
            Color::Black => -score,
        }
    }
}
