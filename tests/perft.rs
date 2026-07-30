use bitforce::board::ChessBoard;

#[test]
fn perft_5_depth() {
    let mut board: ChessBoard = ChessBoard::new();
    assert_eq!(board.perft(5), 4_865_609);
}

#[test]
#[ignore = "takes a few seconds"]
fn perft_6_depth() {
    let mut board: ChessBoard = ChessBoard::new();
    assert_eq!(board.perft(6), 119_060_324);
}

#[test]
fn perft_kiwipete_4_depth() {
    let mut board: ChessBoard =
        ChessBoard::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
            .expect("FEN should be valid");
    assert_eq!(board.perft(4), 4_085_603);
}

#[test]
#[ignore = "takes a few seconds"]
fn perft_kiwipete_5_depth() {
    let mut board: ChessBoard =
        ChessBoard::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
            .expect("FEN should be valid");
    assert_eq!(board.perft(5), 193_690_690);
}
