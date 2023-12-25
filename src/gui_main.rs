use raylib::prelude::*;

use crate::attack::AttackInfo;
use crate::bb::{BB, BBUtil};
use crate::board::Board;
use crate::comm::EngineComm;
use crate::consts::{Piece, PieceColor, PieceType, Sq};
use crate::fen::{self, FEN_POSITIONS};
use crate::moves::{self, Move, MoveFlag, MoveUtil};
use crate::move_gen::{self, MoveList};
use crate::zobrist::ZobristInfo;
use crate::{COL, ROW, SQ};

use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::time::Instant;

const BACKGROUND: Color = Color::new(30, 30, 30, 255);
const PROMOTION_BACKGROUND: Color = Color::new(46, 46, 46, 220);

const LIGHT_SQ_CLR: Color = Color::new(118, 150, 86, 255);
const LIGHT_SELECTED_CLR: Color = Color::new(187, 204, 68, 255);
const DARK_SQ_CLR: Color = Color::new(238, 238, 210, 255);
const DARK_SELECTED_CLR: Color = Color::new(244, 246, 128, 255);

fn draw_board(d: &mut RaylibDrawHandle, sec: &Rectangle, b_ui: &BoardUI) {
    let mut cell_size = Vector2::one();
    cell_size.scale(sec.width / 8.0);

    for r in 0..8 {
        for f in 0..8 {
            let light_sq = (r + f) % 2 != 0;
            let mut sq_clr = if light_sq { LIGHT_SQ_CLR } else { DARK_SQ_CLR };
            if let Some(sq) = b_ui.selected {
                let sq = sq as usize;
                if sq == SQ!(r, f) {
                    sq_clr = if (ROW!(sq) + COL!(sq)) % 2 != 0 { LIGHT_SELECTED_CLR } else { DARK_SELECTED_CLR };
                }
            }
            if let Some(sq) = b_ui.target {
                let sq = sq as usize;
                if sq == SQ!(r, f) {
                    sq_clr = if (ROW!(sq) + COL!(sq)) % 2 != 0 { LIGHT_SELECTED_CLR } else { DARK_SELECTED_CLR };
                }
            }
            if let Some(sq) = b_ui.check {
                let sq = sq as usize;
                if sq == SQ!(r, f) {
                    let check_clr = Color::new(189, 55, 55, 255);
                    sq_clr = Color::color_alpha_blend(&sq_clr, &check_clr, &Color::new(255, 255, 255, 200));
                }
            }

            d.draw_rectangle_v(
                Vector2::new(
                    sec.x + (f as f32) * cell_size.x,
                    sec.y + (r as f32) * cell_size.y
                ),
                cell_size,
                sq_clr
            );
        }
    }
}

fn draw_coords(d: &mut RaylibDrawHandle, font: &Font, sec: &Rectangle) {
    // File markings
    let sq_size = sec.width / 8.0;
    for f in 0..8 {
        // row(r) = 7
        let text_color = if (7+f) % 2 != 0 { DARK_SQ_CLR } else { LIGHT_SQ_CLR };
        d.draw_text_ex(
            font,
            &format!("{}", (b'a' + f) as char),
            Vector2::new(
                sec.x + f as f32 * sq_size + (sq_size * 0.83),
                sec.y + 0.965*sec.height
            ),
            font.baseSize as f32 * 0.5,
            0.0,
            text_color
        );
    }
    // Row markings
    for r in 0..8 {
        // file(f) = 0
        let text_color = if (r+0) % 2 != 0 { DARK_SQ_CLR } else { LIGHT_SQ_CLR };
        d.draw_text_ex(
            font,
            &format!("{}", 8-r),
            Vector2::new(
                sec.x + 0.01*sec.width,
                sec.y + r as f32 * sq_size + (0.01 * sec.height),
            ),
            font.baseSize as f32 * 0.5,
            0.0,
            text_color
        );
    }
}

fn draw_piece(d: &mut RaylibDrawHandle, tex: &Texture2D, target: Rectangle, piece: Piece) {
    let (color, kind) = Piece::to_tuple(Some(piece));
    let source_rect = Rectangle::new(
        (kind as i32 * tex.width() / 6) as f32,
        (color as i32 * tex.height() / 2) as f32,
        (tex.width() / 6) as f32,
        (tex.height() / 2) as f32,
    );
    d.draw_texture_pro(
        &tex,
        source_rect,
        target,
        Vector2::zero(),
        0.0,
        Color::WHITE,
    );
}

