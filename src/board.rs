use std::fmt::Display;
use std::str::FromStr;

use primitive_buffer::Buffer;

use crate::bitboard::prelude::*;
use crate::castle::{CastlingFlags, CastlingType};
use crate::errors::FenParseError;
use crate::piece::{Color, Kind, Piece};
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

#[derive(Clone, Copy)]
struct UndoInfo {
    castling_flags: CastlingFlags,
    en_passant_square: Option<Square>,
    fifty_move_counter: u8,
}

pub struct ChessBoard {
    pub current_turn: Color,
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
    undo_info_stack: Buffer<UndoInfo, MOVE_BUFFER_LEN>,
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
            fullmove_number: 0,
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

    #[inline(always)]
    pub fn get_piece_at(&self, square: Square) -> Option<Piece> {
        unsafe { *self.board.get_unchecked(usize::from(square)) }
    }

    #[inline(always)]
    pub fn get_piece_at_mut(&mut self, square: Square) -> &mut Option<Piece> {
        unsafe { self.board.get_unchecked_mut(usize::from(square)) }
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

    pub fn generate_pseudo_legal_moves<'a>(&self, color: Color, move_buffer: MoveBuffer<'a>) -> &'a [Move] {
        for kind in Kind::ALL_KINDS {
            match kind {
                Kind::Pawn => self.generate_pawn_moves(color, move_buffer),
                Kind::Knight => self.generate_static_moves(color, kind, KNIGHT_MASKS, move_buffer),
                Kind::Bishop => self.generate_sliding_moves(color, kind, &BISHOP_RAYS_MASKS, move_buffer),
                Kind::Rook => self.generate_sliding_moves(color, kind, &ROOK_RAYS_MASKS, move_buffer),
                Kind::Queen => self.generate_queen_moves(color, move_buffer),
                Kind::King => self.generate_king_moves(color, move_buffer),
            }
        }

