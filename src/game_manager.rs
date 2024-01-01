use chess::attack::AttackInfo;
use chess::consts::{Piece, Sq};
use chess::moves::Move;
use chess::move_gen::{self, MoveList};
use chess::zobrist::ZobristInfo;

use crate::comm::EngineComm;
use crate::game::Game;

pub struct GameManager {
    engines: [EngineComm; 2],
    // time left is stored in milliseconds
    time_left: [f32; 2],
    increment: Option<u32>,
    game_history: Vec<Game>,
    game: Game,
    playing: bool,
    white_engine: usize,
}

const FIRST: usize = 0;
const SECOND: usize = 1;

const SECONDS_PER_MOVE: f32 = 1.0;

impl GameManager {
    // Default starting time for a game is 1 min per side (expressed here in milliseconds)
    const DEFAULT_START_TIME: f32 = (1 * 60 * 1000) as f32;

    pub fn new(engine_a: EngineComm, engine_b: EngineComm, zobrist_info: &ZobristInfo) -> Self {
        let game = Game::new(engine_a.name(), engine_b.name(), zobrist_info);
        Self {
            engines: [engine_a, engine_b],
            time_left: [Self::DEFAULT_START_TIME, Self::DEFAULT_START_TIME],
            increment: None,
            game_history: vec![],
            game,
            white_engine: FIRST,
            playing: false,
        }
    }

    fn switch_sides(&mut self) {
        self.white_engine ^= 1;
    }

    pub fn update_time_left(&mut self, frame_time: f32) {
        if !self.playing { return; }
        let tl = if self.game.is_white_to_move() {
            &mut self.time_left[FIRST]
        } else {
            &mut self.time_left[SECOND]
        };
        *tl -= frame_time * 1000.0;
        if let Some(inc) = self.increment {
            *tl += (inc*1000) as f32;
        }
        if *tl <= 0.0 {
            *tl = 0.0;
        }
    }

    pub fn toggle_playing(&mut self) {
        self.playing = !self.playing;
    }

    pub fn check_state(&mut self) {
        if !self.game.is_ongoing() && self.playing { self.playing = false; }
    }

    pub fn start_new_game(&mut self, fens: &String, zobrist_info: &ZobristInfo) {
        self.switch_sides();
        let new_white = self.engines[self.white_engine].name();
        let new_black = self.engines[self.white_engine^1].name();
        let game_count = self.game_history.len();
        let new_game;
        // After switching the sides and playing the game both as white and black, a new
        // position is loaded
        if game_count % 2 == 0 {
            if let Some(fen) = fens.lines().nth(game_count) {
                new_game = Game::from_fen(new_white, new_black, fen, zobrist_info);
            } else {
                eprintln!("[WARN] Couldn't load more positions to play from");
                // Exiting from this process is only temporary and will need to be fixed in the
                // future
                std::process::exit(0);
            }
        } else {
            let fen = self.game.start_fen();
            new_game = Game::from_fen(new_white, new_black, fen, zobrist_info);
        }
        let completed_game = std::mem::replace(&mut self.game, new_game);
        self.game_history.push(completed_game);
        // Reset the amount of time left
        self.time_left[self.white_engine] = Self::DEFAULT_START_TIME;
        self.time_left[self.white_engine^1] = Self::DEFAULT_START_TIME;
    }

    pub fn current_move_count(&self) -> usize {
        self.game.move_count()
    }

    pub fn time_left(&self) -> (f32, f32) {
        (self.time_left[self.white_engine], self.time_left[self.white_engine^1])
    }

    pub fn playing(&self) -> bool {
        self.playing
    }

    fn side(&self) -> usize {
        if self.game.is_white_to_move() { self.white_engine } else { self.white_engine ^ 1 }
    }

    pub fn current_game(&self) -> &Game {
        &self.game
    }

    pub fn play(&mut self, frame_time: f32, attack_info: &AttackInfo, zobrist_info: &ZobristInfo) -> Option<Move> {
        if !self.playing { return None; }
        if let Some(ref mv_str) = self.comm_with_engine(frame_time) {
            let mut found_move = None;
            if let Some(board) = self.game.board_after_last_move() {
                let source = mv_str.get(0..2);
                let target = mv_str.get(2..4);
                let promoted = if let Some(ch) = mv_str.chars().nth(4) {
                    let piece_char = if board.is_white_to_move() {
                        ch.to_ascii_uppercase()
                    } else { ch };
                    Piece::from_char(piece_char)
                } else { None };

                let piece = if let Some(sq_str) = source {
                    board.find_piece(Sq::from_str(sq_str) as usize)
                } else { None };

                if let Some(p) = piece {
                    let mut ml = MoveList::new();
                    move_gen::generate_by_piece(board, &attack_info, &mut ml, p);
                    if source.is_some() && target.is_some() {
                        found_move = ml.search(
                            Sq::from_str(source.unwrap()),
                            Sq::from_str(target.unwrap()),
                            promoted
                        );
                    }
                }
            }
            if let Some(mv) = found_move {
                if self.game.make_move(mv, &attack_info, &zobrist_info) {
                    return Some(mv);
                }
            }
        }
        None
    }

    fn comm_with_engine(&mut self, frame_time: f32) -> Option<String> {
        if !self.game.is_ongoing() || !self.playing { return None; }
        let engine: &mut EngineComm = &mut self.engines[self.side()];
        if !engine.is_searching() {
            engine.fen(&self.game.current_fen());
            engine.search_movetime((SECONDS_PER_MOVE * 1000.0) as u64);
            None
        } else if !engine.search_time_over() {
            engine.update_time_left(frame_time);
            None
        } else {
            self.get_move_from_engine(frame_time)
        }
    }


    fn get_move_from_engine(&mut self, frame_time: f32) -> Option<String> {
        let mut retry_count = 0;
        let side = self.side();
        let engine: &mut EngineComm = &mut self.engines[side];
        while retry_count < 2 {
            if self.time_left[side] <= 0.0 {
                engine.stop();
                self.game.lost_on_time(self.side() == self.white_engine);
                return None;
            }
            if let Some(best_move) = engine.best_move() {
                assert!(best_move.len() == 4 || best_move.len() == 5, "Length: {}", best_move.len());
                if best_move == "a8a8P" {
                    retry_count += 1;
                    eprintln!("Retry because of 'a8a8P'");
                    continue;
                }
                // println!("[{}] '{}'", best_move.len(), &best_move);
                return Some(best_move);
            } else {
                eprintln!("Retry because NO MOVE was sent by engine.");
                retry_count += 1;
            }
        }
        eprintln!("[ERROR] Engine, '{}' couldn't give a legal move", engine.name());
        return None;
    }

}
