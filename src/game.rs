use chess::attack::AttackInfo;
use chess::bb::BBUtil;
use chess::board::Board;
use chess::consts::PieceColor;
use chess::fen;
use chess::moves::{self, Move, MoveFlag, MoveUtil};
use chess::move_gen::{self, MoveList};
use chess::zobrist::ZobristInfo;
use chess::{COL, ROW};

use crate::pgn;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GameState {
    Ongoing,
    LightWinByCheckmate,
    DarkWinByCheckmate,
    LightLostOnTime,
    DarkLostOnTime,
    LightIllegalMove,
    DarkIllegalMove,
    DrawByStalemate,
    DrawByFiftyMoveRule,
    DrawByThreefoldRepetition,
    DrawByInsufficientMaterial,
}
pub struct Game {
    start_fen: String,
    state: GameState,
    boards: Vec<Board>,
    moves: Vec<Move>,
    white_name: String,
    black_name: String
}

impl Game {
    pub fn new(white_name: &str, black_name: &str, zobrist_info: &ZobristInfo) -> Self {
        Self::from_fen(white_name, black_name, fen::FEN_POSITIONS[1], zobrist_info)
    }

    pub fn from_fen(white_name: &str, black_name: &str, fen: &str, zobrist_info: &ZobristInfo) -> Self {
        let board = Board::from_fen(fen, zobrist_info);
        Self {
            start_fen: fen.to_string(),
            state: GameState::Ongoing,
            boards: vec![board],
            moves: vec![],
            white_name: white_name.to_string(),
            black_name: black_name.to_string()
        }
    }

    pub fn set_start_pos(&mut self, fen: &str, zobrist_info: &ZobristInfo) {
        self.start_fen = fen.to_string();
        self.boards.clear();
        let board = Board::from_fen(fen, zobrist_info);
        self.boards.push(board);
    }

    pub fn is_ongoing(&self) -> bool {
        self.state == GameState::Ongoing
    }

    pub fn is_white_to_move(&self) -> bool {
        let b = self.boards.last().unwrap();
        b.is_white_to_move()
    }

    pub fn white_name(&self) -> &String {
        &self.white_name
    }

    pub fn lost_on_time(&mut self, is_white: bool) {
        if is_white {
            self.state = GameState::LightLostOnTime;
        } else {
            self.state = GameState::DarkLostOnTime;
        }
    }

    pub fn black_name(&self) -> &String {
        &self.black_name
    }

    pub fn state(&self) -> GameState {
        self.state
    }

    pub fn start_fen(&self) -> &String {
        &self.start_fen
    }

    pub fn move_count(&self) -> usize {
        assert!(self.moves.len() == self.boards.len() - 1);
        self.moves.len()
    }

    pub fn current_fen(&self) -> String {
        if let Some(recent) = self.boards.last() {
            fen::gen_fen(recent)
        } else {
            self.start_fen.clone()
        }
    }

    pub fn move_at(&self, ind: usize) -> Option<&Move> {
        self.moves.get(ind)
    }

    pub fn last_move(&self) -> Option<&Move> {
        self.moves.last()
    }

    pub fn first_move(&self) -> Option<&Move> {
        self.moves.first()
    }

    pub fn board_before_move(&self, move_ind: usize) -> Option<&Board> {
        self.boards.get(move_ind)
    }

    pub fn board_after_move(&self, move_ind: usize) -> Option<&Board> {
        self.boards.get(move_ind + 1)
    }

    pub fn board_before_last_move(&self) -> Option<&Board> {
        let ind = self.moves.len() as i32 - 1;
        if ind >= 0 {
            return self.boards.get(ind as usize);
        }
        None
    }

    pub fn board_after_last_move(&self) -> Option<&Board> {
        self.boards.last()
    }

    pub fn save(&self, filename: Option<String>, attack_info: &AttackInfo) -> bool {
        let name;
        if let None = filename {
            name = format!("{}_vs_{}.pgn", self.white_name, self.black_name);
        } else { 
            name = filename.unwrap();
        };
        let is_saved = pgn::save(&name, &self, &attack_info).is_err();
        if !is_saved {
            eprintln!("[ERROR] Couldn't save game to file '{}'", name);
        }
        is_saved
    }

