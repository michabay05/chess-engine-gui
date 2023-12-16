use crate::attack::AttackInfo;
use crate::bb::{BBUtil, BB};
use crate::board::{self, Board, CastlingType};
use crate::consts::{Direction, Piece, PieceColor, Sq};
use crate::moves::{Move, MoveUtil};

pub struct MoveList {
    pub moves: Vec<Move>,
}

impl MoveList {
    pub fn new() -> Self {
        Self { moves: vec![] }
    }

    pub fn print(&self) {
        println!("    Source   |   Target  |  Piece  |  Promoted  |  Capture  |  Two Square Push  |  Enpassant  |  Castling");
        println!("  -----------------------------------------------------------------------------------------------------------");
        #[rustfmt::skip]
        fn y_or_n(sth: bool) -> char { if sth {'1'} else {' '} }
        for el in &self.moves {
            println!("       {}    |    {}     |    {}    |     {}      |     {}     |         {}         |      {}      |     {}",
                el.source(),
                el.target(),
                Piece::to_char(Some(el.piece())),
                Piece::to_char(el.promoted()),
                y_or_n(el.is_capture()),
                y_or_n(el.is_twosquare()),
                y_or_n(el.is_enpassant()),
                y_or_n(el.is_castling())
            );
        }
        println!("\n    Total number of moves: {}", self.moves.len());
    }

    pub fn search(&mut self, source: Sq, target: Sq, promoted: Option<Piece>) -> Option<Move> {
        self.moves
            .iter()
            .find(|mv| mv.source() == source && mv.target() == target && mv.promoted() == promoted)
            .copied()
    }
}

pub fn generate_by_piece(board: &Board, attack_info: &AttackInfo, ml: &mut MoveList, piece: Piece) {
    match piece {
        Piece::LP | Piece::DP => generate_pawns(board, attack_info, ml),
        Piece::LN | Piece::DN => generate_knights(board, attack_info, ml),
        Piece::LB | Piece::DB => generate_bishops(board, attack_info, ml),
        Piece::LR | Piece::DR => generate_rooks(board, attack_info, ml),
        Piece::LQ | Piece::DQ => generate_queens(board, attack_info, ml),
        Piece::LK | Piece::DK => generate_kings(board, attack_info, ml)
    }
}

pub fn generate_all(board: &Board, attack_info: &AttackInfo, ml: &mut MoveList) {
    generate_pawns(board, attack_info, ml);
    generate_knights(board, attack_info, ml);
    generate_bishops(board, attack_info, ml);
    generate_rooks(board, attack_info, ml);
    generate_queens(board, attack_info, ml);
    generate_kings(board, attack_info, ml);
}

