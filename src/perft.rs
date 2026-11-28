use primitive_buffer::Buffer;

use crate::board::ChessBoard;
use crate::board::movegen::MOVE_BUFFER_LEN;
use crate::piece_move::Move;

pub fn perft(board: &mut ChessBoard, depth: u8) -> u64 {
    if depth == 0 {
        return 1;
    }

    let mut nodes: u64 = 0;
    let mut buf: Buffer<Move, MOVE_BUFFER_LEN> = Buffer::new();
    let moves: &[Move] = board.generate_legal_moves(board.current_turn(), &mut buf);

    if depth == 1 {
        return moves.len() as u64;
    }

    for &mv in moves {
        board.make_move(mv);
        nodes += perft(board, depth - 1);
        board.unmake_move();
    }
    nodes
}

pub fn perft_divide<F: FnMut(Move, u64)>(board: &mut ChessBoard, depth: u8, mut on_move: F) -> u64 {
    if depth == 0 {
        return 1;
    }

    let mut total: u64 = 0;
    let mut buf: Buffer<Move, MOVE_BUFFER_LEN> = Buffer::new();
    let moves: &[Move] = board.generate_legal_moves(board.current_turn(), &mut buf);

    for &mv in moves {
        board.make_move(mv);
        let nodes: u64 = perft(board, depth - 1);
        board.unmake_move();

        on_move(mv, nodes);
        total += nodes;
    }
    total
}
