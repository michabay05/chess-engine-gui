use crate::attack::AttackInfo;
use crate::bb::{BBUtil, BB};
use crate::consts::{Piece, PieceColor, Sq};
use crate::fen;
use crate::SQ;

#[derive(Clone)]
pub struct Position {
    pub piece: [BB; 12],
    pub units: [BB; 3],
}

impl Position {
    pub fn new() -> Self {
        Position {
            piece: [0; 12],
            units: [0; 3],
        }
    }

    pub fn update_units(&mut self) {
        self.units.fill(0);
        for i in 0..12 {
            self.units[i / 6] |= self.piece[i];
        }
        self.units[PieceColor::Both as usize] =
            self.units[PieceColor::Light as usize] | self.units[PieceColor::Dark as usize];
    }
}

#[derive(Clone)]
pub struct State {
    pub side: PieceColor,
    pub xside: PieceColor,
    pub enpassant: Sq,
    pub castling: u8,
    pub half_moves: u32,
    pub full_moves: u32,
    // ========= Zobrist keys
    // The 'key' is the primary zobrist hashing key, while the 'lock'
    // serves as the secondary hashing key. This is done to prevent the
    // chance of collision(two positions with the same key). Even though,
    // the chances of two positions having the same key is small, they may
    // happen. Therefore, this chance can be significantly reduced by adding
    // a second key to every position
    pub key: u64,
    pub lock: u64,
}

pub enum CastlingType {
    WhiteKingside,
    WhiteQueenside,
    BlackKingside,
    BlackQueenside,
}

impl State {
    pub fn new() -> Self {
        State {
            side: PieceColor::Light,
            xside: PieceColor::Dark,
            enpassant: Sq::NoSq,
            castling: 0,
            half_moves: 0,
            full_moves: 0,
            key: 0,
            lock: 0,
        }
    }

    pub fn change_side(&mut self) {
        if self.side == PieceColor::Light {
            self.side = PieceColor::Dark;
            self.xside = PieceColor::Light;
        } else {
            self.side = PieceColor::Light;
            self.xside = PieceColor::Dark;
        }
    }

    pub fn toggle_castling(&mut self, castling_type: usize) {
        let mut castling = self.castling as BB;
        if castling.get(castling_type) {
            castling.pop(castling_type);
        } else {
            castling.set(castling_type);
        }
        self.castling = castling as u8;
    }
}

#[derive(Clone)]
pub struct Board {
    pub pos: Position,
    pub state: State,
}

impl Board {
    pub fn new() -> Self {
        Board {
            pos: Position::new(),
            state: State::new(),
        }
    }

    pub fn from_fen(fen: &str) -> Self {
        fen::parse(fen)
    }

    pub fn find_piece(&self, sq: usize) -> Option<Piece> {
        for i in 0..12 {
            if self.pos.piece[i].get(sq) {
                return Piece::from_num(i);
            }
        }
        None
    }

    pub fn display(&self) {
        println!("\n    +---+---+---+---+---+---+---+---+");
        for r in 0..8 {
            print!("  {} |", 8 - r);
            for f in 0..8 {
                let piece = self.find_piece(SQ!(r, f));
                let piece_char = Piece::to_char(piece);
                print!(" {} |", piece_char);
            }
            println!("\n    +---+---+---+---+---+---+---+---+");
        }
        println!("      a   b   c   d   e   f   g   h\n");
        println!(
            "\n      Side to move: {}",
            if self.state.side == PieceColor::Light {
                "white"
            } else {
                "black"
            }
        );
        self.print_castling();
        println!(
            "         Enpassant: {}",
            Sq::to_string(self.state.enpassant)
        );
        println!("        Full Moves: {}\n", self.state.full_moves);
    }

    pub fn print_castling(&self) {
        print!("          Castling: ");
        if self.state.castling == 0 {
            println!("none");
            return;
        }
        let mut castling_ltrs = ['-', '-', '-', '-'];
        let castling = self.state.castling as BB;
        if castling.get(CastlingType::WhiteKingside as usize) {
            castling_ltrs[CastlingType::WhiteKingside as usize] = 'K';
        }
        if castling.get(CastlingType::WhiteQueenside as usize) {
            castling_ltrs[CastlingType::WhiteQueenside as usize] = 'Q';
        }
        if castling.get(CastlingType::BlackKingside as usize) {
            castling_ltrs[CastlingType::BlackKingside as usize] = 'k';
        }
        if castling.get(CastlingType::BlackQueenside as usize) {
            castling_ltrs[CastlingType::BlackQueenside as usize] = 'q';
        }
        println!("{}", castling_ltrs.iter().collect::<String>());
    }

    pub fn is_in_check(&self, attack_info: &AttackInfo, side: PieceColor) -> bool {
        let king_type = if side == PieceColor::Light {
            Piece::DK
        } else {
            Piece::LK
        } as usize;
        sq_attacked(
            &self.pos,
            attack_info,
            Sq::from_num(self.pos.piece[king_type].lsb()),
            side,
        )
    }
}

pub fn sq_attacked(pos: &Position, attack_info: &AttackInfo, sq: Sq, side: PieceColor) -> bool {
    assert!(side != PieceColor::Both);
    if side == PieceColor::Light
        && ((attack_info.pawn[PieceColor::Dark as usize][sq as usize]
            & pos.piece[Piece::LP as usize])
            != 0)
    {
        return true;
    }
    if side == PieceColor::Dark
        && ((attack_info.pawn[PieceColor::Light as usize][sq as usize]
            & pos.piece[Piece::DP as usize])
            != 0)
    {
        return true;
    }
    if (attack_info.knight[sq as usize] & pos.piece[(side as usize) * 6 + 1]) != 0 {
        return true;
    }
    if (attack_info.get_bishop_attack(sq, pos.units[PieceColor::Both as usize])
        & pos.piece[(side as usize) * 6 + 2])
        != 0
    {
        return true;
    }
    if (attack_info.get_rook_attack(sq, pos.units[PieceColor::Both as usize])
        & pos.piece[(side as usize) * 6 + 3])
        != 0
    {
        return true;
    }
    if (attack_info.get_queen_attack(sq, pos.units[PieceColor::Both as usize])
        & pos.piece[(side as usize) * 6 + 4])
        != 0
    {
        return true;
    }
    if (attack_info.king[sq as usize] & pos.piece[(side as usize) * 6 + 5]) != 0 {
        return true;
    }
    return false;
}