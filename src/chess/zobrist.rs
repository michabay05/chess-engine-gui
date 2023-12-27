use super::bb::BBUtil;
use super::board::Board;
use super::consts::{Piece, PieceColor, Sq};

#[derive(Clone)]
pub struct ZobristKey {
    pub piece: [[u64; 64]; 12],
    // Unnecessary work is being done for the enpassant keys
    // Only 16 squares (3rd and 6th rank) are enpassant squares; 48 out of the 64 are unnecessary
    pub enpassant: [u64; 64],
    pub castling: [u64; 16],
    pub side: u64,
}

impl ZobristKey {
    fn new() -> Self {
        Self {
            piece: [[0; 64]; 12],
            enpassant: [0; 64],
            castling: [0; 16],
            side: 0,
        }
    }
}

#[derive(Clone)]
pub struct ZobristLock {
    pub piece: [[u64; 64]; 12],
    // Unnecessary work is being done for the enpassant keys
    // Same reasoning as ZobristKey enpassant
    pub enpassant: [u64; 64],
    pub castling: [u64; 16],
    pub side: u64,
}

impl ZobristLock {
    fn new() -> Self {
        Self {
            piece: [[0; 64]; 12],
            enpassant: [0; 64],
            castling: [0; 16],
            side: 0,
        }
    }
}

#[derive(Clone)]
pub struct ZobristInfo {
    pub key: ZobristKey,
    pub lock: ZobristLock,
}

impl ZobristInfo {
    pub fn new() -> Self {
        let mut this = Self {
            key: ZobristKey::new(),
            lock: ZobristLock::new(),
        };
        this.init();
        this
    }

    fn init(&mut self) {
        for piece in 0..=11 {
            for sq in 0..64 {
                self.key.piece[piece][sq] = rand::random();
                self.lock.piece[piece][sq] = rand::random();
            }
        }

        for sq in 0..64 {
            self.key.enpassant[sq] = rand::random();
            self.lock.enpassant[sq] = rand::random();
        }

        // All different variations of castling rights - (1 << 4)
        for i in 0..16 {
            self.key.castling[i] = rand::random();
            self.lock.castling[i] = rand::random();
        }
        self.key.side = rand::random();
        self.lock.side = rand::random();
    }
}

pub enum ZobristAction {
    Castling,
    ChangeColor,
    SetEnpassant(Sq),
    TogglePiece(Piece, Sq),
}

pub fn update(info: &ZobristInfo, action: ZobristAction, board: &mut Board) {
    match action {
        ZobristAction::Castling => {
            board.state.key ^= info.key.castling[board.state.castling as usize];
            board.state.lock ^= info.lock.castling[board.state.castling as usize];
        }
        ZobristAction::ChangeColor => {
            board.state.key ^= info.key.side;
            board.state.lock ^= info.lock.side;
        }
        ZobristAction::SetEnpassant(sq) => {
            board.state.key ^= info.key.enpassant[sq as usize];
            board.state.lock ^= info.lock.enpassant[sq as usize];
        }
        ZobristAction::TogglePiece(piece, sq) => {
            board.state.key ^= info.key.piece[piece as usize][sq as usize];
            board.state.lock ^= info.lock.piece[piece as usize][sq as usize];
        }
    };
}

pub fn gen_board_key(key: &ZobristKey, board: &Board) -> u64 {
    let mut final_key = 0;
    let mut bb_copy;
    for piece in 0..12 {
        bb_copy = board.pos.piece[piece];
        while bb_copy != 0 {
            let sq = bb_copy.pop_lsb();
            final_key ^= key.piece[piece][sq];
        }
    }
    if board.state.enpassant != Sq::NoSq {
        final_key ^= key.enpassant[board.state.enpassant as usize];
    }
    final_key ^= key.castling[board.state.castling as usize];

    // If side to move is dark, then hash the side
    // If not, don't hash the side to move
    if board.state.side == PieceColor::Dark {
        final_key ^= key.side;
    }
    final_key
}

pub fn gen_board_lock(lock: &ZobristLock, board: &Board) -> u64 {
    let mut final_lock = 0;
    let mut bb_copy;
    for piece in 0..12 {
        bb_copy = board.pos.piece[piece];
        while bb_copy != 0 {
            let sq = bb_copy.pop_lsb();
            final_lock ^= lock.piece[piece][sq];
        }
    }
    if board.state.enpassant != Sq::NoSq {
        final_lock ^= lock.enpassant[board.state.enpassant as usize];
    }
    final_lock ^= lock.castling[board.state.castling as usize];

    // If side to move is dark, then hash the side
    // If not, don't hash the side to move
    if board.state.side == PieceColor::Dark {
        final_lock ^= lock.side;
    }
    final_lock
}