fn piece_rect_on_board(sec: &Rectangle, sq: usize) -> Rectangle {
    let min_side = f32::min(sec.width, sec.height);
    let mut cell_size = Vector2::one();
    cell_size.scale(min_side / 8.0);

    let r = ROW!(sq);
    let f = COL!(sq);
    Rectangle::new(
        sec.x + (f as f32) * cell_size.x,
        sec.y + (r as f32) * cell_size.y,
        cell_size.x,
        cell_size.y
    )
}

fn draw_markers(d: &mut RaylibDrawHandle, board: &Board, tex: &Texture2D, sec: &Rectangle, game_state: GameState) {
    let light_king = board.pos.piece[Piece::LK as usize].lsb();
    let dark_king = board.pos.piece[Piece::DK as usize].lsb();
    let tex_ind = match game_state {
        GameState::LightWinByCheckmate => Some((0, 1)),
        GameState::DarkWinByCheckmate => Some((1, 0)),
        GameState::Ongoing => None,
        _ => Some((2, 3))
    };
    if tex_ind.is_none() { return; }
    let (l_ind, d_ind) = tex_ind.unwrap();
    // This texture has 8 icons in it so each 'frame' has a width of 1/8 of the total width
    let frame_width = tex.width() as f32 / 8.0;
    let l_source_rect = Rectangle {
        x: l_ind as f32 * frame_width,
        y: 0.0,
        width: frame_width,
        height: tex.height() as f32
    };
    let target_width = (sec.width / 8.0) * 0.4;
    let target_height = tex.height() as f32 * target_width/frame_width;
    let l_target_rect = Rectangle {
        x: sec.x + ((COL!(light_king) + 1) as f32 * sec.width / 8.0) - target_width / 2.0,
        y: sec.y + (ROW!(light_king) as f32 * sec.height / 8.0) - target_height / 2.0,
        width: target_width,
        height: target_height
    };
    d.draw_texture_pro(
        &tex,
        l_source_rect,
        l_target_rect,
        Vector2::zero(),
        0.0,
        Color::WHITE,
    );

    let d_source_rect = Rectangle {
        x: d_ind as f32 * frame_width,
        y: 0.0,
        width: frame_width,
        height: tex.height() as f32
    };
    let target_height = tex.height() as f32 * target_width/frame_width;
    let d_target_rect = Rectangle {
        x: sec.x + ((COL!(dark_king) + 1) as f32 * sec.width / 8.0) - target_width / 2.0,
        y: sec.y + (ROW!(dark_king) as f32 * sec.height / 8.0) - target_height / 2.0,
        width: target_width,
        height: target_height
    };
    d.draw_texture_pro(
        &tex,
        d_source_rect,
        d_target_rect,
        Vector2::zero(),
        0.0,
        Color::WHITE,
    );
}

fn move_is_legal(board: &Board, attack_info: &AttackInfo, source: Sq, target: Sq, promoted: Option<Piece>) -> Option<Move> {
    let mut ml = MoveList::new();
    let piece = board.find_piece(source as usize);
    if piece.is_none() {
        println!("'{}' -> '{}'", source, target);
        board.display();
    }
    assert!(piece.is_some());
    move_gen::generate_by_piece(board, attack_info, &mut ml, piece.unwrap());
    ml.search(source, target, promoted)
}

fn handle_board_selected(
    rl: &RaylibHandle, board: &Board, board_sec: &Rectangle, selected: &mut Option<Sq>
) {
    if rl.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON) {
        let mouse_pos = rl.get_mouse_position();
        let mut temp_selected = None;
        if board_sec.check_collision_point_rec(mouse_pos) {
            let col = ((mouse_pos.x - board_sec.x) / (board_sec.width / 8.0)) as usize;
            let row = ((mouse_pos.y - board_sec.y) / (board_sec.height / 8.0)) as usize;
            temp_selected = Some(Sq::from_num(SQ!(row, col)));
        } else {
            *selected = None;
            return;
        }
        let sq = temp_selected.unwrap();
        if let Some(piece) = board.find_piece(sq as usize) {
            if selected.is_some() && piece as usize / 6 != board.state.side as usize {
                return;
            }
        } else {
            return;
        }
        if temp_selected == *selected {
            *selected = None;
            return;
        }
        *selected = temp_selected;
    }
}

