use crate::bb::{BB, BBUtil};
use crate::board::{Board, CastlingType, Position};
use crate::consts::{Piece, PieceColor, Sq};
use crate::zobrist::{self, ZobristInfo};
use crate::SQ;

pub const FEN_POSITIONS: [&str; 8] = [
    "8/8/8/8/8/8/8/8 w - - 0 1",
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
    "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
    "rnbqkb1r/pp1p1pPp/8/2p1pP2/1P1P4/3P3P/P1P1P3/RNBQKBNR w KQkq e6 0 1",
];

pub fn parse(fen: &str, zobrist_info: &ZobristInfo) -> Board {
    let mut board: Board = Board::new();
    let mut fen_parts = fen.split_ascii_whitespace().into_iter();

    // Place piece on square
    parse_pieces(fen_parts.next().unwrap(), &mut board.pos);

    // Set side to move
    let side_to_move_str: &str = fen_parts.next().unwrap();
    if side_to_move_str == "w" {
        board.state.side = PieceColor::Light;
        board.state.xside = PieceColor::Dark;
    } else if side_to_move_str == "b" {
        board.state.side = PieceColor::Dark;
        board.state.xside = PieceColor::Light;
    }

    // Set castling right
    for castling_type in fen_parts.next().unwrap().chars().into_iter() {
        if castling_type == 'K' {
            board
                .state
                .toggle_castling(CastlingType::WhiteKingside as usize);
        } else if castling_type == 'Q' {
            board
                .state
                .toggle_castling(CastlingType::WhiteQueenside as usize);
        } else if castling_type == 'k' {
            board
                .state
                .toggle_castling(CastlingType::BlackKingside as usize);
        } else if castling_type == 'q' {
            board
                .state
                .toggle_castling(CastlingType::BlackQueenside as usize);
        }
    }

    // Set enpassant square
    let enpass_square = fen_parts.next().unwrap();
    if enpass_square != "-" {
        board.state.enpassant = Sq::from_str(enpass_square);
    }
    // Set 50 move rule
    let half_moves = fen_parts.next().unwrap().parse::<u32>().unwrap();
    board.state.half_moves = half_moves;
    // Set move counter
    let full_moves = fen_parts.next().unwrap().parse::<u32>().unwrap();
    board.state.full_moves = full_moves;

    // Update units bitboard from piece bitboard
    board.pos.update_units();

    board.state.key = zobrist::gen_board_key(&zobrist_info.key, &board);
    board.state.lock = zobrist::gen_board_lock(&zobrist_info.lock, &board);

    board
}

fn parse_pieces(fen_piece: &str, pos: &mut Position) {
    let mut sq: u8 = 0;
    for piece_char in fen_piece.chars().into_iter() {
        if piece_char == '/' {
        } else if piece_char.is_ascii_digit() {
            // Retrieve the int value of the offset from the char value
            let offset: u8 = piece_char as u8 - '0' as u8;
            // Add offset value to square counter
            sq += offset;
        } else if piece_char.is_ascii_alphabetic() {
            let (piece_color, _) = Piece::to_tuple(Piece::from_char(piece_char));
            if piece_color == PieceColor::Both as usize {
                continue;
            }
            pos.piece[Piece::from_char(piece_char).unwrap() as usize].set(sq as usize);
            // Increment the current square
            sq += 1;
        }
    }
}

pub fn gen_fen(board: &Board) -> String {
    let mut output = String::new();

    // Piece placements
    for r in 0..8 {
        let mut empty_sqs = 0;
        for f in 0..8 {
            if let Some(piece) = board.find_piece(SQ!(r, f)) {
                if empty_sqs != 0 {
                    output.push(char::from_digit(empty_sqs, 10).unwrap());
                }
                empty_sqs = 0;
                output.push(Piece::to_char(Some(piece)));
            } else {
                empty_sqs += 1;
            }
        }
        if empty_sqs != 0 {
            output.push(char::from_digit(empty_sqs, 10).unwrap());
        }
        if r != 7 {
            output.push('/');
        }
    }

    output.push(' ');

    // Side to move
    if board.state.side == PieceColor::Light {
        output.push('w');
    } else {
        output.push('b');
    }

    output.push(' ');

    let castling = board.state.castling as BB;
    if castling != 0 {
        if castling.get(CastlingType::WhiteKingside as usize) {
            output.push('K');
        }
        if castling.get(CastlingType::WhiteQueenside as usize) {
            output.push('Q');
        }
        if castling.get(CastlingType::BlackKingside as usize) {
            output.push('k');
        }
        if castling.get(CastlingType::BlackQueenside as usize) {
            output.push('q');
        }
    } else {
        output.push('-');
    }

    output.push(' ');

    // Enpassant square
    if board.state.enpassant != Sq::NoSq {
        output.push_str(&Sq::to_string(board.state.enpassant));
    } else {
        output.push('-');
    }

    // Half move counter and full move counter
    output.push_str(&format!(" {} {}", board.state.half_moves, board.state.full_moves));

    output
}
