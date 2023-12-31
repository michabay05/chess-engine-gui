use chess::attack::{self, AttackInfo};
use chess::bb::BBUtil;
use chess::board::Board;
use chess::moves::{Move, MoveUtil};
use chess::consts::{Piece, PieceColor, Sq};
use chess::fen;
use chess::COL;

use crate::game::{Game, GameState};

use std::path::Path;
use std::io::{self, BufWriter, Write};

fn should_disambiguate(mv: Move, attack_info: &AttackInfo, board: &Board) -> (bool, bool) {
    let piece = mv.piece();
    if (piece == Piece::LP || piece == Piece::DP) || (piece == Piece::LK || piece == Piece::DK) {
        return (false, false);
    }
    let source = mv.source();
    let target = mv.target();
    let bb = match piece {
        Piece::LN | Piece::DN => attack_info.knight[target as usize],
        Piece::LB | Piece::DB => {
            attack_info.get_bishop_attack(target, board.pos.units[PieceColor::Both as usize])
        },
        Piece::LR | Piece::DR => {
            attack_info.get_rook_attack(target, board.pos.units[PieceColor::Both as usize])
        },
        Piece::LQ | Piece::DQ => {
            attack_info.get_queen_attack(target, board.pos.units[PieceColor::Both as usize])
        },
        _ => unreachable!(),
    };
    let piece_bb = board.pos.piece[piece as usize];
    let mut result = bb & piece_bb;
    if result.count_ones() <= 1 {
        (false, false)
    } else if result.count_ones() == 2 {
        let a = result.pop_lsb();
        let b = result.pop_lsb();
        if COL!(a) == COL!(b) {
            (true, false)
        } else {
            (false, true)
        }
    } else {
        (true, true)
    }
}

fn coord_move_to_san(
    mv: Move, attack_info: &AttackInfo, check: bool,
    (dis_row, dis_col): (bool, bool), checkmate: bool
) -> String {
    let source = mv.source();
    let target = mv.target();
    let piece = mv.piece();
    if mv.is_castling() {
        if COL!(target as usize) == 6 {
            return "O-O".to_string();
        } else if COL!(target as usize) == 2 {
            return "O-O-O".to_string();
        }
    }
    let mut output = String::new();
    // Piece
    if piece != Piece::LP && piece != Piece::DP {
        output.push(Piece::to_char(Some(piece)).to_ascii_uppercase());
    }
    // Disambiguation
    if dis_row || dis_col {
        let mut sq_str = Sq::to_string(source);
        if dis_col {
            output.push(sq_str.remove(0));
        }
        if dis_row {
            output.push(sq_str.remove(1));
        }
    }
    // Capture
    if mv.is_capture() {
        if piece == Piece::LP || piece == Piece::DP {
            let mut sq_str = Sq::to_string(source);
            output.push(sq_str.remove(0));
        }
        output.push('x');
    } else if mv.is_enpassant() {
        let mut sq_str = Sq::to_string(source);
        output.push(sq_str.remove(0));
        output.push('x');
    }
    // Target square
    output.push_str(&Sq::to_string(target));
    let promoted = mv.promoted();
    if promoted.is_some() {
        output.push('=');
        let ch = Piece::to_char(promoted);
        output.push(ch.to_ascii_uppercase());
    }
    // Annotations
    if checkmate {
        output.push('#');
    } else if check {
        output.push('+');
    }
    output
}