fn handle_board_target(
    rl: &RaylibHandle, board: &Board, board_sec: &Rectangle, selected: &Option<Sq>,
    target: &mut Option<Sq>, is_promotion: &mut bool
) {
    if selected.is_none() { return; }
    if *is_promotion || target.is_some() { return; }
    if rl.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON) {
        let mouse_pos = rl.get_mouse_position();
        let mut temp_selected = None;
        if board_sec.check_collision_point_rec(mouse_pos) {
            let col = ((mouse_pos.x - board_sec.x) / (board_sec.width / 8.0)) as usize;
            let row = ((mouse_pos.y - board_sec.y) / (board_sec.height / 8.0)) as usize;
            temp_selected = Some(Sq::from_num(SQ!(row, col)));
        }
        if temp_selected == *selected { return; }
        *target = temp_selected;
        let piece = board.find_piece(selected.unwrap() as usize);
        if piece.is_none() { return; }
        let piece = piece.unwrap();
        let sq = temp_selected.unwrap();
        if (piece == Piece::LP || piece == Piece::DP)
            && (ROW!(sq as usize) == 0 || ROW!(sq as usize) == 7) {
            *is_promotion = true;
        }
    }
}

fn update_player(
    rl: &RaylibHandle, board: &mut Board, attack_info: &AttackInfo,
    boundary: &Rectangle, promoted_boundary: &Rectangle, selected: &mut Option<Sq>, target: &mut Option<Sq>,
    is_promotion: &mut bool, promoted_piece: &mut Option<Piece>
) {
    if *is_promotion {
        let mouse_pos = rl.get_mouse_position();
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON)
        && promoted_boundary.check_collision_point_rec(mouse_pos) {
            let mut piece = (mouse_pos.x / (promoted_boundary.width / 4.0)).trunc() as usize;
            if board.state.side == PieceColor::Dark {
                piece += 6;
            }
            *promoted_piece = match piece {
                1 => Some(Piece::LN),
                2 => Some(Piece::LB),
                3 => Some(Piece::LR),
                4 => Some(Piece::LQ),
                7 => Some(Piece::DN),
                8 => Some(Piece::DB),
                9 => Some(Piece::DR),
                10 => Some(Piece::DQ),
                _ => None
            };
            *is_promotion = false;
        }
    }
    handle_board_selected(rl, board, boundary, selected);
    handle_board_target(rl, board, boundary, &selected, target, is_promotion);
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum GameState {
    Ongoing,
    LightWinByCheckmate,
    DarkWinByCheckmate,
    DrawByStalemate,
    DrawByFiftyMoveRule,
    DrawByThreefoldRepetition,
    DrawByInsufficientMaterial,
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

fn save_game(
    filename: &str, engine_a_name: &str, engine_b_name: &str, fen: &str,
    game_state: &GameState, board_info: &Vec<BoardInfo>
) -> Result<(), io::Error> {
    let f = std::fs::File::create(Path::new(filename))?;
    let mut f = BufWriter::new(f);
    writeln!(f, "[Event \"?\"]")?;
    writeln!(f, "[Site \"?\"]")?;
    writeln!(f, "[Date \"????.??.??\"]")?;
    writeln!(f, "[Round \"?\"]")?;
    writeln!(f, "[White \"{}\"]", engine_a_name)?;
    writeln!(f, "[Black \"{}\"]", engine_b_name)?;
    let result_str = match game_state {
        GameState::Ongoing => "*",
        GameState::LightWinByCheckmate => "1-0",
        GameState::DarkWinByCheckmate => "0-1",
        _ => "1/2-1/2"
    };
    writeln!(f, "[Result \"{}\"]", result_str)?;
    writeln!(f, "[FEN \"{}\"]", fen)?;
    writeln!(f, "[SetUp \"1\"]")?;
    writeln!(f)?;

    for (i, b_info) in board_info.iter().enumerate() {
        if i % 2 == 0 && b_info.game_state == GameState::Ongoing {
            write!(f, "{}. ", (i / 2) + 1)?;
        }
        // TODO: change move from coordinate form to short algebraic notation
        if let Some(mv) = b_info.mv {
            write!(f, "{}", mv.to_str().trim())?;
        } else {
            eprintln!("[ERROR] Couldn't find move associated with the current position");
            b_info.b_ui.board.display();
        }
        // Every 5 moves from each side, add a newline
        if i == board_info.len() - 1 {
            writeln!(f, " {}", result_str)?;
        } else if i != 0 && i % 10 == 0 {
            writeln!(f)?;
        } else {
            write!(f, " ")?;
        }
    }

    Ok(())
}

#[derive(Clone)]
struct BoardUI {
    board: Board,
    selected: Option<Sq>,
    target: Option<Sq>,
    check: Option<Sq>,
}

impl BoardUI {
    fn from(board: &Board) -> Self {
        Self {
            board: board.clone(),
            selected: None,
            target: None,
            check: None,
        }
    }

    fn highlight_selected(&mut self, sq: usize) {
        // Reset everything before setting the `selected` square
        self.selected = None;
        self.target = None;
        self.check = None;

        self.selected = Some(Sq::from_num(sq));
    }

    fn highlight_target(&mut self, sq: usize) {
        self.target = Some(Sq::from_num(sq));
    }

    fn highlight_check(&mut self, is_white: bool) {
        let king_ind = if is_white { 5 } else { 11 };
        let king_sq = self.board.pos.piece[king_ind].lsb();
        self.check = Some(Sq::from_num(king_sq));
    }
}

struct BoardInfo {
    b_ui: BoardUI,
    mv: Option<Move>,
    game_state: GameState,
}

impl BoardInfo {
    fn from(b_ui: &BoardUI) -> Self {
        Self {
            b_ui: b_ui.clone(),
            mv: None,
            game_state: GameState::Ongoing
        }
    }

    fn update(&mut self, mv: Move, game_state: GameState) {
        self.mv = Some(mv);
        self.game_state = game_state;
    }
}


fn draw_players_name(d: &mut RaylibDrawHandle, font: &Font, sec: &Rectangle, light_name: &str, dark_name: &str) {
    let margin = Vector2::new(sec.width * 0.01, sec.height * 0.03);

    let white_text_dim = text::measure_text_ex(font, light_name, font.baseSize as f32, 0.0);
    let black_text_dim = text::measure_text_ex(font, dark_name, font.baseSize as f32, 0.0);
    let larger_x = f32::max(white_text_dim.x, black_text_dim.x);

    let rect_width = f32::max(sec.width * 0.3, larger_x * 1.35);
    let rect_height = f32::max(rect_width * 0.35, font.baseSize as f32 * 1.35);
    let light_bkgd_rect = Rectangle {
        x: sec.x + (sec.width - margin.x) * 0.225 - rect_width / 2.0,
        y: sec.y + margin.y + 0.1 * (sec.height - margin.y),
        width: rect_width,
        height: rect_height
    };
    let dark_bkgd_rect = Rectangle {
        x: (sec.x + sec.width) - (sec.width - margin.x) * 0.225 - rect_width / 2.0,
        y: light_bkgd_rect.y,
        width: light_bkgd_rect.width,
        height: light_bkgd_rect.height
    };
    d.draw_rectangle_rec(light_bkgd_rect, Color::WHITE);
    d.draw_rectangle_lines_ex(dark_bkgd_rect, 3, Color::WHITE);

    d.draw_text_ex(&font, light_name,
        Vector2::new(
            light_bkgd_rect.x + light_bkgd_rect.width / 2.0 - (white_text_dim.x / 2.0),
            light_bkgd_rect.y + light_bkgd_rect.height / 2.0 - (white_text_dim.y / 2.0)
        ),
        font.baseSize as f32, 0.0, BACKGROUND);
    d.draw_text_ex(&font, dark_name,
        Vector2::new(
            dark_bkgd_rect.x + dark_bkgd_rect.width / 2.0 - (black_text_dim.x / 2.0),
            dark_bkgd_rect.y + dark_bkgd_rect.height / 2.0 - (black_text_dim.y / 2.0)
        ),
        font.baseSize as f32, 0.0, Color::WHITE);
}

const SECONDS_PER_MOVE: f32 = 0.75;

struct GUI {
    selected: Option<Sq>,
    target: Option<Sq>,
    is_promotion: bool,
    promoted_piece: Option<Piece>,

    // Engine related
    play_game: bool,
    eval: i32,
    is_mate: bool,

    // Current board position
    original_fen: String,
    current_fen: String,
    current: BoardUI,
    state: GameState,
    // History of moves and board
    history: Vec<BoardInfo>,
    current_index: usize,

    // Animation
    anim_start_time: Instant,
    anim_mv: Option<Move>,
    is_animating: bool,
    anim_target_board: Option<Board>,
    anim_duration_secs: f32,

    // Sections on the screen
    board_sec: Rectangle,
    promotion_sec: Rectangle,
    info_sec: Rectangle
}

impl GUI {
    fn new(zobrist_info: &ZobristInfo) -> Self {
        Self::from_fen(FEN_POSITIONS[1], zobrist_info)
    }

    fn from_fen(fen: &str, zobrist_info: &ZobristInfo) -> Self {
        let board = Board::from_fen(fen, zobrist_info);
        let current = BoardUI::from(&board);
        let history = vec![BoardInfo::from(&current)];
        Self {
            selected: None,
            target: None,
            is_promotion: false,
            promoted_piece: None,

            play_game: false,
            eval: 0,
            is_mate: false,

            original_fen: String::from(fen),
            current_fen: String::from(fen),
            current,
            state: GameState::Ongoing,

            history,
            current_index: 0,

            anim_start_time: Instant::now(),
            anim_mv: None,
            is_animating: false,
            anim_target_board: None,
            anim_duration_secs: f32::min(0.15, SECONDS_PER_MOVE - 0.05),

            board_sec: Rectangle::default(),
            promotion_sec: Rectangle::default(),
            info_sec: Rectangle::default(),
        }
    }

    fn update_sections(&mut self, size: Vector2) {
        let margin = Vector2::new(size.x * 0.01, size.y * 0.03);
        let min_side = f32::min((size.x - 2.0*margin.x) * 0.7, size.y - 2.0*margin.y);
        self.board_sec = Rectangle {
            x: margin.x,
            y: margin.y,
            width: min_side,
            height: min_side
        };
        let promoted_height = self.board_sec.height * 0.15;
        let promoted_width = 4.0 * promoted_height;
        self.promotion_sec = Rectangle {
            x: self.board_sec.x + (self.board_sec.width / 2.0) - (promoted_width / 2.0),
            y: self.board_sec.y + (self.board_sec.height / 2.0) - (promoted_height / 2.0),
            width: promoted_width,
            height: promoted_height,
        };

        self.info_sec = Rectangle {
            x: self.board_sec.x + self.board_sec.width + margin.x,
            y: self.board_sec.y,
            width: size.x - (self.board_sec.x + self.board_sec.width + 2.0*margin.x),
            height: min_side,
        };
    }

    fn update_state(&mut self, attack_info: &AttackInfo, zobrist_info: &ZobristInfo) {
        // Check for draw by fifty move rule
        //   - units[0] -> all the white pieces
        //   - units[1] -> all the black pieces
        //   - Since kings can't be captured, if both sides only have one piece
        //     then that means that only kings are left on the board
        if self.current.board.state.half_moves >= 100 {
            self.state = GameState::DrawByFiftyMoveRule;
            return;
        }
        // Check for draw by insufficient material
        if insufficient_material(&self.current.board) {
            self.state = GameState::DrawByInsufficientMaterial;
            return;
        }

        // Check for draw by checkmate or stalemate
        let board = &mut self.current.board;
        let mut ml = MoveList::new();
        move_gen::generate_all(&board, attack_info, &mut ml);
        // Remove illegal moves from the move list
        for i in (0..ml.moves.len()).rev() {
            let clone = board.clone();
            if !moves::make(board, attack_info, zobrist_info, ml.moves[i], MoveFlag::AllMoves) {
                ml.moves.remove(i);
            }
            *board = clone;
        }
        if ml.moves.len() == 0 {
            if self.current.board.is_in_check(attack_info, self.current.board.state.xside) {
                if self.current.board.state.xside == PieceColor::Light {
                    self.state = GameState::LightWinByCheckmate;
                    return;
                } else {
                    self.state = GameState::DarkWinByCheckmate;
                    return;
                }
            } else {
                self.state = GameState::DrawByStalemate;
                return;
            }
        }

        // Check for draw by three fold repetition
        let mut repetition_count = 0;
        let (curr_key, curr_lock) = (self.current.board.state.key, self.current.board.state.lock);
        for pos in &self.history {
            let b = &pos.b_ui.board;
            if curr_key == b.state.key && curr_lock == b.state.lock {
                repetition_count += 1;
                if repetition_count == 3 {
                    self.state = GameState::DrawByThreefoldRepetition;
                    return;
                }
            }
        }

        self.state = GameState::Ongoing;
    }

    fn make_move(&mut self, attack_info: &AttackInfo, zobrist_info: &ZobristInfo) -> bool {
        let mut is_legal = false;
        let curr_move = move_is_legal(
            &self.current.board, attack_info, self.selected.unwrap(),
            self.target.unwrap(), self.promoted_piece
        );
        self.anim_target_board = Some(self.current.board.clone());
        if let Some(mv) = curr_move {
            if moves::make(self.anim_target_board.as_mut().unwrap(), attack_info, zobrist_info, mv, MoveFlag::AllMoves) {
                self.current_fen = fen::gen_fen(self.anim_target_board.as_ref().unwrap());
                if let Some(b_info) = self.history.last_mut() {
                    b_info.update(mv, self.state);
                }
                if self.state == GameState::Ongoing {
                    self.history.push(BoardInfo::from(&self.current));
                } else {
                    println!("Game ended by {:?}", self.state);
                }
                self.current_index += 1;

                let b = self.anim_target_board.as_ref().unwrap();
                if b.is_in_check(attack_info, b.state.xside) {
                    self.current.highlight_check(b.state.side as usize == 0);
                }

                self.anim_mv = Some(mv);
                self.anim_start_time = Instant::now();
                self.is_animating = true;
                is_legal = true;
            } else {
                eprintln!("[ERROR] Illegal move! {}", mv.to_str());
                is_legal = false;
            }
        }
        self.selected = None;
        self.target = None;
        self.promoted_piece = None;
        self.is_promotion = false;
        is_legal
    }

    fn update_engine(&mut self, frame_time: f32, engine: &mut EngineComm) {
        if !engine.is_searching() {
            engine.fen(&self.current_fen);
            engine.search_movetime((SECONDS_PER_MOVE * 1000.0) as u64);
        } else {
            if !engine.search_time_over() {
                engine.update_time_left(frame_time);
            } else {
                if let Some(best_move) = engine.best_move(&mut self.eval, &mut self.is_mate) {
                    assert!(best_move.len() == 4 || best_move.len() == 5, "Length: {}", best_move.len());
                    println!("[{}] '{}'", best_move.len(), &best_move);
                    // Move from engine example: "b7b8q"
                    self.selected = Some(Sq::from_str(&best_move[0..2])); // "b7"
                    self.target = Some(Sq::from_str(&best_move[2..4]));   // "b8"
                    self.promoted_piece = if best_move.len() == 5 {       // 'q'
                        if let Some(piece_char) = best_move.chars().nth(4) {
                            Piece::from_char(piece_char)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                }
            }
        }
    }
}

pub fn gui_main(engine_a_path: String, engine_b_path: Option<String>) -> Result<(), String> {
    let attack_info = AttackInfo::new();
    let zobrist_info = ZobristInfo::new();

    // Load in a list of fens
    let fens = if let Ok(content) = std::fs::read_to_string("fens.txt") {
        content
    } else {
        eprintln!("[ERROR] Couldn't load fens from 'fens.txt'");
        // Exiting due to the failure of reading fens from a file is temporary.
        // This is only needed for testing
        std::process::exit(0);
    };

    let engine_a = EngineComm::new(&engine_a_path);
    let engine_b = if let Some(b_path) = engine_b_path {
        EngineComm::new(&b_path)
    } else {
        EngineComm::new(&engine_a_path)
    };

    if engine_a.is_err() || engine_b.is_err() {
        return Err("Failed to establish communication with specified engine(s) ".to_string());
    }
    let mut engine_a = engine_a.unwrap();
    let mut engine_b = engine_b.unwrap();

    println!("[INFO] Time per move: {} s", SECONDS_PER_MOVE);
    println!("[INFO] {} vs {}", engine_a.name(), engine_b.name());

    let (mut rl, thread) = raylib::init()
        .size(1000, 600)
        .title("Chess Engine GUI")
        // .resizable()
        .build();

    rl.set_window_min_size(1000, 600);
    rl.set_target_fps(60);

    // let piece_tex = rl.load_texture(&thread, "assets/chesscom-pieces/chesscom_pieces.png")?;
    let piece_tex = rl.load_texture(&thread, "assets/lichess-pieces/cburnett_pieces.png")?;
    piece_tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_TRILINEAR);
    let game_end_tex = rl.load_texture(&thread, "assets/chesscom-pieces/game-end-icons.png")?;
    game_end_tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);

    let font = rl.load_font(&thread, "assets/fonts/Inter-Regular.ttf")?;
    let bold_font = rl.load_font(&thread, "assets/fonts/Inter-Bold.ttf")?;

    let mut gui = GUI::new(&zobrist_info);

    let mut is_engine_a = true;

    while !rl.window_should_close() {
        /* ==================== UPDATE PHASE ==================== */
        let size = Vector2::new(rl.get_screen_width() as f32, rl.get_screen_height() as f32);
        gui.update_sections(size);

        if rl.is_key_pressed(KeyboardKey::KEY_SPACE) {
            if gui.current_index == gui.history.len() - 1 {
                gui.play_game = !gui.play_game;
            } else {
                eprintln!(
                    "[WARN] The board must be at the current position. You are {} moves behind the current.",
                    (gui.history.len() - 1) - gui.current_index
                );
            }
        } else if rl.is_key_pressed(KeyboardKey::KEY_LEFT) {
            if !gui.play_game {
                gui.current_index = gui.current_index.saturating_sub(1);
                gui.current = gui.history[gui.current_index].b_ui.clone();
            }
        } else if rl.is_key_pressed(KeyboardKey::KEY_RIGHT) {
            if !gui.play_game {
                gui.current_index += 1;
                if gui.current_index >= gui.history.len() { gui.current_index = gui.history.len() - 1; };
                gui.current = gui.history[gui.current_index].b_ui.clone();
            }
        } else if rl.is_key_pressed(KeyboardKey::KEY_UP) {
            if !gui.play_game {
                gui.current_index = 0;
                gui.current = gui.history[gui.current_index].b_ui.clone();
            }
        } else if rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
            if !gui.play_game {
                gui.current_index = gui.history.len() - 1;
                gui.current = gui.history[gui.current_index].b_ui.clone();
            }
        } else if rl.is_key_pressed(KeyboardKey::KEY_F) {
            let current_fen = fen::gen_fen(&gui.current.board);
            if rl.set_clipboard_text(&current_fen).is_err() {
                eprintln!("[ERROR] Failed to copy clipboard to fen");
            }
        } else if rl.is_key_pressed(KeyboardKey::KEY_N) {
            if !gui.play_game {
                gui = GUI::new(&zobrist_info);
                is_engine_a = true;
                println!("fen: '{}'", gui.current_fen);
            }
        } else if rl.is_key_pressed(KeyboardKey::KEY_R) {
            // Reset board and load a random fen to current position
            if !gui.play_game {
                let rand_ind = rand::random::<usize>() % 500;
                let random_fen = fens.lines().nth(rand_ind).unwrap();
                gui = GUI::from_fen(random_fen, &zobrist_info);
                is_engine_a = true;
                println!("fen: '{}'", gui.current_fen);
            }
        } else if rl.is_key_pressed(KeyboardKey::KEY_S) {
            if !gui.play_game {
                if save_game("game.pgn", engine_a.name(), engine_b.name(), &gui.original_fen, &gui.state, &gui.history).is_err() {
                    return Err("Couldn't save game to file 'game.pgn'".to_string());
                }
            }
        }

        // game_state = update_game_state(&mut b_ui.board, &attack_info, &zobrist_info, &gui.history);
        if gui.state == GameState::Ongoing {
            gui.update_state(&attack_info, &zobrist_info);
        }

        if gui.play_game && gui.state != GameState::Ongoing {
            gui.play_game = false;
        }

        if gui.play_game {
            let engine = if is_engine_a { &mut engine_a } else { &mut engine_b };

            gui.update_engine(rl.get_frame_time(), engine);

            if let Some(sq) = gui.selected {
                gui.current.highlight_selected(sq as usize);
            }
            if let Some(sq) = gui.target {
                gui.current.highlight_target(sq as usize);
            }
            // Correct the promotion piece from the UCI string
            if let Some(piece) = gui.promoted_piece {
                let mut piece_num = piece as usize % 6;
                if gui.current.board.state.side == PieceColor::Dark { piece_num += 6 };
                gui.promoted_piece = Piece::from_num(piece_num);
            }

            if !engine.is_searching() {
                is_engine_a = !is_engine_a;
            }
        }
        // update_player(&rl, &mut board, &attack_info, &boundary, &promoted_boundary, &mut selected, &mut target,
        //     &mut is_promotion, &mut promoted_piece);
        if gui.selected.is_some() && gui.target.is_some() && !gui.is_promotion {
            let _ = gui.make_move(&attack_info, &zobrist_info);
        }


        /* ==================== RENDER PHASE ==================== */
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(BACKGROUND);
        draw_board(&mut d, &gui.board_sec, &gui.current);
        draw_coords(&mut d, &bold_font, &gui.board_sec);
        let skip_sq = if gui.current_index == gui.history.len() - 1 {
            gui.current.selected
        } else { None };
        // let skip_sq = gui.history[gui.current_index].b_ui.selected;
        draw_pieces(&mut d, skip_sq, &piece_tex, &gui.current, &gui.board_sec);

        fn draw_pieces(d: &mut RaylibDrawHandle, skip_sq: Option<Sq>, tex: &Texture2D, b_ui: &BoardUI, sec: &Rectangle) {
            for r in 0..8 {
                for f in 0..8 {
                    let sq = SQ!(r, f);
                    if let Some(s_sq) = skip_sq {
                        if s_sq as usize == sq {
                            continue;
                        }
                    }
                    if let Some(piece) = b_ui.board.find_piece(sq) {
                        draw_piece(d, tex, piece_rect_on_board(sec, sq), piece);
                    }
                }
            }
        }

        fn anim_piece(d: &mut RaylibDrawHandle, boundary: &Rectangle, tex: &Texture2D, mv: Move, t: f32) {
            let source_rect = piece_rect_on_board(boundary, mv.source() as usize);
            let target_rect = piece_rect_on_board(boundary, mv.target() as usize);
            let piece = mv.piece();
            let source_vec = Vector2::new(source_rect.x, source_rect.y);
            let target_vec = Vector2::new(target_rect.x, target_rect.y);
            let anim_pos = source_vec.lerp(target_vec, t as f32);
            let anim_rect = Rectangle::new(anim_pos.x, anim_pos.y, source_rect.width, source_rect.height);
            draw_piece(d, tex, anim_rect, piece);
        }

        if let Some(mv) = gui.anim_mv {
            // anim_t = (NOW - anim_start_time) / ANIM_DURATION_SECS;
            let elapsed = Instant::now().duration_since(gui.anim_start_time);
            let anim_t = elapsed.div_f32(gui.anim_duration_secs).as_secs_f32();
            if gui.is_animating && anim_t >= 1.0 {
                gui.is_animating = false;
                gui.anim_mv = None;
                gui.current.board = gui.anim_target_board.unwrap();
                gui.anim_target_board = None;
                // Instantly make the move by drawing the target board
                draw_pieces(&mut d, None, &piece_tex, &gui.current, &gui.board_sec);
            }

            if gui.is_animating {
                anim_piece(&mut d, &gui.board_sec, &piece_tex, mv, anim_t);
            }
        }

        if gui.state != GameState::Ongoing {
            draw_markers(&mut d, &gui.current.board, &game_end_tex, &gui.board_sec, gui.state);
        }

        if gui.is_promotion {
            d.draw_rectangle_rec(gui.promotion_sec, PROMOTION_BACKGROUND);
            let color = gui.current.board.state.side as usize;
            for i in (PieceType::Knight as usize)..=(PieceType::Queen as usize) {
                let kind = i;
                let source_rect = Rectangle::new(
                    (kind as i32 * piece_tex.width() / 6) as f32,
                    (color as i32 * piece_tex.height() / 2) as f32,
                    (piece_tex.width() / 6) as f32,
                    (piece_tex.height() / 2) as f32,
                );
                let target_rect = Rectangle::new(
                    gui.promotion_sec.x + ((i-1) as f32) * gui.promotion_sec.width / 4.0,
                    gui.promotion_sec.y,
                    gui.promotion_sec.width / 4.0,
                    gui.promotion_sec.height
                );
                d.draw_texture_pro(
                    &piece_tex,
                    source_rect,
                    target_rect,
                    Vector2::zero(),
                    0.0,
                    Color::WHITE,
                );
            }
        }

        draw_players_name(&mut d, &font, &gui.info_sec, engine_a.name(), engine_b.name());
    }

    Ok(())
}