fn generate_pawns(board: &Board, attack_info: &AttackInfo, ml: &mut MoveList) {
    const PROMOTED_PIECE_LIST: [[Piece; 4]; 2] = [
        [Piece::LQ, Piece::LR, Piece::LB, Piece::LN],
        [Piece::DQ, Piece::DR, Piece::DB, Piece::DN],
    ];

    let mut bb_copy: BB;
    let mut attack_copy: BB;
    let promotion_start; // The first square of the promoting rank
    let twosquarepush_start; // The first square of the current side's pawn starting square
    let enemy_rank_start; // The first square of the current side's pawn starting square
    let direction: Direction;
    let piece: Piece;
    let enemy_color: PieceColor;
    let is_white = board.state.side == PieceColor::Light;
    if is_white {
        piece = Piece::LP;
        enemy_color = PieceColor::Dark;
        promotion_start = Sq::A7;
        twosquarepush_start = Sq::A2;
        enemy_rank_start = Sq::A8;
        direction = Direction::South;
    } else {
        piece = Piece::DP;
        enemy_color = PieceColor::Light;
        promotion_start = Sq::A2;
        twosquarepush_start = Sq::A7;
        enemy_rank_start = Sq::H1;
        direction = Direction::North;
    }

    bb_copy = board.pos.piece[piece as usize];
    let mut source: i32;
    let mut target: i32;

    while bb_copy > 0 {
        source = bb_copy.pop_lsb() as i32;
        target = source + direction as i32;
        let are_pawns_in_bound = if is_white {
            target >= enemy_rank_start as i32
        } else {
            target <= enemy_rank_start as i32
        };
        if are_pawns_in_bound && !board.pos.units[PieceColor::Both as usize].get(target as usize) {
            // If true, this move is a promotion
            if (source >= promotion_start as i32) && (source <= (promotion_start as i32 + 7)) {
                let ind = if is_white { 0 } else { 1 };
                for i in 0..4 {
                    ml.moves.push(Move::encode(
                        Sq::from_num(source as usize),
                        Sq::from_num(target as usize),
                        piece,
                        Some(PROMOTED_PIECE_LIST[ind][i]),
                        false,
                        false,
                        false,
                        false,
                    ));
                }
            } else {
                // If false, this is a normal(quiet or non-capture) move
                ml.moves.push(Move::encode(
                    Sq::from_num(source as usize),
                    Sq::from_num(target as usize),
                    piece,
                    None,
                    false,
                    false,
                    false,
                    false,
                ));
                let source_is_in_bound = (source >= twosquarepush_start as i32)
                    && (source <= (twosquarepush_start as i32 + 7));
                if source_is_in_bound
                    && !board.pos.units[PieceColor::Both as usize]
                        .get((target + (direction as i32)) as usize)
                {
                    ml.moves.push(Move::encode(
                        Sq::from_num(source as usize),
                        Sq::from_num((target + (direction as i32)) as usize),
                        piece,
                        None,
                        false,
                        true,
                        false,
                        false,
                    ));
                }
            }
        }

        attack_copy = attack_info.pawn[board.state.side as usize][source as usize]
            & board.pos.units[enemy_color as usize];
        while attack_copy > 0 {
            target = attack_copy.pop_lsb() as i32;
            if (source >= promotion_start as i32) && (source <= (promotion_start as i32 + 7)) {
                // If true, this move is a capture promotion
                let ind = if is_white { 0 } else { 1 };
                for i in 0..4 {
                    ml.moves.push(Move::encode(
                        Sq::from_num(source as usize),
                        Sq::from_num(target as usize),
                        piece,
                        Some(PROMOTED_PIECE_LIST[ind][i]),
                        true,
                        false,
                        false,
                        false,
                    ));
                }
            } else {
                // If false, this move is a normal capture
                ml.moves.push(Move::encode(
                    Sq::from_num(source as usize),
                    // Sq::from_num((target + (direction as i32)) as usize),
                    Sq::from_num(target as usize),
                    piece,
                    None,
                    true,
                    false,
                    false,
                    false,
                ));
            }
        }
        if board.state.enpassant != Sq::NoSq {
            let enpassant_capture = attack_info.pawn[board.state.side as usize][source as usize]
                & (1 << (board.state.enpassant as usize));
            if enpassant_capture != 0 {
                let enpassant_target = enpassant_capture.lsb();
                ml.moves.push(Move::encode(
                    Sq::from_num(source as usize),
                    Sq::from_num(enpassant_target),
                    piece,
                    None,
                    true,
                    false,
                    true,
                    false,
                ));
            }
        }
    }
}

fn generate_knights(board: &Board, attack_info: &AttackInfo, ml: &mut MoveList) {
    let mut source;
    let mut target;
    let is_white = board.state.side == PieceColor::Light;
    let piece = if is_white { Piece::LN } else { Piece::DN };
    let color = if is_white {
        PieceColor::Light
    } else {
        PieceColor::Dark
    };
    let enemy_color = if is_white {
        PieceColor::Dark
    } else {
        PieceColor::Light
    };
    let mut bb_copy = board.pos.piece[piece as usize];
    let mut attack_copy;

    while bb_copy > 0 {
        source = bb_copy.pop_lsb();
        attack_copy = attack_info.knight[source] & (!board.pos.units[color as usize]);
        while attack_copy > 0 {
            target = attack_copy.pop_lsb();
            let is_capture_move = board.pos.units[enemy_color as usize].get(target);
            ml.moves.push(Move::encode(
                Sq::from_num(source),
                Sq::from_num(target),
                piece,
                None,
                is_capture_move,
                false,
                false,
                false,
            ));
        }
    }
}

