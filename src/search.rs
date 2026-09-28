use primitive_buffer::Buffer;

use crate::board::ChessBoard;
use crate::eval::{ClassicEval, Evaluable};
use crate::piece_move::Move;

const INFINITY: i32 = 64_000;
const MATE_SCORE: i32 = 32_000;
pub struct Search {
    start_depth: u8,
}

impl Search {
    pub fn new(start_depth: u8) -> Self {
        Self { start_depth }
    }

    pub fn find_best_move(&self, board: &mut ChessBoard) -> Option<Move> {
        let mut buf: Buffer<Move, 256> = Buffer::new();
        let moves: &[Move] = board.generate_legal_moves(board.current_turn(), &mut buf);

        let mut best_score: i32 = i32::MIN;
        let mut best_move: Option<Move> = None;

        for mv in moves {
            board.make_move_unchecked(*mv);
            let score: i32 = -self.alpha_beta(board, self.start_depth - 1, -INFINITY, INFINITY);
            board.unmake_move();

            if score > best_score {
                best_score = score;
                best_move = Some(*mv)
            }
        }

        best_move
    }

    fn alpha_beta(&self, board: &mut ChessBoard, depth: u8, mut alpha: i32, beta: i32) -> i32 {
        if depth == 0 {
            return ClassicEval::eval(board);
        }

        let mut buf: Buffer<Move, 256> = Buffer::new();
        let moves: &[Move] = board.generate_legal_moves(board.current_turn(), &mut buf);

        if moves.is_empty() {
            return if board.is_in_check(board.current_turn()) {
                -MATE_SCORE + (self.start_depth - depth) as i32
            } else {
                0
            };
        }

        let mut best_score: i32 = i32::MIN;

        for mv in moves {
            board.make_move_unchecked(*mv);
            let score: i32 = -self.alpha_beta(board, depth - 1, -beta, -alpha);
            board.unmake_move();

            if score > best_score {
                best_score = score
            }

            if best_score > alpha {
                alpha = best_score
            }

            if alpha >= beta {
                break;
            }
        }

        best_score
    }
}
