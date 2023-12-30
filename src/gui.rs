use raylib::prelude::*;

use crate::comm::EngineComm;
use crate::utils::Button;
use crate::pgn;

use chess::attack::AttackInfo;
use chess::bb::BBUtil;
use chess::board::Board;
use chess::consts::{Piece, PieceColor, PieceType, Sq};
use chess::fen::{self, FEN_POSITIONS};
use chess::moves::{self, Move, MoveFlag, MoveUtil};
use chess::move_gen::{self, MoveList};
use chess::zobrist::ZobristInfo;
use chess::{COL, ROW, SQ};

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
        x: sec.x + (COL!(light_king) as f32 * sec.width / 8.0) + (0.9 * sec.width / 8.0) - target_width / 2.0,
        y: sec.y + (ROW!(light_king) as f32 * sec.height / 8.0) + (0.05 * sec.height / 8.0) - target_height / 2.0,
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
        x: sec.x + (COL!(dark_king) as f32 * sec.width / 8.0) + (0.9 * sec.width / 8.0) - target_width / 2.0,
        y: sec.y + (ROW!(dark_king) as f32 * sec.height / 8.0) + (0.05 * sec.height / 8.0) - target_height / 2.0,
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
pub enum GameState {
    Ongoing,
    LightWinByCheckmate,
    DarkWinByCheckmate,
    LightLostOnTime,
    DarkLostOnTime,
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

pub struct BoardInfo {
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

    pub fn mv(&self) -> Option<Move> {
        self.mv.clone()
    }

    pub fn board(&self) -> &Board {
        &self.b_ui.board
    }

    pub fn is_checkmate(&self) -> bool {
        self.game_state == GameState::LightWinByCheckmate
            || self.game_state == GameState::DarkWinByCheckmate
    }

    pub fn check(&self) -> bool {
        self.b_ui.check.is_some()
    }

    pub fn is_ongoing(&self) -> bool {
        self.game_state == GameState::Ongoing
    }

    fn update(&mut self, mv: Move, game_state: GameState) {
        self.mv = Some(mv);
        self.game_state = game_state;
    }
}


fn draw_players_name(d: &mut RaylibDrawHandle, font: &Font, sec: &Rectangle, light_name: &str, dark_name: &str) {
    let margin = Vector2::new(sec.width * 0.01, sec.height * 0.03);

    let text = &format!("{}   vs    {}", light_name, dark_name);
    let text_dim = text::measure_text_ex(font, text, font.baseSize as f32, 0.0);

    let rect_width = f32::max(sec.width * 0.9, 1.5*text_dim.x);
    let rect_height = text_dim.y * 1.65;
    let bkgd_rect = Rectangle {
        x: sec.x + (sec.width - margin.x) / 2.0 - rect_width / 2.0,
        y: sec.y + margin.y + 0.05 * (sec.height - margin.y),
        width: rect_width,
        height: rect_height
    };

    d.draw_rectangle_lines_ex(bkgd_rect, 3, Color::RAYWHITE);
    let text_pos = Vector2::new(
        bkgd_rect.x + bkgd_rect.width/2.0 - text_dim.x/2.0,
        bkgd_rect.y + bkgd_rect.height/2.0 - text_dim.y/2.0,
    );
    d.draw_text_ex(&font, text, text_pos, font.baseSize as f32, 0.0, Color::RAYWHITE);
}

fn draw_moves(s: &mut impl RaylibDraw, sec: &mut Rectangle, font: &Font, moves: &Vec<BoardInfo>, current: usize) -> Rectangle {
    let mut move_counter = 1;
    let mut x;
    let mut y = 0.0;
    let gap = font.baseSize as f32 * 1.5;
    let each_height = font.baseSize as f32 * 2.0;
    let mut draw_bkgd = false;
    let mut curr_move_rect = Rectangle::default();
    // [ (move number) (gap 1) (white's move) (gap 2) (black's move) ]
    // [ (   0.05    ) ( 0.2 ) (   0.325    ) ( 0.1 ) (   0.325    ) ]
    for (i, b_info) in moves.iter().enumerate() {
        let mv = b_info.mv;
        if mv.is_none() { break; }
        let mv = mv.unwrap().to_str();
        let mv = mv.trim();

        if i % 2 == 0 {
            y = sec.y + (each_height * (i as f32)/2.0) + gap;
            if draw_bkgd {
                s.draw_rectangle_rec(
                    Rectangle::new(sec.x, y - (each_height - gap), sec.width, each_height),
                    MOVELIST_LIGHT_BKGD
                );
            }
            draw_bkgd = !draw_bkgd;

            x = sec.x + (0.05*sec.width);
            s.draw_text_ex(font, &move_counter.to_string(), Vector2::new(x, y),
                font.baseSize as f32, 0.0, Color::GRAY);
            move_counter += 1;

            if (y + each_height) - sec.y > sec.height {
                sec.height += gap;
            }
            x = sec.x + 0.25*sec.width;
        } else {
            x = sec.x + 0.675*sec.width;
        }
        let curr_ind = current.saturating_sub(1);
        if i == curr_ind {
            let text_dim = text::measure_text_ex(font, mv, font.baseSize as f32, 0.0);
            let (pad_horz, pad_vert) = (0.75*text_dim.x, 0.5*text_dim.y);
            curr_move_rect = Rectangle::new(x - pad_horz/2.0, y - pad_vert/2.0, text_dim.x + pad_horz, text_dim.y + pad_vert);
            s.draw_rectangle_rounded(curr_move_rect, 0.2, 10, Color::DARKGRAY);
        }
        s.draw_text_ex(font, mv, Vector2::new(x, y), font.baseSize as f32, 0.0, Color::RAYWHITE);
    }
    curr_move_rect
}

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
    other: BoardUI,
    other_index: usize,
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
    info_sec: Rectangle,

