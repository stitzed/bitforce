use primitive_buffer::Buffer;

use super::ChessBoard;
use crate::bitboard::prelude::*;
use crate::castle::CastlingType;
use crate::piece::{BlackPawnDirections, Color, Kind, PawnDirections, Piece, WhitePawnDirections};
use crate::piece_move::{Move, MoveBuilder};
use crate::square::Square;

pub const MOVE_BUFFER_LEN: usize = 256;

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
        let slider: Bitboard = Bitboard::from(square);

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

impl ChessBoard {
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

                legal_squares[usize::from(pinned_sq)] = squares_between | Bitboard::from(pinner);
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

                legal_squares[usize::from(pinned_sq)] = squares_between | Bitboard::from(pinner);
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

                SQUARES_BETWEEN[usize::from(checker)][usize::from(king_square)] | Bitboard::from(checker)
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
}
