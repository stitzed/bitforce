use std::fmt::Display;
use std::str::FromStr;

use primitive_buffer::Buffer;

use crate::bitboard::{
    BISHOP_RAYS_MASKS, BLACK_PAWN_ATTACKERS_MASKS, Bitboard, FILE_A, FILE_H, KING_MASKS, KNIGHT_MASKS, RANK_1, RANK_3,
    RANK_6, RANK_8, ROOK_RAYS_MASKS, WHITE_PAWN_ATTACKERS_MASKS,
};
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
        };

        let mut total_parts: u8 = 0;

        for (i, part) in fen.split_whitespace().enumerate() {
            total_parts += 1;

            match i {
                FEN_PIECE_SETUP_IDX => {
                    Self::parse_piece_setup(&mut board, part)?;
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

    fn parse_piece_setup<'a>(board: &mut Self, pieces_setup: &'a str) -> Result<(), FenParseError<'a>> {
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

                    board.board[usize::from(square)] = Some(piece);
                    board.bitboards[piece.to_index()] |= Bitboard::new(square.to_bitboard_mask());
                    board.side_bitboards[color.to_index()] |= Bitboard::new(square.to_bitboard_mask());

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

        board.all_pieces_bitboard = board.side_bitboards[0] | board.side_bitboards[1];

        Ok(())
    }

    #[inline(always)]
    pub fn get_piece_at(&self, square: Square) -> Option<Piece> {
        self.board[usize::from(square)]
    }

    pub fn generate_pseudo_legal_moves<'a>(
        &self,
        color: Color,
        move_buffer: &'a mut Buffer<Move, MOVE_BUFFER_LEN>,
    ) -> &'a [Move] {
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

    fn generate_pawn_moves(&self, color: Color, move_buffer: &mut Buffer<Move, MOVE_BUFFER_LEN>) {
        match color {
            Color::White => self.generate_white_pawn_moves(move_buffer),
            Color::Black => self.generate_black_pawn_moves(move_buffer),
        }
    }

    #[inline]
    fn generate_white_pawn_moves(&self, move_buffer: &mut Buffer<Move, MOVE_BUFFER_LEN>) {
        let pawn_bitboard: Bitboard = self.bitboards[Piece::new(Color::White, Kind::Pawn).to_index()];
        let opposite_bitboard: Bitboard = self.side_bitboards[Color::Black.to_index()];

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
    fn generate_black_pawn_moves(&self, move_buffer: &mut Buffer<Move, MOVE_BUFFER_LEN>) {
        let pawn_bitboard: Bitboard = self.bitboards[Piece::new(Color::Black, Kind::Pawn).to_index()];
        let opposite_bitboard: Bitboard = self.side_bitboards[Color::White.to_index()];

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
        move_buffer: &mut Buffer<Move, MOVE_BUFFER_LEN>,
    ) {
        let piece_bitboard: Bitboard = self.bitboards[Piece::new(color, kind).to_index()];

        for sq in piece_bitboard {
            let piece_mask: Bitboard = piece_masks[usize::from(sq)];

            let quiet_squares: Bitboard = piece_mask & !self.all_pieces_bitboard;

            for quiet_sq in quiet_squares {
                move_buffer.push(MoveBuilder::new(sq, quiet_sq).build());
            }

            let capture_squares: Bitboard = piece_mask & self.side_bitboards[(!color).to_index()];

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
                Bitboard::new((1u64 << (u8::from(bitboard_ray.first_square_unchecked()) + 1)) - 1)
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
        move_buffer: &mut Buffer<Move, MOVE_BUFFER_LEN>,
    ) {
        let piece_bitboard: Bitboard = self.bitboards[Piece::new(color, kind).to_index()];

        for sq in piece_bitboard {
            let sliding_attacks: Bitboard = self.get_sliding_attacks(sq, rays_masks);

            let quiet_squares: Bitboard = sliding_attacks & !self.all_pieces_bitboard;

            for quiet_sq in quiet_squares {
                move_buffer.push(MoveBuilder::new(sq, quiet_sq).build());
            }

            let capture_squares: Bitboard = sliding_attacks & self.side_bitboards[(!color).to_index()];

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

    fn generate_queen_moves(&self, color: Color, move_buffer: &mut Buffer<Move, MOVE_BUFFER_LEN>) {
        let piece_bitboard: Bitboard = self.bitboards[Piece::new(color, Kind::Queen).to_index()];

        for sq in piece_bitboard {
            let rook_attacks: Bitboard = self.get_sliding_attacks(sq, &ROOK_RAYS_MASKS);
            let bishop_attacks: Bitboard = self.get_sliding_attacks(sq, &BISHOP_RAYS_MASKS);

            let sliding_attacks: Bitboard = rook_attacks | bishop_attacks;

            let quiet_squares: Bitboard = sliding_attacks & !self.all_pieces_bitboard;

            for quiet_sq in quiet_squares {
                move_buffer.push(MoveBuilder::new(sq, quiet_sq).build());
            }

            let capture_squares: Bitboard = sliding_attacks & self.side_bitboards[(!color).to_index()];

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

    fn generate_king_moves(&self, color: Color, move_buffer: &mut Buffer<Move, MOVE_BUFFER_LEN>) {
        self.generate_static_moves(color, Kind::King, KING_MASKS, move_buffer);

        let (castle_square, mask_shift) = match color {
            Color::White => (Square::E1, 0),
            Color::Black => (Square::E8, 8 * 7),
        };

        let king_bitboard: Bitboard = self.bitboards[Piece::new(color, Kind::King).to_index()];
        let king_square: Square = king_bitboard.first_square_unchecked();

        if king_square != castle_square {
            return;
        }

        for castle in CastlingType::ALL_CASTLING_TYPES {
            if !self.castling_flags.can_castle(color, castle) {
                continue;
            }

            if ((castle.path_mask() << mask_shift) & self.all_pieces_bitboard).is_empty() {
                let mask: Bitboard = castle.xor_mask().king_mask << mask_shift;
                let to_square: Square = (king_bitboard ^ mask).first_square_unchecked();

                move_buffer.push(MoveBuilder::new(king_square, to_square).with_castling(castle).build());
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
