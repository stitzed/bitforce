use core::fmt::{Display, Write};
use core::str::FromStr;

use primitive_buffer::Buffer;

use crate::bitboard::prelude::*;
use crate::castle::{CastlingFlags, CastlingType};
use crate::errors::FenParseError;
use crate::piece::{BlackPawnDirections, Color, Kind, PawnDirections, Piece, WhitePawnDirections};
use crate::piece_move::{Move, MoveBuilder};
use crate::square::Square;

const FEN_PIECE_SETUP_IDX: usize = 0;
const FEN_CURRENT_TURN_IDX: usize = 1;
const FEN_CASTLING_FLAGS_IDX: usize = 2;
const FEN_EN_PASSANT_SQUARE_IDX: usize = 3;
const FEN_FIFTY_MOVE_COUNTER_IDX: usize = 4;
const FEN_FULLMOVE_NUMBER_IDX: usize = 5;

const MOVE_HISTORY_BUFFER_LEN: usize = 512;
const MOVE_BUFFER_LEN: usize = 128;

type MoveBuffer<'a> = &'a mut Buffer<Move, MOVE_BUFFER_LEN>;

mod attacks {
    use crate::bitboard::prelude::*;
    use crate::piece::Color;
    use crate::square::Square;

    pub fn hyperbola_quintessence_attacks(
        square: Square,
        line_masks: &[Bitboard; 64],
        occupancy: Bitboard,
    ) -> Bitboard {
        let mask: Bitboard = unsafe { *line_masks.get_unchecked(usize::from(square)) };
        let line: Bitboard = occupancy & mask;
        let slider: Bitboard = Bitboard::new(square.to_bitboard_mask());

        let forward: Bitboard = (line - (slider << 1)) ^ line;
        let reverse: Bitboard = ((line.swap_ranks() - (slider.swap_ranks() << 1)) ^ line.swap_ranks()).swap_ranks();

        (forward | reverse) & mask
    }

    pub fn get_pawn_attacks(square: Square, color: Color) -> Bitboard {
        unsafe { *PAWN_ATTACKERS_MASKS[color as usize].get_unchecked(usize::from(square)) }
    }

    pub fn get_knight_attacks(square: Square) -> Bitboard {
        unsafe { *KNIGHT_MASKS.get_unchecked(usize::from(square)) }
    }

