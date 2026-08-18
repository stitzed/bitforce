use crate::board::ChessBoard;
use crate::piece::{Color, Kind, Piece};

pub trait Evaluable {
    fn eval(board: &ChessBoard) -> i32;
}

pub struct ClassicEval;

impl Evaluable for ClassicEval {
    fn eval(board: &ChessBoard) -> i32 {
        let mut score: i32 = 0;

        for color in Color::ALL_COLORS {
            for kind in Kind::ALL_KINDS {
                let value: i32 = board.bitboard(Piece::new(color, kind)).count_squares() as i32 * kind.value();

                match color {
                    Color::White => score += value,
                    Color::Black => score -= value,
                }
            }
        }

        match board.current_turn() {
            Color::White => score,
            Color::Black => -score,
        }
    }
}