        move_buffer.as_slice()
    }

    fn generate_pawn_moves(&self, color: Color, move_buffer: MoveBuffer<'_>) {
        match color {
            Color::White => self.generate_white_pawn_moves(move_buffer),
            Color::Black => self.generate_black_pawn_moves(move_buffer),
        }
    }

    #[inline]
    fn generate_white_pawn_moves(&self, move_buffer: MoveBuffer<'_>) {
        let pawn_bitboard: Bitboard = self.bitboards[Piece::new(Color::White, Kind::Pawn).to_index()];
        let opposite_bitboard: Bitboard = self.side_bitboards[Color::Black as usize];

        let once_push: Bitboard = (pawn_bitboard << 8) & !self.all_pieces_bitboard;
        let double_push: Bitboard = ((once_push & RANK_3) << 8) & !self.all_pieces_bitboard;

        let left_attacks_mask: Bitboard = (pawn_bitboard & !FILE_A) << 7;
        let right_attacks_mask: Bitboard = (pawn_bitboard & !FILE_H) << 9;

        let left_attacks: Bitboard = left_attacks_mask & opposite_bitboard;
        let right_attacks: Bitboard = right_attacks_mask & opposite_bitboard;

        // En passant moves
        if let Some(ep_sq) = self.en_passant_square {
            let attackers: Bitboard = pawn_bitboard & WHITE_PAWN_ATTACKERS_MASKS[usize::from(ep_sq)];

            for sq in attackers {
                move_buffer.push(
                    MoveBuilder::new(sq, ep_sq)
                        .with_capture(Kind::Pawn)
                        .with_en_passant()
                        .build(),
                );
            }
        }

        // Quiet one push moves
        for sq in once_push & !RANK_8 {
            move_buffer.push(MoveBuilder::new(sq - 8, sq).build());
        }

        // Quiet one push moves with promotion
        for sq in once_push & RANK_8 {
            for promotion_type in Kind::PROMOTION_KINDS {
                move_buffer.push(MoveBuilder::new(sq - 8, sq).with_promotion(promotion_type).build());
            }
        }

        // Quiet double push moves
        for sq in double_push {
            move_buffer.push(MoveBuilder::new(sq - 16, sq).build());
        }

        // Left capture moves
        for sq in left_attacks & !RANK_8 {
            let captured_type: Option<Kind> = self.get_piece_at(sq).map(|p| p.kind());

            move_buffer.push(
                MoveBuilder::new(sq - 7, sq)
                    .with_optional_capture(captured_type)
                    .build(),
            );
        }

        // Left capture moves with promotion
        for sq in left_attacks & RANK_8 {
            let captured_type: Option<Kind> = self.get_piece_at(sq).map(|p| p.kind());

            for promotion_type in Kind::PROMOTION_KINDS {
                move_buffer.push(
                    MoveBuilder::new(sq - 7, sq)
                        .with_optional_capture(captured_type)
                        .with_promotion(promotion_type)
                        .build(),
                );
            }
        }

        // Right capture moves
        for sq in right_attacks & !RANK_8 {
            let captured_type: Option<Kind> = self.get_piece_at(sq).map(|p| p.kind());

            move_buffer.push(
                MoveBuilder::new(sq - 9, sq)
                    .with_optional_capture(captured_type)
                    .build(),
            );
        }

        // Right capture moves with promotion
        for sq in right_attacks & RANK_8 {
            let captured_type: Option<Kind> = self.get_piece_at(sq).map(|p| p.kind());

            for promotion_type in Kind::PROMOTION_KINDS {
                move_buffer.push(
                    MoveBuilder::new(sq - 9, sq)
                        .with_optional_capture(captured_type)
                        .with_promotion(promotion_type)
                        .build(),
                );
            }
        }
    }

    #[inline]
    fn generate_black_pawn_moves(&self, move_buffer: MoveBuffer<'_>) {
        let pawn_bitboard: Bitboard = self.bitboards[Piece::new(Color::Black, Kind::Pawn).to_index()];
        let opposite_bitboard: Bitboard = self.side_bitboards[Color::White as usize];

        let once_push: Bitboard = (pawn_bitboard >> 8) & !self.all_pieces_bitboard;
        let double_push: Bitboard = ((once_push & RANK_6) >> 8) & !self.all_pieces_bitboard;

        let left_attacks_mask: Bitboard = (pawn_bitboard & !FILE_A) >> 9;
        let right_attacks_mask: Bitboard = (pawn_bitboard & !FILE_H) >> 7;

        let left_attacks: Bitboard = left_attacks_mask & opposite_bitboard;
        let right_attacks: Bitboard = right_attacks_mask & opposite_bitboard;

        // En passant moves
        if let Some(ep_sq) = self.en_passant_square {
            let attackers: Bitboard = pawn_bitboard & BLACK_PAWN_ATTACKERS_MASKS[usize::from(ep_sq)];

            for sq in attackers {
                move_buffer.push(
                    MoveBuilder::new(sq, ep_sq)
                        .with_capture(Kind::Pawn)
                        .with_en_passant()
                        .build(),
                );
            }
        }

        // Quiet one push moves
        for sq in once_push & !RANK_1 {
            move_buffer.push(MoveBuilder::new(sq + 8, sq).build());
        }

        // Quiet one push moves with promotion
        for sq in once_push & RANK_1 {
            for promotion_type in Kind::PROMOTION_KINDS {
                move_buffer.push(MoveBuilder::new(sq + 8, sq).with_promotion(promotion_type).build());
            }
        }

        // Quiet double push moves
        for sq in double_push {
            move_buffer.push(MoveBuilder::new(sq + 16, sq).build());
        }

        // Left capture moves
        for sq in left_attacks & !RANK_1 {
            let captured_type: Option<Kind> = self.get_piece_at(sq).map(|p| p.kind());

            move_buffer.push(
                MoveBuilder::new(sq + 9, sq)
                    .with_optional_capture(captured_type)
                    .build(),
            );
        }

        // Left capture moves with promotion
        for sq in left_attacks & RANK_1 {
            let captured_type: Option<Kind> = self.get_piece_at(sq).map(|p| p.kind());

            for promotion_type in Kind::PROMOTION_KINDS {
                move_buffer.push(
                    MoveBuilder::new(sq + 9, sq)
                        .with_optional_capture(captured_type)
                        .with_promotion(promotion_type)
                        .build(),
                );
            }
        }

        // Right capture moves
        for sq in right_attacks & !RANK_1 {
            let captured_type: Option<Kind> = self.get_piece_at(sq).map(|p| p.kind());

            move_buffer.push(
                MoveBuilder::new(sq + 7, sq)
                    .with_optional_capture(captured_type)
                    .build(),
            );
        }

        // Right capture moves with promotion
        for sq in right_attacks & RANK_1 {
            let captured_type: Option<Kind> = self.get_piece_at(sq).map(|p| p.kind());

            for promotion_type in Kind::PROMOTION_KINDS {
                move_buffer.push(
                    MoveBuilder::new(sq + 7, sq)
                        .with_optional_capture(captured_type)
                        .with_promotion(promotion_type)
                        .build(),
                );
            }
        }
    }

    fn generate_static_moves(
        &self,
        color: Color,
        kind: Kind,
        piece_masks: [Bitboard; 64],
        move_buffer: MoveBuffer<'_>,
    ) {
        let piece_bitboard: Bitboard = self.bitboards[Piece::new(color, kind).to_index()];

        for sq in piece_bitboard {
            let piece_mask: Bitboard = piece_masks[usize::from(sq)];

            let quiet_squares: Bitboard = piece_mask & !self.all_pieces_bitboard;

            for quiet_sq in quiet_squares {
                move_buffer.push(MoveBuilder::new(sq, quiet_sq).build());
            }

            let capture_squares: Bitboard = piece_mask & self.side_bitboards[(!color) as usize];

            for cap_sq in capture_squares {
                let captured_type: Option<Kind> = self.get_piece_at(cap_sq).map(|p| p.kind());

                move_buffer.push(
                    MoveBuilder::new(sq, cap_sq)
                        .with_optional_capture(captured_type)
                        .build(),
                );
            }
        }
    }

    fn get_sliding_attacks(&self, square: Square, rays_masks: &[[Bitboard; 4]; 64]) -> Bitboard {
        let mut attacks: Bitboard = Bitboard::new(0);

        let rays: &[Bitboard; 4] = unsafe { rays_masks.get_unchecked(usize::from(square)) };

        for (i, &ray) in rays.iter().enumerate() {
            let bitboard_ray: Bitboard = self.all_pieces_bitboard & ray;

            if bitboard_ray.is_empty() {
                attacks |= ray;
                continue;
            }

            let blocker_path: Bitboard = if i < 2 {
                let sq: u8 = u8::from(bitboard_ray.first_square_unchecked());
                let mask: u64 = 1u64.checked_shl((sq + 1) as u32).map(|v| v - 1).unwrap_or(u64::MAX);

                Bitboard::new(mask)
            } else {
                Bitboard::new(!((1u64 << u8::from(bitboard_ray.last_square_unchecked())) - 1))
            };

            attacks |= ray & blocker_path;
        }

        attacks
    }

    fn generate_sliding_moves(
        &self,
        color: Color,
        kind: Kind,
        rays_masks: &[[Bitboard; 4]; 64],
        move_buffer: MoveBuffer<'_>,
    ) {
        let piece_bitboard: Bitboard = self.bitboards[Piece::new(color, kind).to_index()];

        for sq in piece_bitboard {
            let sliding_attacks: Bitboard = self.get_sliding_attacks(sq, rays_masks);

            let quiet_squares: Bitboard = sliding_attacks & !self.all_pieces_bitboard;

            for quiet_sq in quiet_squares {
                move_buffer.push(MoveBuilder::new(sq, quiet_sq).build());
            }

            let capture_squares: Bitboard = sliding_attacks & self.side_bitboards[(!color) as usize];

            for cap_sq in capture_squares {
                let captured_type: Option<Kind> = self.get_piece_at(cap_sq).map(|p| p.kind());

                move_buffer.push(
                    MoveBuilder::new(sq, cap_sq)
                        .with_optional_capture(captured_type)
                        .build(),
                );
            }
        }
    }

    fn generate_queen_moves(&self, color: Color, move_buffer: MoveBuffer<'_>) {
        let piece_bitboard: Bitboard = self.bitboards[Piece::new(color, Kind::Queen).to_index()];

        for sq in piece_bitboard {
            let sliding_attacks: Bitboard = self.get_queen_attacks(sq);

            let quiet_squares: Bitboard = sliding_attacks & !self.all_pieces_bitboard;

            for quiet_sq in quiet_squares {
                move_buffer.push(MoveBuilder::new(sq, quiet_sq).build());
            }

            let capture_squares: Bitboard = sliding_attacks & self.side_bitboards[(!color) as usize];

            for cap_sq in capture_squares {
                let captured_type: Option<Kind> = self.get_piece_at(cap_sq).map(|p| p.kind());

                move_buffer.push(
                    MoveBuilder::new(sq, cap_sq)
                        .with_optional_capture(captured_type)
                        .build(),
                );
            }
        }
    }

    fn generate_king_moves(&self, color: Color, move_buffer: MoveBuffer<'_>) {
        self.generate_static_moves(color, Kind::King, KING_MASKS, move_buffer);

        let color_shift: u8 = color.shift();
        let castle_square: Square = Square::E1 + color_shift;

        let king_bitboard: Bitboard = self.bitboards[Piece::new(color, Kind::King).to_index()];
        let king_square: Square = king_bitboard.first_square_unchecked();

        if king_square != castle_square {
            return;
        }

        if self.is_square_attacked(king_square, !color) {
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
            if self.is_square_attacked(passed_square, !color) {
                continue;
            }

            let mask: Bitboard = castle.xor_mask().king_mask << color_shift;
            let to_square: Square = (king_bitboard ^ mask).first_square_unchecked();

            move_buffer.push(MoveBuilder::new(king_square, to_square).with_castling(castle).build());
        }
    }

    fn get_pawn_attacks(square: Square, color: Color) -> Bitboard {
        unsafe { *PAWN_ATTACKERS_MASKS[color as usize].get_unchecked(usize::from(square)) }
    }

    fn get_knight_attacks(square: Square) -> Bitboard {
        unsafe { *KNIGHT_MASKS.get_unchecked(usize::from(square)) }
    }

    fn get_bishop_attacks(&self, square: Square) -> Bitboard {
        self.get_sliding_attacks(square, &BISHOP_RAYS_MASKS)
    }

    fn get_rook_attacks(&self, square: Square) -> Bitboard {
        self.get_sliding_attacks(square, &ROOK_RAYS_MASKS)
    }

    fn get_queen_attacks(&self, square: Square) -> Bitboard {
        self.get_bishop_attacks(square) | self.get_rook_attacks(square)
    }

    fn get_king_attacks(square: Square) -> Bitboard {
        unsafe { *KING_MASKS.get_unchecked(usize::from(square)) }
    }

    pub fn is_square_attacked(&self, square: Square, opposite_color: Color) -> bool {
        // Pawn
        if !(Self::get_pawn_attacks(square, opposite_color)
            & self.bitboards[Piece::new(opposite_color, Kind::Pawn).to_index()])
        .is_empty()
        {
            return true;
        }

        // Knight
        if !(Self::get_knight_attacks(square) & self.bitboards[Piece::new(opposite_color, Kind::Knight).to_index()])
            .is_empty()
        {
            return true;
        }

        // Bishop and Queen
        if !(self.get_bishop_attacks(square)
            & (self.bitboards[Piece::new(opposite_color, Kind::Bishop).to_index()]
                | self.bitboards[Piece::new(opposite_color, Kind::Queen).to_index()]))
        .is_empty()
        {
            return true;
        }

        // Rook and Queen
        if !(self.get_rook_attacks(square)
            & (self.bitboards[Piece::new(opposite_color, Kind::Rook).to_index()]
                | self.bitboards[Piece::new(opposite_color, Kind::Queen).to_index()]))
        .is_empty()
        {
            return true;
        }

        // King
        if !(Self::get_king_attacks(square) & self.bitboards[Piece::new(opposite_color, Kind::King).to_index()])
            .is_empty()
        {
            return true;
        }

        false
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

        let undo_info: UndoInfo = UndoInfo {
            castling_flags: self.castling_flags,
            en_passant_square: self.en_passant_square,
            fifty_move_counter: self.fifty_move_counter,
        };

        self.undo_info_stack.push(undo_info);

        self.clear_piece(target_piece, from_square);

        if let Some(captured_type) = piece_move.captured_type() {
            let square: Square = if piece_move.is_en_passant() {
                en_passant_square
            } else {
                to_square
            };

            let opposite_piece: Piece = Piece::new(!target_piece_color, captured_type);

            self.clear_piece(opposite_piece, square);
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

        let is_king_moved: bool = target_piece.kind() == Kind::King;

        for castle in CastlingType::ALL_CASTLING_TYPES {
            let old_flag: bool = self.castling_flags.can_castle(target_piece_color, castle);

            self.castling_flags
                .update_castling(target_piece_color, castle, old_flag & !is_king_moved);
        }

        for color in Color::ALL_COLORS {
            for castle in CastlingType::ALL_CASTLING_TYPES {
                let color_shift: u8 = color.shift();
                let old_flag: bool = self.castling_flags.can_castle(color, castle);

                let is_rook_on_start_square: bool = self.bitboards[Piece::new(color, Kind::Rook).to_index()]
                    .is_square_set(castle.rook_start_square() + color_shift);

                self.castling_flags
                    .update_castling(color, castle, old_flag & is_rook_on_start_square);
            }
        }

        let is_double_push: bool =
            target_piece.kind() == Kind::Pawn && u8::from(to_square) ^ u8::from(from_square) == 16;
        self.en_passant_square = is_double_push.then_some(en_passant_square);

        let reset_mask: bool = !(target_piece.kind() == Kind::Pawn || piece_move.captured_type().is_some());
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

        if let Some(captured_type) = piece_move.captured_type() {
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
            let sq: Square = Square::new(i as u8);

            match piece {
                Some(p) => {
                    let bb: Bitboard = self.bitboards[p.to_index()];
                    if !bb.is_square_set(sq) {
                        return false;
                    }

                    for (i, bb) in self.bitboards.iter().enumerate() {
                        if i == p.to_index() {
                            continue;
                        }

                        if bb.is_square_set(sq) {
                            return false;
                        }
                    }

                    let s_bb: Bitboard = self.side_bitboards[p.color() as usize];
                    if !s_bb.is_square_set(sq) {
                        return false;
                    }

                    if self.side_bitboards[!p.color() as usize].is_square_set(sq) {
                        return false;
                    }

                    if !self.all_pieces_bitboard.is_square_set(sq) {
                        return false;
                    }
                }
                None => {
                    for bb in self.bitboards {
                        if bb.is_square_set(sq) {
                            return false;
                        }
                    }

                    for bb in self.side_bitboards {
                        if bb.is_square_set(sq) {
                            return false;
                        }
                    }

                    if self.all_pieces_bitboard.is_square_set(sq) {
                        return false;
                    }
                }
            }
        }

        true
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
