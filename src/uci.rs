use std::io::{self, Write};
use std::num::NonZeroU8;
use std::str::FromStr;

use bitforce::board::ChessBoard;
use bitforce::piece::Kind;
use bitforce::piece_move::Move;
use bitforce::search::Search;
use bitforce::square::Square;
use primitive_buffer::Buffer;

pub enum UciCommand<'a> {
    Uci,
    IsReady,
    Position { fen: Option<&'a str>, moves: &'a str },
    Go { depth: Option<NonZeroU8> },
    Quit,
}

impl<'a> UciCommand<'a> {
    pub fn parse(cmd: &'a str) -> Option<Self> {
        let mut args = cmd.split_whitespace();

        let command: Option<Self> = match args.next()? {
            "uci" => Some(Self::Uci),
            "isready" => Some(Self::IsReady),
            "position" => {
                let rest: &str = cmd.strip_prefix("position")?.trim();

                let (head, moves) = match rest.split_once("moves") {
                    Some((h, t)) => (h.trim(), t.trim()),
                    None => (rest, ""),
                };

                let fen: Option<&str> = if head == "startpos" {
                    None
                } else {
                    Some(head.strip_prefix("fen")?.trim())
                };

                Some(Self::Position { fen, moves })
            }
            "go" => {
                let depth: Option<NonZeroU8> = if args.next() == Some("depth") {
                    args.next().and_then(|d| NonZeroU8::from_str(d).ok())
                } else {
                    None
                };

                Some(Self::Go { depth })
            }
            "quit" => Some(Self::Quit),
            _ => None,
        };

        command
    }
}

pub struct UciProtocol<W: Write> {
    board: ChessBoard,
    out: W,
}

impl<W: Write> UciProtocol<W> {
    pub fn new(out: W) -> Self {
        Self {
            board: ChessBoard::new(),
            out,
        }
    }

    fn send(&mut self, line: &str) {
        writeln!(self.out, "{line}").expect("failed to write");
        self.out.flush().expect("failed to flush")
    }

    pub fn run(&mut self) {
        let mut buf: String = String::new();

        loop {
            buf.clear();

            if io::stdin().read_line(&mut buf).expect("failed to read from stdin") == 0 {
                break;
            }

            let Some(cmd) = UciCommand::parse(&buf) else {
                continue;
            };

            if let UciCommand::Quit = cmd {
                break;
            };

            self.handle(cmd);
        }
    }

    pub fn handle(&mut self, cmd: UciCommand) {
        match cmd {
            UciCommand::Uci => {
                self.send("id name BitForce");
                self.send("id author stitzed");
                self.send("uciok")
            }
            UciCommand::IsReady => self.send("readyok"),
            UciCommand::Position { fen, moves } => {
                if let Some(b) = Self::build_board(fen, moves) {
                    self.board = b
                }
            }
            UciCommand::Go { depth } => {
                let search: Search = Search::new(depth.map(|d| d.get()).unwrap_or(6));
                let best_move: Option<Move> = search.find_best_move(&mut self.board);

                match best_move {
                    Some(mv) => self.send(&format!("bestmove {mv}")),
                    None => self.send("bestmove 0000"),
                }
            }
            UciCommand::Quit => {}
        }
    }

    fn build_board(fen: Option<&str>, moves: &str) -> Option<ChessBoard> {
        let mut board: ChessBoard = match fen {
            Some(fen) => ChessBoard::from_fen(fen).ok()?,
            None => ChessBoard::new(),
        };

        let mut buffer: Buffer<Move, 256> = Buffer::new();

        for mv in moves.split_whitespace() {
            buffer.clear();

            let from_square: Square = mv.get(0..=1).and_then(|sq| Square::from_str(sq).ok())?;
            let to_square: Square = mv.get(2..=3).and_then(|sq| Square::from_str(sq).ok())?;

            let promotion_type: Option<Kind> = match mv.chars().nth(4) {
                Some('n') => Some(Kind::Knight),
                Some('b') => Some(Kind::Bishop),
                Some('r') => Some(Kind::Rook),
                Some('q') => Some(Kind::Queen),
                Some(_) => None,
                None => None,
            };

            let moves: &[Move] = board.generate_legal_moves(board.current_turn(), &mut buffer);

            let legal_move: Option<&Move> = moves.iter().find(|&m| {
                m.from_square() == from_square && m.to_square() == to_square && m.promotion_type() == promotion_type
            });

            match legal_move {
                Some(&mv) => board.make_move_unchecked(mv),
                None => return None,
            }
        }

        Some(board)
    }
}