    move_list_sec: Rectangle,
    move_list_rect: Rectangle,
    curr_move_rect: Rectangle,
    follow_move_list: bool,
}

impl GUI {
    fn new(zobrist_info: &ZobristInfo) -> Self {
        Self::from_fen(FEN_POSITIONS[1], zobrist_info)
    }

    fn from_fen(fen: &str, zobrist_info: &ZobristInfo) -> Self {
        let board = Board::from_fen(fen, zobrist_info);
        let current = BoardUI::from(&board);
        let other = current.clone();
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
            other,
            other_index: 0,
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

            move_list_sec: Rectangle::default(),
            move_list_rect: Rectangle::default(),
            curr_move_rect: Rectangle::default(),
            follow_move_list: false,
        }
    }

    fn init_sections(&mut self, width: i32, height: i32) {
        let size = Vector2::new(width as f32, height as f32);
        let margin = Vector2::new(size.x * 0.01, size.y * 0.03);
        self.update_sections(size, margin);
        self.move_list_rect = self.move_list_sec;
    }

    fn update_sections(&mut self, size: Vector2, margin: Vector2) {
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
        let height = 0.5*self.info_sec.height;
        self.move_list_sec = Rectangle {
            x: self.info_sec.x,
            y: self.info_sec.y + height,
            width: self.info_sec.width,
            height: self.info_sec.height - height
        };
        self.move_list_rect = Rectangle {
            y: self.move_list_rect.y,
            height: self.move_list_rect.height,
            ..self.move_list_sec
        };
    }

    fn handle_scrolling(&mut self, rl: &RaylibHandle) {
        let wheel_move = rl.get_mouse_wheel_move();
        self.move_list_rect.y += wheel_move * 100.0;

        let sec = &mut self.move_list_sec;
        let rect = &mut self.move_list_rect;

        if self.follow_move_list {
            if self.curr_move_rect.y + self.curr_move_rect.height < sec.y {
                rect.y += sec.height;
            }
            if self.curr_move_rect.y + self.curr_move_rect.height > sec.y + sec.height {
                rect.y -= sec.height;
            }
        }

        if rect.y + (rect.height - sec.height) < sec.y {
            rect.y = sec.y - (rect.height - sec.height);
        }

        if rect.y > sec.y {
            rect.y = sec.y;
        }
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
                is_legal = true;
                self.current_fen = fen::gen_fen(self.anim_target_board.as_ref().unwrap());
                if let Some(b_info) = self.history.last_mut() {
                    b_info.update(mv, self.state);
                }
                if self.state == GameState::Ongoing {
                    self.history.push(BoardInfo::from(&self.current));
                } else {
                    println!("Game ended by {:?}", self.state);
                }
                self.current_index = self.history.len() - 1;
                self.other_index = self.current_index;

                // TO BE REMOVED
                self.other = BoardUI {
                    board: self.anim_target_board.clone().unwrap().clone(),
                    ..self.current.clone()
                };
                // =======================================

                let b = self.anim_target_board.as_ref().unwrap();
                if b.is_in_check(attack_info, b.state.xside) {
                    self.current.highlight_check(b.state.side as usize == 0);
                }

                self.anim_mv = Some(mv);
                self.anim_start_time = Instant::now();
                self.is_animating = true;
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

    fn go_to_start(&mut self) {
        if !self.play_game {
            self.other_index = 0;
            self.other = self.history[self.other_index].b_ui.clone();
        }
    }

    fn go_to_current(&mut self) {
        if !self.play_game {
            self.other_index = self.current_index;
            self.other = BoardUI {
                board: self.current.board.clone(),
                ..self.history[self.other_index].b_ui.clone()
            };
        }
    }

    fn go_to_first_move(&mut self) {
        if !self.play_game {
            self.other_index = 1;
            self.other = BoardUI {
                board: self.history[self.other_index + 1].b_ui.board.clone(),
                ..self.history[self.other_index].b_ui.clone()
            };
        }
    }

    fn undo_move(&mut self) {
        if !self.play_game {
            if self.other_index == 1 {
                self.go_to_first_move();
                return;
            } else if self.other_index == 0 {
                self.go_to_start();
                return;
            }
            println!("[UNDO] current_index = {}", self.current_index);
            println!("[UNDO] other_index = {}\n", self.other_index);
            self.other = BoardUI {
                board: self.history[self.other_index].b_ui.board.clone(),
                ..self.history[self.other_index - 1].b_ui.clone()
            };
            self.other_index = self.other_index.saturating_sub(1);
        }
    }

    fn forward_move(&mut self) {
        if !self.play_game {
            if self.other_index >= self.history.len() - 2 {
                self.go_to_current();
                return;
            }
            self.other_index += 1;
            println!("[FORW] current_index = {}", self.current_index);
            println!("[FORW] other_index = {}\n", self.other_index);
            self.other = BoardUI {
                board: self.history[self.other_index + 1].b_ui.board.clone(),
                ..self.history[self.other_index].b_ui.clone()
            };
        }
    }

    fn play(&mut self) {
        if self.current_index == self.history.len() - 1 {
            self.play_game = true;
            self.follow_move_list = true;
        } else {
            self.current_index = self.history.len() - 1;
            self.toggle_play();
        }
    }

    fn pause(&mut self) {
        if self.play_game {
            self.play_game = false;
            self.follow_move_list = false;
        }
    }

    fn toggle_play(&mut self) {
        if self.play_game {
            self.pause();
        } else {
            self.play();
        }
    }
}