    // The returned boolean value tells whether or not the inputted move has been made successfully
    pub fn make_move(&mut self, mv: Move, attack_info: &AttackInfo, zobrist_info: &ZobristInfo) -> bool {
        let current = if let Some(b) = self.boards.last() { b } else {
            eprintln!("[ERROR] Couldn't get last board to make move on");
            return false;
        };

        let mut next_board = current.clone();
        let is_legal;

        if moves::make(&mut next_board, attack_info, zobrist_info, mv, MoveFlag::AllMoves) {
            is_legal = true;
            self.moves.push(mv);
            self.state = Self::set_state(attack_info, zobrist_info, &next_board, &self.boards);
            self.boards.push(next_board);
        } else {
            is_legal = false;
            eprintln!("[WARN] Illegal move! {}", mv.to_str().trim());
        }
        is_legal
    }

    fn set_state(attack_info: &AttackInfo, zobrist_info: &ZobristInfo, current: &Board, boards: &[Board]) -> GameState {
        // Check for draw by fifty move rule
        //   - units[0] -> all the white pieces
        //   - units[1] -> all the black pieces
        //   - Since kings can't be captured, if both sides only have one piece
        //     then that means that only kings are left on the board
        if current.state.half_moves >= 100 {
            return GameState::DrawByFiftyMoveRule;
        }
        // Check for draw by insufficient material
        if insufficient_material(current) {
            return GameState::DrawByInsufficientMaterial;
        }

        // Check for draw by checkmate or stalemate
        let board = &mut current.clone();
        let mut ml = MoveList::new();
        move_gen::generate_all(board, attack_info, &mut ml);
        // Remove illegal moves from the move list
        for i in (0..ml.moves.len()).rev() {
            let clone = board.clone();
            if !moves::make(board, attack_info, zobrist_info, ml.moves[i], MoveFlag::AllMoves) {
                ml.moves.remove(i);
            }
            *board = clone;
        }
        if ml.moves.len() == 0 {
            if board.is_in_check(attack_info, board.state.xside) {
                if board.state.xside == PieceColor::Light {
                    return GameState::LightWinByCheckmate;
                } else {
                    return GameState::DarkWinByCheckmate;
                }
            } else {
                return GameState::DrawByStalemate;
            }
        }

        // Check for draw by three fold repetition
        let mut repetition_count = 0;
        let (curr_key, curr_lock) = (board.state.key, board.state.lock);
        for b in boards {
            if curr_key == b.state.key && curr_lock == b.state.lock {
                repetition_count += 1;
                if repetition_count == 3 {
                    return GameState::DrawByThreefoldRepetition;
                }
            }
        }

        GameState::Ongoing
    }
}

fn insufficient_material(b: &Board) -> bool {
    if b.pos.units[0].count_ones() == 1 && b.pos.units[1].count_ones() == 1 {
        // K vs k
        return true;
    }
    if (b.pos.units[0].count_ones() == 2 && b.pos.piece[1].count_ones() == 1 && b.pos.units[1].count_ones() == 1)
        || (b.pos.units[1].count_ones() == 2 && b.pos.piece[7].count_ones() == 1 && b.pos.units[0].count_ones() == 1)
    {
        // (KN vs k) and (K vs kn)
        return true;
    }
    if (b.pos.units[0].count_ones() == 2 && b.pos.piece[2].count_ones() == 1 && b.pos.units[1].count_ones() == 1)
        || (b.pos.units[1].count_ones() == 2 && b.pos.piece[8].count_ones() == 1 && b.pos.units[0].count_ones() == 1)
    {
        // (KB vs k) and (K vs kb)
        return true;
    }
    if b.pos.units[0].count_ones() == 2 && b.pos.piece[1].count_ones() == 1
        && b.pos.units[1].count_ones() == 2 && b.pos.piece[7].count_ones() == 1 {
        // KN vs kn
        return true;
    }
    if b.pos.units[0].count_ones() == 2 && b.pos.piece[2].count_ones() == 1
        && b.pos.units[1].count_ones() == 2 && b.pos.piece[8].count_ones() == 1 {
        // KB vs kb
        let white_bishop = b.pos.piece[2].lsb();
        let (wr, wf) = (ROW!(white_bishop), COL!(white_bishop));
        let black_bishop = b.pos.piece[8].lsb();
        let (br, bf) = (ROW!(black_bishop), COL!(black_bishop));
        // If both bishops are the same color and there are only 1 bishops per side,
        // it's a draw due to insufficient material
        return (wr + wf) % 2 == (br + bf) % 2;
    }
    false
}