    pub fn get_bishop_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
        hyperbola_quintessence_attacks(square, &DIAGONAL_LINES, occupancy)
            | hyperbola_quintessence_attacks(square, &ANTI_DIAGONAL_LINES, occupancy)
    }

    pub fn get_rank_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
        let row: u8 = square.row();
        let col: usize = square.col() as usize;
        let shift_bits: u8 = row * 8;

        let rank_bits: usize = usize::from(occupancy >> shift_bits);
        let index: usize = rank_bits & 0xFF;

        let attack_pattern: u64 = RANKS_ATTACKS[index][col] as u64;
        let shifted_attack: u64 = attack_pattern << shift_bits;

        let rank_attacks: Bitboard = Bitboard::new(shifted_attack);

        rank_attacks
    }

    pub fn get_rook_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
        hyperbola_quintessence_attacks(square, &FILE_LINES, occupancy) | get_rank_attacks(square, occupancy)
    }

    pub fn get_queen_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
        get_bishop_attacks(square, occupancy) | get_rook_attacks(square, occupancy)
    }

    pub fn get_king_attacks(square: Square) -> Bitboard {
        unsafe { *KING_MASKS.get_unchecked(usize::from(square)) }
    }
}

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

                    let square: Square = Square::from_coords(row, col);

                    self.board[usize::from(square)] = Some(piece);
                    self.bitboards[piece.to_index()] |= Bitboard::new(square.to_bitboard_mask());
                    self.side_bitboards[color as usize] |= Bitboard::new(square.to_bitboard_mask());

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

    fn get_pinned_rays(&self, color: Color) -> [Bitboard; 64] {
        let king_square: Square = self.bitboard(Piece::new(color, Kind::King)).first_square_unchecked();
        let opposite_bitboard: Bitboard = self.side_bitboards[!color as usize];
        let our_bitboard: Bitboard = self.side_bitboards[color as usize];

        let mut legal_squares: [Bitboard; 64] = [Bitboard::new(u64::MAX); 64];

        let bishop_attacks: Bitboard = attacks::get_bishop_attacks(king_square, opposite_bitboard);
        let enemy_diagonal: Bitboard =
            self.bitboard(Piece::new(!color, Kind::Bishop)) | self.bitboard(Piece::new(!color, Kind::Queen));
        let diagonal_pinners: Bitboard = bishop_attacks & enemy_diagonal;

        for pinner in diagonal_pinners {
            let squares_between: Bitboard = SQUARES_BETWEEN[usize::from(pinner)][usize::from(king_square)];
            let pinned_squares: Bitboard = squares_between & our_bitboard;

            if pinned_squares.count_squares() == 1 {
                let pinned_sq: Square = pinned_squares.first_square_unchecked();

                legal_squares[usize::from(pinned_sq)] = squares_between | Bitboard::new(pinner.to_bitboard_mask());
            }
        }

        let rook_attacks: Bitboard = attacks::get_rook_attacks(king_square, opposite_bitboard);
        let enemy_orthogonal: Bitboard =
            self.bitboard(Piece::new(!color, Kind::Rook)) | self.bitboard(Piece::new(!color, Kind::Queen));
        let orthogonal_pinners: Bitboard = rook_attacks & enemy_orthogonal;

        for pinner in orthogonal_pinners {
            let squares_between: Bitboard = SQUARES_BETWEEN[usize::from(pinner)][usize::from(king_square)];
            let pinned_squares: Bitboard = squares_between & our_bitboard;

            if pinned_squares.count_squares() == 1 {
                let pinned_sq: Square = pinned_squares.first_square_unchecked();

                legal_squares[usize::from(pinned_sq)] = squares_between | Bitboard::new(pinner.to_bitboard_mask());
            }
        }

        legal_squares
    }

    fn get_danger_squares(&self, color: Color) -> Bitboard {
        let opposite_color: Color = !color;

        let our_king_bitboard: Bitboard = self.bitboard(Piece::new(color, Kind::King));
        let occupancy: Bitboard = self.all_pieces_bitboard & !our_king_bitboard;

        let mut danger_squares: Bitboard = Bitboard::default();

        for sq in self.bitboard(Piece::new(opposite_color, Kind::Pawn)) {
            danger_squares |= attacks::get_pawn_attacks(sq, color);
        }

        for sq in self.bitboard(Piece::new(opposite_color, Kind::Knight)) {
            danger_squares |= attacks::get_knight_attacks(sq);
        }

        let queen_bitboard: Bitboard = self.bitboard(Piece::new(opposite_color, Kind::Queen));

        let diagonal_sliders: Bitboard = self.bitboard(Piece::new(opposite_color, Kind::Bishop)) | queen_bitboard;

        for sq in diagonal_sliders {
            danger_squares |= attacks::get_bishop_attacks(sq, occupancy);
        }

        let straight_sliders: Bitboard = self.bitboard(Piece::new(opposite_color, Kind::Rook)) | queen_bitboard;

        for sq in straight_sliders {
            danger_squares |= attacks::get_rook_attacks(sq, occupancy);
        }

        let enemy_king_square: Square = self
            .bitboard(Piece::new(opposite_color, Kind::King))
            .first_square_unchecked();

        danger_squares |= attacks::get_king_attacks(enemy_king_square);

        danger_squares
    }

    fn get_checkers(&self, color: Color) -> Bitboard {
        let opposite_color: Color = !color;

        let king_square: Square = self.bitboard(Piece::new(color, Kind::King)).first_square_unchecked();

        let mut checkers: Bitboard = Bitboard::default();

        let pawn_attacks: Bitboard = attacks::get_pawn_attacks(king_square, opposite_color);
        let enemy_pawns: Bitboard = self.bitboard(Piece::new(opposite_color, Kind::Pawn));
        checkers |= pawn_attacks & enemy_pawns;

        let knight_attacks: Bitboard = attacks::get_knight_attacks(king_square);
        let enemy_knights: Bitboard = self.bitboard(Piece::new(opposite_color, Kind::Knight));
        checkers |= knight_attacks & enemy_knights;

        let bishop_attacks: Bitboard = attacks::get_bishop_attacks(king_square, self.all_pieces_bitboard);
        let enemy_diagonal: Bitboard = self.bitboard(Piece::new(opposite_color, Kind::Bishop))
            | self.bitboard(Piece::new(opposite_color, Kind::Queen));
        checkers |= bishop_attacks & enemy_diagonal;

        let rook_attacks: Bitboard = attacks::get_rook_attacks(king_square, self.all_pieces_bitboard);
        let enemy_orthogonal: Bitboard = self.bitboard(Piece::new(opposite_color, Kind::Rook))
            | self.bitboard(Piece::new(opposite_color, Kind::Queen));
        checkers |= rook_attacks & enemy_orthogonal;

        let king_attacks: Bitboard = attacks::get_king_attacks(king_square);
        let enemy_kings: Bitboard = self.bitboard(Piece::new(opposite_color, Kind::King));
        checkers |= king_attacks & enemy_kings;

        checkers
    }

    pub fn generate_legal_moves<'a>(&self, color: Color, move_buffer: MoveBuffer<'a>) -> &'a [Move] {
        let knight_attacks = attacks::get_knight_attacks;
        let bishop_attacks = |sq| attacks::get_bishop_attacks(sq, self.all_pieces_bitboard);
        let rook_attacks = |sq| attacks::get_rook_attacks(sq, self.all_pieces_bitboard);
        let queen_attacks = |sq| attacks::get_queen_attacks(sq, self.all_pieces_bitboard);

        let checkers: Bitboard = self.get_checkers(self.current_turn);
        let danger_squares: Bitboard = self.get_danger_squares(self.current_turn);

        let king_square: Square = self.bitboard(Piece::new(color, Kind::King)).first_square_unchecked();

        let checker_ray: Bitboard = match checkers.count_squares() {
            0 => Bitboard::new(u64::MAX),
            1 => {
                let checker = checkers.first_square_unchecked();

                SQUARES_BETWEEN[usize::from(checker)][usize::from(king_square)]
                    | Bitboard::new(checker.to_bitboard_mask())
            }
            _ => {
                self.generate_king_moves(color, danger_squares, move_buffer);
                return move_buffer.as_slice();
            }
        };

        let pinned_rays: [Bitboard; 64] = self.get_pinned_rays(self.current_turn);

        for kind in Kind::ALL_KINDS {
            match kind {
                Kind::Pawn => self.generate_pawn_moves(color, &pinned_rays, checker_ray, move_buffer),
                Kind::Knight => {
                    self.generate_moves_for_kind(color, kind, knight_attacks, &pinned_rays, checker_ray, move_buffer)
                }
                Kind::Bishop => {
                    self.generate_moves_for_kind(color, kind, bishop_attacks, &pinned_rays, checker_ray, move_buffer)
                }
                Kind::Rook => {
                    self.generate_moves_for_kind(color, kind, rook_attacks, &pinned_rays, checker_ray, move_buffer)
                }
                Kind::Queen => {
                    self.generate_moves_for_kind(color, kind, queen_attacks, &pinned_rays, checker_ray, move_buffer)
                }
                Kind::King => self.generate_king_moves(color, danger_squares, move_buffer),
            }
        }

        move_buffer.as_slice()
    }

    fn generate_pawn_moves(
        &self,
        color: Color,
        pinned_rays: &[Bitboard; 64],
        checker_ray: Bitboard,
        move_buffer: MoveBuffer<'_>,
    ) {
        match color {
            Color::White => {
                self.generate_pawn_moves_for_color::<WhitePawnDirections>(pinned_rays, checker_ray, move_buffer)
            }
            Color::Black => {
                self.generate_pawn_moves_for_color::<BlackPawnDirections>(pinned_rays, checker_ray, move_buffer)
            }
        }
    }

    fn generate_pawn_moves_for_color<D: PawnDirections>(
        &self,
        pinned_rays: &[Bitboard; 64],
        checker_ray: Bitboard,
        move_buffer: MoveBuffer<'_>,
    ) {
        let pawn_bitboard: Bitboard = self.bitboard(Piece::new(D::COLOR, Kind::Pawn));
        let opposite_bitboard: Bitboard = self.side_bitboards[!D::COLOR as usize];

        let once_push: Bitboard = D::shift_bitboard(pawn_bitboard, 8) & !self.all_pieces_bitboard;
        let double_push: Bitboard = D::shift_bitboard(once_push & D::ONCE_PUSH_RANK, 8) & !self.all_pieces_bitboard;

        let legal_once_push: Bitboard = once_push & checker_ray;
        let legal_double_push: Bitboard = double_push & checker_ray;

        let left_attacks_mask: Bitboard = D::shift_bitboard(pawn_bitboard & !FILE_A, D::LEFT_ATTACKS_OFFSET);
        let right_attacks_mask: Bitboard = D::shift_bitboard(pawn_bitboard & !FILE_H, D::RIGHT_ATTACKS_OFFSET);

        let left_attacks: Bitboard = left_attacks_mask & opposite_bitboard & checker_ray;
        let right_attacks: Bitboard = right_attacks_mask & opposite_bitboard & checker_ray;

        let is_legal = |from_sq, to_sq| pinned_rays[usize::from(from_sq)].contains(to_sq);

        // En passant moves
        if let Some(ep_sq) = self.en_passant_square {
            let king_square: Square = self.bitboard(Piece::new(D::COLOR, Kind::King)).first_square_unchecked();
            let opposite_pawn: Square = D::offest_square(ep_sq, 8);

            let mut ep_rank: Bitboard = self.all_pieces_bitboard & Bitboard::new(0xFF << (opposite_pawn.row() * 8));
            ep_rank.clear_square(opposite_pawn);

            let pawn_attackers_mask: Bitboard =
                unsafe { *PAWN_ATTACKERS_MASKS[D::COLOR as usize].get_unchecked(usize::from(ep_sq)) };

            let attackers: Bitboard = pawn_bitboard & pawn_attackers_mask;

            for attacker in attackers {
                let ep_move: Move = MoveBuilder::new(attacker, ep_sq)
                    .with_capture()
                    .with_en_passant()
                    .build();

                if !is_legal(attacker, ep_sq) {
                    continue;
                }

                if king_square.row() != opposite_pawn.row() {
                    move_buffer.push(ep_move);
                    continue;
                }

                let mut simulated_rank: Bitboard = ep_rank;
                simulated_rank.clear_square(attacker);

                let enemy_orthogonal: Bitboard = self.bitboard(Piece::new(!D::COLOR, Kind::Rook))
                    | self.bitboard(Piece::new(!D::COLOR, Kind::Queen));

                if (attacks::get_rank_attacks(king_square, simulated_rank) & enemy_orthogonal).is_empty() {
                    move_buffer.push(ep_move);
                }
            }
        }

        // Quiet one push moves
        for sq in legal_once_push & !D::PROMOTION_RANK {
            let from_square: Square = D::offest_square(sq, 8);

            if !is_legal(from_square, sq) {
                continue;
            };

            move_buffer.push(MoveBuilder::new(from_square, sq).build());
        }

        // Quiet one push moves with promotion
        for sq in legal_once_push & D::PROMOTION_RANK {
            let from_square: Square = D::offest_square(sq, 8);

            if !is_legal(from_square, sq) {
                continue;
            };

            for promotion_type in Kind::PROMOTION_KINDS {
                move_buffer.push(MoveBuilder::new(from_square, sq).with_promotion(promotion_type).build());
            }
        }

        // Quiet double push moves
        for sq in legal_double_push {
            let from_square: Square = D::offest_square(sq, 16);

            if !is_legal(from_square, sq) {
                continue;
            };

            move_buffer.push(MoveBuilder::new(from_square, sq).build());
        }

        // Left capture moves
        for sq in left_attacks & !D::PROMOTION_RANK {
            let from_square: Square = D::offest_square(sq, D::LEFT_ATTACKS_OFFSET);

            if !is_legal(from_square, sq) {
                continue;
            };

            move_buffer.push(MoveBuilder::new(from_square, sq).with_capture().build());
        }

        // Left capture moves with promotion
        for sq in left_attacks & D::PROMOTION_RANK {
            let from_square: Square = D::offest_square(sq, D::LEFT_ATTACKS_OFFSET);

            if !is_legal(from_square, sq) {
                continue;
            };

            for promotion_type in Kind::PROMOTION_KINDS {
                move_buffer.push(
                    MoveBuilder::new(from_square, sq)
                        .with_capture()
                        .with_promotion(promotion_type)
                        .build(),
                );
            }
        }

        // Right capture moves
        for sq in right_attacks & !D::PROMOTION_RANK {
            let from_square: Square = D::offest_square(sq, D::RIGHT_ATTACKS_OFFSET);

            if !is_legal(from_square, sq) {
                continue;
            };

            move_buffer.push(MoveBuilder::new(from_square, sq).with_capture().build());
        }

        // Right capture moves with promotion
        for sq in right_attacks & D::PROMOTION_RANK {
            let from_square: Square = D::offest_square(sq, D::RIGHT_ATTACKS_OFFSET);

            if !is_legal(from_square, sq) {
                continue;
            };

            for promotion_type in Kind::PROMOTION_KINDS {
                move_buffer.push(
                    MoveBuilder::new(from_square, sq)
                        .with_capture()
                        .with_promotion(promotion_type)
                        .build(),
                );
            }
        }
    }

    fn generate_moves_for_kind<F>(
        &self,
        color: Color,
        kind: Kind,
        get_attacks: F,
        pinned_rays: &[Bitboard; 64],
        checker_ray: Bitboard,
        move_buffer: MoveBuffer<'_>,
    ) where
        F: Fn(Square) -> Bitboard,
    {
        let piece_bitboard: Bitboard = self.bitboard(Piece::new(color, kind));

        for sq in piece_bitboard {
            let pinned_ray: Bitboard = pinned_rays[usize::from(sq)];
            let attacks: Bitboard = get_attacks(sq) & pinned_ray & checker_ray;

            let quiet_squares: Bitboard = attacks & !self.all_pieces_bitboard;

            for quiet_sq in quiet_squares {
                move_buffer.push(MoveBuilder::new(sq, quiet_sq).build());
            }

            let capture_squares: Bitboard = attacks & self.side_bitboards[(!color) as usize];

            for cap_sq in capture_squares {
                move_buffer.push(MoveBuilder::new(sq, cap_sq).with_capture().build());
            }
        }
    }

    fn generate_king_moves(&self, color: Color, danger_squares: Bitboard, move_buffer: MoveBuffer<'_>) {
        let king_bitboard: Bitboard = self.bitboard(Piece::new(color, Kind::King));
        let king_square: Square = king_bitboard.first_square_unchecked();

        let attacks: Bitboard = attacks::get_king_attacks(king_square);

        let quiet_squares: Bitboard = attacks & !self.all_pieces_bitboard & !danger_squares;

        for quiet_sq in quiet_squares {
            move_buffer.push(MoveBuilder::new(king_square, quiet_sq).build());
        }

        let capture_squares: Bitboard = attacks & self.side_bitboards[(!color) as usize] & !danger_squares;

        for cap_sq in capture_squares {
            move_buffer.push(MoveBuilder::new(king_square, cap_sq).with_capture().build());
        }

        let color_shift: u8 = color.shift();
        let castle_square: Square = Square::E1 + color_shift;

        if king_square != castle_square {
            return;
        }

        if danger_squares.contains(king_square) {
            return;
        }

        for castle in CastlingType::ALL_CASTLING_TYPES {
            if !self.castling_flags.can_castle(color, castle) {
                continue;
            }

            let path_mask: Bitboard = (castle.path_mask() << color_shift) & self.all_pieces_bitboard;
            if !(path_mask).is_empty() {
                continue;
            }

            let passed_square: Square = castle.passed_square() + color_shift;
            if danger_squares.contains(passed_square) {
                continue;
            }

            let mask: Bitboard = castle.xor_mask().king_mask << color_shift;
            let to_square: Square = (king_bitboard ^ mask).first_square_unchecked();

            if danger_squares.contains(to_square) {
                continue;
            }

            move_buffer.push(MoveBuilder::new(king_square, to_square).with_castling(castle).build());
        }
    }

    pub fn is_square_attacked(&self, square: Square, opposite_color: Color) -> bool {
        // Pawn
        let pawn_attacks: Bitboard = attacks::get_pawn_attacks(square, opposite_color);
        let enemy_pawns: Bitboard = self.bitboard(Piece::new(opposite_color, Kind::Pawn));
        if !(pawn_attacks & enemy_pawns).is_empty() {
            return true;
        }

        // Knight
        let knight_attacks: Bitboard = attacks::get_knight_attacks(square);
        let enemy_knights: Bitboard = self.bitboard(Piece::new(opposite_color, Kind::Knight));
        if !(knight_attacks & enemy_knights).is_empty() {
            return true;
        }

        // Bishop and Queen
        let bishop_attacks: Bitboard = attacks::get_bishop_attacks(square, self.all_pieces_bitboard);
        let enemy_diagonal: Bitboard = self.bitboard(Piece::new(opposite_color, Kind::Bishop))
            | self.bitboard(Piece::new(opposite_color, Kind::Queen));
        if !(bishop_attacks & enemy_diagonal).is_empty() {
            return true;
        }

        // Rook and Queen
        let rook_attacks: Bitboard = attacks::get_rook_attacks(square, self.all_pieces_bitboard);
        let enemy_orthogonal: Bitboard = self.bitboard(Piece::new(opposite_color, Kind::Rook))
            | self.bitboard(Piece::new(opposite_color, Kind::Queen));
        if !(rook_attacks & enemy_orthogonal).is_empty() {
            return true;
        }

        // King
        let king_attacks: Bitboard = attacks::get_king_attacks(square);
        let enemy_kings: Bitboard = self.bitboard(Piece::new(opposite_color, Kind::King));
        if !(king_attacks & enemy_kings).is_empty() {
            return true;
        }

        false
    }

    pub fn is_in_check(&self, color: Color) -> bool {
        self.is_square_attacked(
            self.bitboard(Piece::new(color, Kind::King)).first_square_unchecked(),
            !color,
        )
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

    pub fn perft(&mut self, depth: usize) -> u64 {
        if depth == 0 {
            return 1;
        }

        let mut nodes: u64 = 0;
        let mut buf: Buffer<Move, MOVE_BUFFER_LEN> = Buffer::new();
        let moves: &[Move] = self.generate_legal_moves(self.current_turn, &mut buf);

        if depth == 1 {
            return moves.len() as u64;
        }

        for &mv in moves {
            self.make_move_unchecked(mv);
            nodes += self.perft(depth - 1);
            self.unmake_move();
        }
        nodes
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