const SECONDS_PER_MOVE: f32 = 1.25;

const MOVELIST_LIGHT_BKGD: Color = Color::new(28, 28, 28, 255);
const MOVELIST_DARK_BKGD: Color = Color::new(22, 22, 22, 255);
const MOVE_BTN_COLOR: Color = Color::new(48, 48, 48, 255);

#[derive(Clone, Debug)]
enum MoveButtonType {
    First,
    Previous,
    PlayPause,
    Next,
    Last
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
        .resizable()
        .msaa_4x()
        .build();

    rl.set_window_min_size(1000, 600);
    rl.set_target_fps(60);

    let piece_tex = rl.load_texture(&thread, "assets/chesscom-pieces/chesscom_pieces.png")?;
    piece_tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
    let game_end_tex = rl.load_texture(&thread, "assets/chesscom-pieces/game-end-icons.png")?;
    game_end_tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
    let btn_icons = rl.load_texture(&thread, "assets/btn-icons.png")?;
    btn_icons.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);

    let font = rl.load_font(&thread, "assets/fonts/Inter-Regular.ttf")?;
    let move_list_font = rl.load_font_ex(&thread, "assets/fonts/Inter-Medium.ttf", 20, FontLoadEx::Default(0))?;
    let bold_font = rl.load_font(&thread, "assets/fonts/Inter-Bold.ttf")?;

    // let mut gui = GUI::new(&zobrist_info);
    let mut gui = GUI::from_fen("4r1k1/3p1p2/3p1Q2/B2P4/P1r5/6P1/5K1P/8 b - - 0 33", &zobrist_info);
    gui.init_sections(rl.get_screen_width(), rl.get_screen_height());

    let mut is_engine_a = true;

    while !rl.window_should_close() {
        /* ==================== UPDATE PHASE ==================== */
        let mouse_pos = rl.get_mouse_position();
        let size = Vector2::new(rl.get_screen_width() as f32, rl.get_screen_height() as f32);
        let margin = Vector2::new(size.x * 0.01, size.y * 0.03);
        gui.update_sections(size, margin);
        gui.handle_scrolling(&rl);

        let sec = gui.info_sec;
        let sec_margin = sec.width * 0.05;
        let width = (sec.width - 2.0*sec_margin) / 6.5;
        let height = f32::min(60.0, sec.height * 0.1);
        let y = gui.move_list_sec.y - margin.x - height;
        let mut move_btns = [
            Button::padded_content(MoveButtonType::First, Rectangle {
                x: sec.x + sec_margin, y, width, height
            }, MOVE_BTN_COLOR),
            Button::padded_content(MoveButtonType::Previous, Rectangle {
                x: sec.x + sec_margin + sec_margin + width, y, width, height
            }, MOVE_BTN_COLOR),
            Button::padded_content(MoveButtonType::PlayPause, Rectangle {
                x: sec.x + sec_margin + 2.0*(sec_margin + width), y, width, height
            }, MOVE_BTN_COLOR),
            Button::padded_content(MoveButtonType::Next, Rectangle {
                x: sec.x + sec_margin + 3.0*(sec_margin + width), y, width, height
            }, MOVE_BTN_COLOR),
            Button::padded_content(MoveButtonType::Last, Rectangle {
                x: sec.x + sec_margin + 4.0*(sec_margin + width), y, width, height
            }, MOVE_BTN_COLOR),
        ];

        for btn in &move_btns {
            if btn.is_clicked(&rl) {
                match btn.kind() {
                    MoveButtonType::First => gui.go_to_start(),
                    MoveButtonType::Previous => gui.undo_move(),
                    MoveButtonType::PlayPause => gui.toggle_play(),
                    MoveButtonType::Next => gui.forward_move(),
                    MoveButtonType::Last => gui.go_to_current(),
                }
            }
        }

        if rl.is_key_pressed(KeyboardKey::KEY_SPACE) {
            gui.toggle_play();
        } else if rl.is_key_pressed(KeyboardKey::KEY_ONE) {
            gui.go_to_first_move();
        } else if rl.is_key_pressed(KeyboardKey::KEY_LEFT) {
            gui.undo_move();
        } else if rl.is_key_pressed(KeyboardKey::KEY_RIGHT) {
            gui.forward_move();
        } else if rl.is_key_pressed(KeyboardKey::KEY_UP) {
            gui.go_to_start();
        } else if rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
            gui.go_to_current();
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
                if pgn::save("game.pgn", engine_a.name(), engine_b.name(), &gui.original_fen, &gui.state, &attack_info, &gui.history).is_err() {
                    return Err("Couldn't save game to file 'game.pgn'".to_string());
                }
            }
        }

        if gui.state == GameState::Ongoing {
            gui.update_state(&attack_info, &zobrist_info);
        }

        if gui.play_game && gui.state != GameState::Ongoing {
            gui.play_game = false;
            gui.follow_move_list = false;
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
        let b_ui = if gui.play_game { &gui.current } else { &gui.other };
        draw_board(&mut d, &gui.board_sec, b_ui);
        draw_coords(&mut d, &bold_font, &gui.board_sec);
        let skip_sq = if gui.current_index == gui.history.len() - 1 { b_ui.selected } else { None };
        // let skip_sq = gui.history[gui.current_index].b_ui.selected;
        draw_pieces(&mut d, skip_sq, &piece_tex, b_ui, &gui.board_sec);

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

        {
            let mut s = d.begin_scissor_mode(
                gui.move_list_sec.x as i32,
                gui.move_list_sec.y as i32,
                gui.move_list_sec.width as i32,
                gui.move_list_sec.height as i32,
            );
            gui.curr_move_rect = draw_moves(&mut s, &mut gui.move_list_rect, &move_list_font, &gui.history, gui.other_index);
            s.draw_rectangle_lines_ex(gui.move_list_sec, 3, Color::RAYWHITE);
        }

        d.draw_rectangle_lines_ex(gui.info_sec, 2, Color::RED);

        for btn in &move_btns {
            btn.draw(&mut d, mouse_pos);
            let min_side = f32::min(btn.content_rect().width, btn.content_rect().height);
            let target = Rectangle {
                x: btn.content_rect().x + btn.content_rect().width / 2.0 - min_side / 2.0,
                y: btn.content_rect().y + btn.content_rect().height / 2.0 - min_side / 2.0,
                width: min_side,
                height: min_side,
            };
            let frame_width = btn_icons.width() as f32 / 3.0;
            let (ind, center, rotation) = match *btn.kind() {
                MoveButtonType::First => (2, Vector2::zero(), 0.0),
                MoveButtonType::Previous => (1, Vector2::zero(), 0.0),
                MoveButtonType::PlayPause => (0, Vector2::zero(), 0.0),
                MoveButtonType::Next => (1, Vector2::new(target.width, target.height), 180.0),
                MoveButtonType::Last => (2, Vector2::new(target.width, target.height), 180.0),
            };
            let source = Rectangle::new(ind as f32*frame_width, 0.0, frame_width, btn_icons.height() as f32);
            d.draw_texture_pro(&btn_icons, source, target, center, rotation, Color::WHITE);
        }
    }
    
    Ok(())
}