pub fn save(
    filename: &str, game: &Game, attack_info: &AttackInfo
) -> Result<bool, io::Error> {
    let f = std::fs::File::create(Path::new(filename))?;
    let mut f = BufWriter::new(f);
    writeln!(f, "[Event \"?\"]")?;
    writeln!(f, "[Site \"?\"]")?;
    writeln!(f, "[Date \"????.??.??\"]")?;
    writeln!(f, "[Round \"?\"]")?;
    writeln!(f, "[White \"{}\"]", game.white_name())?;
    writeln!(f, "[Black \"{}\"]", game.black_name())?;
    let result_str = match game.state() {
        GameState::Ongoing => "*",
        GameState::LightWinByCheckmate => "1-0",
        GameState::DarkWinByCheckmate => "0-1",
        _ => "1/2-1/2"
    };
    writeln!(f, "[Result \"{}\"]", result_str)?;
    let start_fen = game.start_fen();
    if start_fen != fen::FEN_POSITIONS[1] {
        writeln!(f, "[FEN \"{}\"]", start_fen)?;
        writeln!(f, "[SetUp \"1\"]")?;
    }
    writeln!(f)?;

    for i in 0..game.move_count() {
        if i % 2 == 0 {
            write!(f, "{}. ", (i / 2) + 1)?;
        }
        if let Some(mv) = game.move_at(i) {
            // write!(f, "{}", mv.to_str().trim())?;
            let disambiguate = should_disambiguate(*mv, attack_info, game.board_before_move(i).unwrap());
            // let ind = if i + 1 > board_info.len() - 1 { board_info.len() - 1 } else { i + 1 };
            let next_board = game.board_after_move(i).unwrap();
            let check = next_board.is_in_check(&attack_info, next_board.state.xside);
            write!(f, "{}", coord_move_to_san(*mv, attack_info, check, disambiguate, false))?;
        }
        // Every 5 moves from each side, add a newline
        if i < game.move_count() - 1 {
            if i != 0 && i % 10 == 0 {
                writeln!(f)?;
            } else {
                write!(f, " ")?;
            }
        }
    }
    writeln!(f, " {}", result_str)?;

    Ok(true)
}
/*
pub fn save(
    filename: &str, white_name: &str, black_name: &str, fen: &str,
    game_state: &GameState, attack_info: &AttackInfo, board_info: &Vec<BoardInfo>
) -> Result<bool, io::Error> {
    let f = std::fs::File::create(Path::new(filename))?;
    let mut f = BufWriter::new(f);
    writeln!(f, "[Event \"?\"]")?;
    writeln!(f, "[Site \"?\"]")?;
    writeln!(f, "[Date \"????.??.??\"]")?;
    writeln!(f, "[Round \"?\"]")?;
    writeln!(f, "[White \"{}\"]", white_name)?;
    writeln!(f, "[Black \"{}\"]", black_name)?;
    let result_str = match game_state {
        GameState::Ongoing => "*",
        GameState::LightWinByCheckmate => "1-0",
        GameState::DarkWinByCheckmate => "0-1",
        _ => "1/2-1/2"
    };
    writeln!(f, "[Result \"{}\"]", result_str)?;
    if fen != fen::FEN_POSITIONS[1] {
        writeln!(f, "[FEN \"{}\"]", fen)?;
        writeln!(f, "[SetUp \"1\"]")?;
    }
    writeln!(f)?;

    for (i, b_info) in board_info.iter().enumerate() {
        if i % 2 == 0 && b_info.is_ongoing() {
            write!(f, "{}. ", (i / 2) + 1)?;
        }
        if let Some(mv) = b_info.mv() {
            // write!(f, "{}", mv.to_str().trim())?;
            let disambiguate = should_disambiguate(mv, attack_info, b_info.board());
            // let ind = if i + 1 > board_info.len() - 1 { board_info.len() - 1 } else { i + 1 };
            let next_board = board_info[i].board();
            let check = next_board.is_in_check(&attack_info, next_board.state.xside);
            write!(f, "{}", coord_move_to_san(mv, attack_info, check, disambiguate, b_info.is_checkmate()))?;
        }
        // Every 5 moves from each side, add a newline
        if i < board_info.len() - 1 {
            if i != 0 && i % 10 == 0 {
                writeln!(f)?;
            } else {
                write!(f, " ")?;
            }
        }
    }
    writeln!(f, " {}", result_str)?;

    Ok(true)
}
*/

#[cfg(test)]
mod tests {
    use chess::attack::AttackInfo;
    use chess::board::Board;
    use chess::zobrist::ZobristInfo;
    use chess::moves::{self, Move, MoveFlag, MoveUtil};
    use chess::consts::Piece;

    use crate::pgn;

