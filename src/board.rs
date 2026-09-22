use core::fmt::{Display, Write};
use core::str::FromStr;

use primitive_buffer::Buffer;

use crate::bitboard::Bitboard;
use crate::castle::CastlingFlags;
use crate::errors::FenParseError;
use crate::piece::{Color, Kind, Piece};
use crate::piece_move::Move;
use crate::square::Square;

pub mod movegen;

const FEN_PIECE_SETUP_IDX: usize = 0;
const FEN_CURRENT_TURN_IDX: usize = 1;
const FEN_CASTLING_FLAGS_IDX: usize = 2;
const FEN_EN_PASSANT_SQUARE_IDX: usize = 3;
const FEN_FIFTY_MOVE_COUNTER_IDX: usize = 4;
const FEN_FULLMOVE_NUMBER_IDX: usize = 5;

const MOVE_HISTORY_BUFFER_LEN: usize = 512;

#[derive(Clone, Copy)]
struct UndoInfo {
    captured_type: Option<Kind>,
    castling_flags: CastlingFlags,
    en_passant_square: Option<Square>,
    fifty_move_counter: u8,
}

pub struct ChessBoard {
    current_turn: Color,
    history_of_moves: Buffer<Move, MOVE_HISTORY_BUFFER_LEN>,
    castling_flags: CastlingFlags,
    en_passant_square: Option<Square>,
    fifty_move_counter: u8,
    fullmove_number: u16,
    board: [Option<Piece>; 64],
    /// `[WP, WN, WB, WR, WQ, WK | BP, BN, BB, BR, BQ, BK]`
    bitboards: [Bitboard; 12],
    /// `[ALL_W, ALL_B]`
    side_bitboards: [Bitboard; 2],
    all_pieces_bitboard: Bitboard,
    undo_info_stack: Buffer<UndoInfo, MOVE_HISTORY_BUFFER_LEN>,
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
                    (Color::Black, _) => 8 * 7,
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
            castling_flags: CastlingFlags::new(true, true, true, true),
            en_passant_square: None,
            fifty_move_counter: 0,
            fullmove_number: 1,
            board,
            bitboards,
            side_bitboards,
            all_pieces_bitboard: all_white_bitboard | all_black_bitboard,
            undo_info_stack: Buffer::new(),
        }
    }

    pub fn from_fen<'a>(fen: &'a str) -> Result<Self, FenParseError<'a>> {
        let mut board: Self = Self {
            current_turn: Color::White,
            history_of_moves: Buffer::new(),
            castling_flags: CastlingFlags::new(false, false, false, false),
            en_passant_square: None,
            fifty_move_counter: 0,
            fullmove_number: 0,
            board: [None; 64],
            bitboards: [Bitboard::default(); 12],
            side_bitboards: [Bitboard::default(); 2],
            all_pieces_bitboard: Bitboard::default(),
            undo_info_stack: Buffer::new(),
        };

        let mut total_parts: u8 = 0;

        for (i, part) in fen.split_whitespace().enumerate() {
            total_parts += 1;

            match i {
                FEN_PIECE_SETUP_IDX => {
                    board.parse_piece_setup(part)?;
                }

                FEN_CURRENT_TURN_IDX => {
                    board.current_turn = match part {
                        "w" => Color::White,
                        "b" => Color::Black,
                        _ => {
                            return Err(FenParseError::InvalidColor(part));
                        }
                    };
                }

                FEN_CASTLING_FLAGS_IDX => {
                    board.castling_flags = CastlingFlags::from_str(part).map_err(FenParseError::InvalidCastle)?;
                }

                FEN_EN_PASSANT_SQUARE_IDX => {
                    if part != "-" {
                        board.en_passant_square =
                            Some(Square::from_str(part).map_err(FenParseError::InvalidEnPassantSquare)?);
                    }
                }

                FEN_FIFTY_MOVE_COUNTER_IDX => {
                    board.fifty_move_counter =
                        part.parse().map_err(|_| FenParseError::InvalidFiftyMoveCounter(part))?;
                }

                FEN_FULLMOVE_NUMBER_IDX => {
                    board.fullmove_number = part.parse().map_err(|_| FenParseError::InvalidFullmoveCounter(part))?;
                }

                _ => {
                    return Err(FenParseError::InvalidFormat);
                }
            }
        }

        if total_parts != 6 {
            return Err(FenParseError::InvalidFormat);
        }

        Ok(board)
    }

    fn parse_piece_setup<'a>(&mut self, pieces_setup: &'a str) -> Result<(), FenParseError<'a>> {
        let mut row: u8 = 7;
        let mut col: u8 = 0;

        for c in pieces_setup.chars() {
            match c {
                '1'..='8' => {
                    let next_col: u8 = col + (c as u8) - b'0';

                    if next_col > 8 {
                        return Err(FenParseError::RowOverflow(col));
                    }

                    col = next_col;
                }

                fig if fig.is_ascii_alphabetic() => {
                    let kind: Kind = match fig.to_ascii_lowercase() {
                        'p' => Kind::Pawn,
                        'n' => Kind::Knight,
                        'b' => Kind::Bishop,
                        'r' => Kind::Rook,
                        'q' => Kind::Queen,
                        'k' => Kind::King,
                        _ => {
                            return Err(FenParseError::InvalidPieceChar(fig));
                        }
                    };

                    let color: Color = if fig.is_ascii_uppercase() {
                        Color::White
                    } else {
                        Color::Black
                    };

                    let piece: Piece = Piece::new(color, kind);

                    if col >= 8 {
                        return Err(FenParseError::RowOverflow(col));
                    }

                    let square: Square = unsafe { Square::from_coords_unchecked(row, col) };

                    self.board[usize::from(square)] = Some(piece);
                    self.bitboards[piece.to_index()] |= Bitboard::from(square);
                    self.side_bitboards[color as usize] |= Bitboard::from(square);

                    col += 1;
                }

                '/' => {
                    row = match row.checked_sub(1) {
                        Some(n) => n,
                        None => {
                            return Err(FenParseError::ExtraRow);
                        }
                    };

                    col = 0;
                }

                _ => {
                    return Err(FenParseError::UnexpectedChar(c));
                }
            }
        }

        self.all_pieces_bitboard = self.side_bitboards[0] | self.side_bitboards[1];

        Ok(())
    }

    pub fn to_fen<W: Write>(&self, dest: &mut W) -> core::fmt::Result {
        let mut is_first_row: bool = true;

        for row in self.board.chunks(8).rev() {
            let mut empty_squares: u8 = 0;

            if !is_first_row {
                write!(dest, "/")?
            }

            for piece in row {
                match piece {
                    Some(p) => {
                        if empty_squares != 0 {
                            write!(dest, "{empty_squares}")?;
                        }

                        match (p.color(), p.kind()) {
                            (Color::White, Kind::Pawn) => write!(dest, "P")?,
                            (Color::White, Kind::Knight) => write!(dest, "N")?,
                            (Color::White, Kind::Bishop) => write!(dest, "B")?,
                            (Color::White, Kind::Rook) => write!(dest, "R")?,
                            (Color::White, Kind::Queen) => write!(dest, "Q")?,
                            (Color::White, Kind::King) => write!(dest, "K")?,
                            (Color::Black, Kind::Pawn) => write!(dest, "p")?,
                            (Color::Black, Kind::Knight) => write!(dest, "n")?,
                            (Color::Black, Kind::Bishop) => write!(dest, "b")?,
                            (Color::Black, Kind::Rook) => write!(dest, "r")?,
                            (Color::Black, Kind::Queen) => write!(dest, "q")?,
                            (Color::Black, Kind::King) => write!(dest, "k")?,
                        };

                        empty_squares = 0;
                    }

                    None => {
                        empty_squares += 1;
                    }
                }
            }

            if empty_squares != 0 {
                write!(dest, "{empty_squares}")?;
            }

            is_first_row = false;
        }

        write!(dest, " ")?;

        let color_char: char = match self.current_turn {
            Color::White => 'w',
            Color::Black => 'b',
        };

        write!(dest, "{color_char}")?;

        write!(dest, " ")?;

        write!(dest, "{}", self.castling_flags)?;

        write!(dest, " ")?;

        match self.en_passant_square {
            Some(sq) => write!(dest, "{sq}")?,
            None => write!(dest, "-")?,
        }

        write!(dest, " ")?;

        write!(dest, "{}", self.fifty_move_counter)?;

        write!(dest, " ")?;

        write!(dest, "{}", self.fullmove_number)?;

        Ok(())
    }

    #[inline(always)]
    pub fn get_piece_at(&self, square: Square) -> Option<Piece> {
        unsafe { *self.board.get_unchecked(usize::from(square)) }
    }

    #[inline(always)]
    pub fn get_piece_at_mut(&mut self, square: Square) -> &mut Option<Piece> {
        unsafe { self.board.get_unchecked_mut(usize::from(square)) }
    }

    #[inline(always)]
    pub fn current_turn(&self) -> Color {
        self.current_turn
    }

    #[inline(always)]
    pub fn bitboard(&self, piece: Piece) -> Bitboard {
        self.bitboards[piece.to_index()]
    }

    #[inline(always)]
    pub fn set_piece(&mut self, piece: Piece, square: Square) {
        self.bitboards[piece.to_index()].set_square(square);
        self.side_bitboards[piece.color() as usize].set_square(square);
        self.all_pieces_bitboard.set_square(square);
        *self.get_piece_at_mut(square) = Some(piece);
    }

    #[inline(always)]
    pub fn clear_piece(&mut self, piece: Piece, square: Square) {
        self.bitboards[piece.to_index()].clear_square(square);
        self.side_bitboards[piece.color() as usize].clear_square(square);
        self.all_pieces_bitboard.clear_square(square);
        *self.get_piece_at_mut(square) = None;
    }

    pub fn make_move_unchecked(&mut self, piece_move: Move) {
        let from_square: Square = piece_move.from_square();
        let to_square: Square = piece_move.to_square();

        let target_piece: Piece = self.get_piece_at(from_square).expect("from_square should be non-empty");
        let target_piece_color: Color = target_piece.color();
        let color_shift: u8 = target_piece_color.shift();
        let en_passant_square: Square = match target_piece_color {
            Color::White => to_square - 8,
            Color::Black => to_square + 8,
        };

        self.clear_piece(target_piece, from_square);

        let mut captured_type: Option<Kind> = None;

        // Order matters: clear the captured piece before setting the moving piece.
        // Otherwise, on regular captures, `clear_piece` would erase the newly placed piece at `to_square`.
        if piece_move.is_capture() {
            let (square, kind) = if piece_move.is_en_passant() {
                (en_passant_square, Kind::Pawn)
            } else {
                // SAFETY: `is_capture()` guarantees a piece exists at `to_square`
                // based on bitboard validation.
                let kind: Kind = unsafe { self.get_piece_at(to_square).unwrap_unchecked().kind() };
                (to_square, kind)
            };

            let opposite_piece: Piece = Piece::new(!target_piece_color, kind);

            self.clear_piece(opposite_piece, square);

            captured_type = Some(kind);
        }

        self.set_piece(target_piece, to_square);

        if let Some(promotion_type) = piece_move.promotion_type() {
            let promotion_piece: Piece = Piece::new(target_piece_color, promotion_type);

            self.clear_piece(target_piece, to_square);
            self.set_piece(promotion_piece, to_square);
        }

        if let Some(castling_type) = piece_move.castling_type() {
            let rook: Piece = Piece::new(target_piece_color, Kind::Rook);
            let rook_mask: Bitboard = castling_type.xor_mask().rook_mask << color_shift;

            self.bitboards[rook.to_index()] ^= rook_mask;
            self.side_bitboards[target_piece_color as usize] ^= rook_mask;
            self.all_pieces_bitboard ^= rook_mask;
            *self.get_piece_at_mut(castling_type.passed_square() + color_shift) = Some(rook);

            *self.get_piece_at_mut(castling_type.rook_start_square() + color_shift) = None;
        }

        let undo_info: UndoInfo = UndoInfo {
            captured_type,
            castling_flags: self.castling_flags,
            en_passant_square: self.en_passant_square,
            fifty_move_counter: self.fifty_move_counter,
        };

        self.undo_info_stack.push(undo_info);

        self.castling_flags.clear_rights_for_move(from_square, to_square);

        let is_double_push: bool =
            target_piece.kind() == Kind::Pawn && u8::from(to_square) ^ u8::from(from_square) == 16;
        self.en_passant_square = is_double_push.then_some(en_passant_square);

        let reset_mask: bool = !(target_piece.kind() == Kind::Pawn || piece_move.is_capture());
        self.fifty_move_counter = (self.fifty_move_counter + 1) * reset_mask as u8;

        self.fullmove_number += self.current_turn as u16;

        self.current_turn = !self.current_turn;
        self.history_of_moves.push(piece_move);

        debug_assert!(self.is_synchronized())
    }

    pub fn unmake_move(&mut self) {
        let piece_move: Move = self
            .history_of_moves
            .pop()
            .expect("history_of_moves should be non-empty");

        let undo_info: UndoInfo = self.undo_info_stack.pop().expect("undo_info_stack should be non-empty");

        let from_square: Square = piece_move.from_square();
        let to_square: Square = piece_move.to_square();

        let target_piece: Piece = self.get_piece_at(to_square).expect("to_square should be non-empty");
        let target_piece_color: Color = target_piece.color();
        let color_shift: u8 = target_piece_color.shift();

        self.clear_piece(target_piece, to_square);
        self.set_piece(target_piece, from_square);

        if let Some(captured_type) = undo_info.captured_type {
            let square: Square = if piece_move.is_en_passant() {
                match target_piece_color {
                    Color::White => to_square - 8,
                    Color::Black => to_square + 8,
                }
            } else {
                to_square
            };

            let opposite_piece: Piece = Piece::new(!target_piece_color, captured_type);

            self.set_piece(opposite_piece, square);
        }

        if piece_move.promotion_type().is_some() {
            self.clear_piece(target_piece, from_square);
            self.set_piece(Piece::new(target_piece_color, Kind::Pawn), from_square);
        }

        if let Some(castling_type) = piece_move.castling_type() {
            let rook: Piece = Piece::new(target_piece_color, Kind::Rook);
            let rook_mask: Bitboard = castling_type.xor_mask().rook_mask << color_shift;

            self.bitboards[rook.to_index()] ^= rook_mask;
            self.side_bitboards[target_piece_color as usize] ^= rook_mask;
            self.all_pieces_bitboard ^= rook_mask;
            *self.get_piece_at_mut(castling_type.passed_square() + color_shift) = None;

            *self.get_piece_at_mut(castling_type.rook_start_square() + color_shift) = Some(rook);
        }

        self.castling_flags = undo_info.castling_flags;
        self.en_passant_square = undo_info.en_passant_square;
        self.fifty_move_counter = undo_info.fifty_move_counter;

        self.current_turn = !self.current_turn;
        self.fullmove_number -= self.current_turn as u16;

        debug_assert!(self.is_synchronized())
    }

    pub fn is_synchronized(&self) -> bool {
        for i in 0..64 {
            let piece: Option<Piece> = self.board[i];
            let sq: Square = unsafe { Square::new_unchecked(i as u8) };

            match piece {
                Some(p) => {
                    let bb: Bitboard = self.bitboard(p);
                    if !bb.contains(sq) {
                        return false;
                    }

                    for (i, bb) in self.bitboards.iter().enumerate() {
                        if i == p.to_index() {
                            continue;
                        }

                        if bb.contains(sq) {
                            return false;
                        }
                    }

                    let s_bb: Bitboard = self.side_bitboards[p.color() as usize];
                    if !s_bb.contains(sq) {
                        return false;
                    }

                    if self.side_bitboards[!p.color() as usize].contains(sq) {
                        return false;
                    }

                    if !self.all_pieces_bitboard.contains(sq) {
                        return false;
                    }
                }
                None => {
                    for bb in self.bitboards {
                        if bb.contains(sq) {
                            return false;
                        }
                    }

                    for bb in self.side_bitboards {
                        if bb.contains(sq) {
                            return false;
                        }
                    }

                    if self.all_pieces_bitboard.contains(sq) {
                        return false;
                    }
                }
            }
        }

        true
    }
}

impl Display for ChessBoard {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for row in (0..8).rev() {
            for col in 0..8 {
                let index: u8 = row * 8 + col;
                let square: Square = unsafe { Square::new_unchecked(index) };

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
                        (Color::Black, Kind::King) => write!(f, "k ")?,
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