fn generate_bishops(board: &Board, attack_info: &AttackInfo, ml: &mut MoveList) {
    let mut source;
    let mut target;
    let is_white = board.state.side == PieceColor::Light;
    let piece = if is_white { Piece::LB } else { Piece::DB };
    let color = if is_white {
        PieceColor::Light
    } else {
        PieceColor::Dark
    };
    let enemy_color = if is_white {
        PieceColor::Dark
    } else {
        PieceColor::Light
    };
    let mut bb_copy = board.pos.piece[piece as usize];
    let mut attack_copy;

    while bb_copy > 0 {
        source = bb_copy.pop_lsb();
        attack_copy = attack_info.get_bishop_attack(
            Sq::from_num(source),
            board.pos.units[PieceColor::Both as usize],
        ) & (!board.pos.units[color as usize]);
        while attack_copy > 0 {
            target = attack_copy.pop_lsb();
            let is_capture_move = board.pos.units[enemy_color as usize].get(target);
            ml.moves.push(Move::encode(
                Sq::from_num(source),
                Sq::from_num(target),
                piece,
                None,
                is_capture_move,
                false,
                false,
                false,
            ));
        }
    }
}

fn generate_rooks(board: &Board, attack_info: &AttackInfo, ml: &mut MoveList) {
    let mut source;
    let mut target;
    let is_white = board.state.side == PieceColor::Light;
    let piece = if is_white { Piece::LR } else { Piece::DR };
    let color = if is_white {
        PieceColor::Light
    } else {
        PieceColor::Dark
    };
    let enemy_color = if is_white {
        PieceColor::Dark
    } else {
        PieceColor::Light
    };
    let mut bb_copy = board.pos.piece[piece as usize];
    let mut attack_copy;

    while bb_copy > 0 {
        source = bb_copy.pop_lsb();
        attack_copy = attack_info.get_rook_attack(
            Sq::from_num(source),
            board.pos.units[PieceColor::Both as usize],
        ) & (!board.pos.units[color as usize]);
        while attack_copy > 0 {
            target = attack_copy.pop_lsb();
            let is_capture_move = board.pos.units[enemy_color as usize].get(target);
            ml.moves.push(Move::encode(
                Sq::from_num(source),
                Sq::from_num(target),
                piece,
                None,
                is_capture_move,
                false,
                false,
                false,
            ));
        }
    }
}

fn generate_queens(board: &Board, attack_info: &AttackInfo, ml: &mut MoveList) {
    let mut source;
    let mut target;
    let is_white = board.state.side == PieceColor::Light;
    let piece = if is_white { Piece::LQ } else { Piece::DQ };
    let color = if is_white {
        PieceColor::Light
    } else {
        PieceColor::Dark
    };
    let enemy_color = if is_white {
        PieceColor::Dark
    } else {
        PieceColor::Light
    };
    let mut bb_copy = board.pos.piece[piece as usize];
    let mut attack_copy;

    while bb_copy > 0 {
        source = bb_copy.pop_lsb();
        attack_copy = attack_info.get_queen_attack(
            Sq::from_num(source),
            board.pos.units[PieceColor::Both as usize],
        ) & (!board.pos.units[color as usize]);
        while attack_copy > 0 {
            target = attack_copy.pop_lsb();
            let is_capture_move = board.pos.units[enemy_color as usize].get(target);
            ml.moves.push(Move::encode(
                Sq::from_num(source),
                Sq::from_num(target),
                piece,
                None,
                is_capture_move,
                false,
                false,
                false,
            ));
        }
    }
}