    #[test]
    fn move_to_san() {
        let white_moves_arr = [
            // Quiet moves
            (Move::from_str("d5d6", Piece::LP, false, false, false, false),  "d6"),
            (Move::from_str("h2h4", Piece::LP, false,  true, false, false),  "h4"),
            (Move::from_str("c3a4", Piece::LN, false, false, false, false), "Na4"),
            (Move::from_str("d2e3", Piece::LB, false, false, false, false), "Be3"),
            (Move::from_str("a1d1", Piece::LR, false, false, false, false), "Rd1"),
            (Move::from_str("f3g3", Piece::LQ, false, false, false, false), "Qg3"),
            (Move::from_str("e1d1", Piece::LK, false, false, false, false), "Kd1"),
            // Capture moves
            (Move::from_str("d5e6", Piece::LP,  true, false, false, false), "dxe6"),
            (Move::from_str("e5f7", Piece::LN,  true, false, false, false), "Nxf7"),
            (Move::from_str("e2a6", Piece::LB,  true, false, false, false), "Bxa6"),
            (Move::from_str("f3g2", Piece::LQ,  true, false, false, false), "Qxg2"),
            // Promotion moves
            (Move::from_str("b7b8R", Piece::LP, false, false, false, false),  "b8=R+"),
            (Move::from_str("b7a8N", Piece::LP,  true, false, false, false), "bxa8=N"),
            // Castling moves
            (Move::from_str("e1c1", Piece::LK, false, false, false, true), "O-O-O"),
            // Enpassant moves
            (Move::from_str("d5c6", Piece::LP, false, false, true, false), "dxc6"),
        ];
        let black_moves_arr = [
            // Quiet moves
            (Move::from_str("d7d6", Piece::DP, false, false, false, false),  "d6"),
            (Move::from_str("b6c8", Piece::DN, false, false, false, false), "Nc8"),
            (Move::from_str("a6b7", Piece::DB, false, false, false, false), "Bb7"),
            (Move::from_str("h8g8", Piece::DR, false, false, false, false), "Rg8"),
            (Move::from_str("e7c5", Piece::DQ, false, false, false, false), "Qc5"),
            (Move::from_str("e8f8", Piece::DK, false, false, false, false), "Kf8"),
            // Capture moves
            (Move::from_str("b4c3", Piece::DP,  true, false, false, false), "bxc3"),
            (Move::from_str("f6e4", Piece::DN,  true, false, false, false), "Nxe4"),
            (Move::from_str("a6e2", Piece::DB,  true, false, false, false), "Bxe2"),
            // Promotion moves
            (Move::from_str("g2g1b", Piece::DP, false, false, false, false), "g1=B"),
            (Move::from_str("g2h1q", Piece::DP,  true, false, false, false), "gxh1=Q+"),
            // Castling moves
            (Move::from_str("e8g8", Piece::DK, false, false, false, true), "O-O"),
        ];

        let attack_info = AttackInfo::new();
        let zobrist_info = ZobristInfo::new();
        // Board to test white's moves
        let board = Board::from_fen("r3k2r/pP1pqpb1/bn2pnp1/2pPN3/1p2P3/2N2Q2/PPPBBPpP/R3K2R w KQkq c6 0 1", &zobrist_info);

        for (i, (mv, expected)) in white_moves_arr.iter().enumerate() {
            check_move((*mv, expected), &board, &attack_info, &zobrist_info, false);
        }

        // Board to test black's moves
        let board = Board::from_fen("r3k2r/pP1pqpb1/bn2pnp1/2pPN3/1p2P3/2N2Q2/PPPBBPpP/R3K2R b KQkq - 0 1", &zobrist_info);

        for (i, (mv, expected)) in black_moves_arr.iter().enumerate() {
            check_move((*mv, expected), &board, &attack_info, &zobrist_info, false);
        }
    }

    #[test]
    fn move_to_san_2() {
        let attack_info = AttackInfo::new();
        let zobrist_info = ZobristInfo::new();
        let board = Board::from_fen("k7/8/1K6/8/8/8/2R5/8 w - - 0 1", &zobrist_info);
        let (mv, expected) = (Move::from_str("c2c8", Piece::LR, false, false, false, false), "Rc8#");
        check_move((mv, expected), &board, &attack_info, &zobrist_info, true);
    }

    #[test]
    fn move_to_san_3() {
        let attack_info = AttackInfo::new();
        let zobrist_info = ZobristInfo::new();
        let board = Board::from_fen("8/8/8/8/3b1kbK/8/8/8 b - - 0 1", &zobrist_info);
        let (mv, expected) = (Move::from_str("d4f2", Piece::DB, false, false, false, false), "Bf2#");
        check_move((mv, expected), &board, &attack_info, &zobrist_info, true);
    }

    #[test]
    fn move_to_san_4() {
        let attack_info = AttackInfo::new();
        let zobrist_info = ZobristInfo::new();
        let board = Board::from_fen("8/8/8/8/8/4k3/7p/4K1R1 b - - 0 1", &zobrist_info);
        let (mv, expected) = (Move::from_str("h2g1q", Piece::DP, true, false, false, false), "hxg1=Q#");
        check_move((mv, expected), &board, &attack_info, &zobrist_info, true);
    }

    fn check_move(
        (mv, expected): (Move, &str), board: &Board, attack_info: &AttackInfo,
        zobrist_info: &ZobristInfo, checkmate: bool
    ) {
        let mut clone_board = board.clone();
        let legal_move = moves::make(&mut clone_board, &attack_info, &zobrist_info, mv, MoveFlag::AllMoves);
        assert_eq!(legal_move, true);

        let disambiguate = pgn::should_disambiguate(mv, &attack_info, &board);
        let check = clone_board.is_in_check(&attack_info, clone_board.state.xside);
        let generated = pgn::coord_move_to_san(mv, &attack_info, check, disambiguate, checkmate);
        assert_eq!(&generated, expected);
    }
}