fn generate_kings(board: &Board, attack_info: &AttackInfo, ml: &mut MoveList) {
    let mut source;
    let mut target;
    let is_white = board.state.side == PieceColor::Light;
    let piece = if is_white { Piece::LK } else { Piece::DK };
    let color = if is_white {
        PieceColor::Light
    } else {
        PieceColor::Dark
    };
    let enemy_color = if is_white {
        PieceColor::Dark
    } else {
        PieceColor::Light
    };
    let mut bb_copy = board.pos.piece[piece as usize];
    let mut attack_copy;

    while bb_copy > 0 {
        source = bb_copy.pop_lsb();
        attack_copy = attack_info.king[source] & (!board.pos.units[color as usize]);
        while attack_copy > 0 {
            target = attack_copy.pop_lsb();
            let is_capture_move = board.pos.units[enemy_color as usize].get(target);
            ml.moves.push(Move::encode(
                Sq::from_num(source),
                Sq::from_num(target),
                piece,
                None,
                is_capture_move,
                false,
                false,
                false,
            ));
        }
    }
    if is_white {
        gen_light_castling(board, attack_info, ml);
    } else {
        gen_dark_castling(board, attack_info, ml);
    }
}

fn gen_light_castling(board: &Board, attack_info: &AttackInfo, ml: &mut MoveList) {
    let castling = board.state.castling as BB;
    if castling.get(CastlingType::WhiteKingside as usize) {
        if !board.pos.units[PieceColor::Both as usize].get(Sq::F1 as usize)
            && !board.pos.units[PieceColor::Both as usize].get(Sq::G1 as usize)
        {
            if !board::sq_attacked(&board.pos, attack_info, Sq::E1, PieceColor::Dark)
                && !board::sq_attacked(&board.pos, attack_info, Sq::F1, PieceColor::Dark)
            {
                ml.moves
                    .push(Move::from_str("e1g1", Piece::LK, false, false, false, true));
            }
        }
    }

    if castling.get(CastlingType::WhiteQueenside as usize) {
        if !board.pos.units[PieceColor::Both as usize].get(Sq::B1 as usize)
            && !board.pos.units[PieceColor::Both as usize].get(Sq::C1 as usize)
            && !board.pos.units[PieceColor::Both as usize].get(Sq::D1 as usize)
        {
            if !board::sq_attacked(&board.pos, attack_info, Sq::D1, PieceColor::Dark)
                && !board::sq_attacked(&board.pos, attack_info, Sq::E1, PieceColor::Dark)
            {
                ml.moves
                    .push(Move::from_str("e1c1", Piece::LK, false, false, false, true));
            }
        }
    }
}

fn gen_dark_castling(board: &Board, attack_info: &AttackInfo, ml: &mut MoveList) {
    let castling = board.state.castling as BB;
    if castling.get(CastlingType::BlackKingside as usize) {
        if !board.pos.units[PieceColor::Both as usize].get(Sq::F8 as usize)
            && !board.pos.units[PieceColor::Both as usize].get(Sq::G8 as usize)
        {
            if !board::sq_attacked(&board.pos, attack_info, Sq::E8, PieceColor::Light)
                && !board::sq_attacked(&board.pos, attack_info, Sq::F8, PieceColor::Light)
            {
                ml.moves
                    .push(Move::from_str("e8g8", Piece::DK, false, false, false, true));
            }
        }
    }

    if castling.get(CastlingType::BlackQueenside as usize) {
        if !board.pos.units[PieceColor::Both as usize].get(Sq::B8 as usize)
            && !board.pos.units[PieceColor::Both as usize].get(Sq::C8 as usize)
            && !board.pos.units[PieceColor::Both as usize].get(Sq::D8 as usize)
        {
            if !board::sq_attacked(&board.pos, attack_info, Sq::D8, PieceColor::Light)
                && !board::sq_attacked(&board.pos, attack_info, Sq::E8, PieceColor::Light)
            {
                ml.moves
                    .push(Move::from_str("e8c8", Piece::DK, false, false, false, true));
            }
        }
    }
}
