use raylib::prelude::*;

use chess::attack::AttackInfo;
use chess::bb::BBUtil;
use chess::board::Board;
use chess::consts::{Piece, Sq};
use chess::fen;
use chess::moves::{Move, MoveUtil};
use chess::move_gen::{self, MoveList};
use chess::zobrist::ZobristInfo;
use chess::{COL, ROW, SQ};

use crate::comm::EngineComm;
use crate::game::{Game, GameState};
use crate::utils::Button;
use crate::game_manager::GameManager;

use std::time::Instant;

const BACKGROUND: Color = Color::new(30, 30, 30, 255);
const PROMOTION_BACKGROUND: Color = Color::new(46, 46, 46, 220);

const LIGHT_SQ_CLR: Color = Color::new(118, 150, 86, 255);
const LIGHT_SELECTED_CLR: Color = Color::new(187, 204, 68, 255);
const DARK_SQ_CLR: Color = Color::new(238, 238, 210, 255);
const DARK_SELECTED_CLR: Color = Color::new(244, 246, 128, 255);

// TODO: display checks
fn draw_board(d: &mut RaylibDrawHandle, sec: &Rectangle, source: Option<Sq>, target: Option<Sq>) {
    let mut cell_size = Vector2::one();
    cell_size.scale(sec.width / 8.0);

    for r in 0..8 {
        for f in 0..8 {
            let light_sq = (r + f) % 2 != 0;
            let mut sq_clr = if light_sq { LIGHT_SQ_CLR } else { DARK_SQ_CLR };
            if let Some(sq) = source {
                let sq = sq as usize;
                if sq == SQ!(r, f) {
                    sq_clr = if (ROW!(sq) + COL!(sq)) % 2 != 0 { LIGHT_SELECTED_CLR } else { DARK_SELECTED_CLR };
                }
            }
            if let Some(sq) = target {
                let sq = sq as usize;
                if sq == SQ!(r, f) {
                    sq_clr = if (ROW!(sq) + COL!(sq)) % 2 != 0 { LIGHT_SELECTED_CLR } else { DARK_SELECTED_CLR };
                }
            }
            /*
            if let Some(sq) = b_ui.check {
                let sq = sq as usize;
                if sq == SQ!(r, f) {
                    let check_clr = Color::new(189, 55, 55, 255);
                    sq_clr = Color::color_alpha_blend(&sq_clr, &check_clr, &Color::new(255, 255, 255, 200));
                }
            }
            */

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
        GameState::LightLostOnTime => Some((6, 0)),
        GameState::DarkLostOnTime => Some((0, 7)),
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

fn draw_moves(s: &mut impl RaylibDraw, sec: &mut Rectangle, font: &Font, game: &Game, current: usize) -> Rectangle {
    let mut move_counter = 1;
    let mut x;
    let mut y = 0.0;
    let gap = font.baseSize as f32 * 1.5;
    let each_height = font.baseSize as f32 * 2.0;
    let mut draw_bkgd = false;
    let mut curr_move_rect = Rectangle::default();
    // [ (move number) (gap 1) (white's move) (gap 2) (black's move) ]
    // [ (   0.05    ) ( 0.2 ) (   0.325    ) ( 0.1 ) (   0.325    ) ]
    // for (i, b_info) in moves.iter().enumerate() {
    for i in 0..game.move_count() {
        let mv = game.move_at(i);
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

/* ===================================== USER INPUT RELATED ===================================== */
/*
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
*/
/* ===================================== USER INPUT RELATED ===================================== */

fn get_move_from_engine(frame_time: f32, current_fen: &str, engine: &mut EngineComm) -> Option<String> {
    let mut retry_count = 0;
    while retry_count < 2 {
        if let Some(best_move) = engine.best_move() {
            assert!(best_move.len() == 4 || best_move.len() == 5, "Length: {}", best_move.len());
            if best_move == "a8a8P" {
                retry_count += 1;
                println!("Retry because of 'a8a8P'");
                continue;
            }
            // println!("[{}] '{}'", best_move.len(), &best_move);
            return Some(best_move);
        } else {
            println!("Retry because NO MOVE was sent by engine. ");
            retry_count += 1;
        }
    }
    eprintln!("[ERROR] Engine, '{}' couldn't give a legal move", engine.name());
    return None;
}

struct GUI {
    selected: Option<Sq>,
    target: Option<Sq>,
    is_promotion: bool,
    promoted_piece: Option<Piece>,

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
    fn new() -> Self {
        Self {
            selected: None,
            target: None,
            is_promotion: false,
            promoted_piece: None,

            // Sections on the screen
            board_sec: Rectangle::default(),
            promotion_sec: Rectangle::default(),
            info_sec: Rectangle::default(),

            move_list_sec: Rectangle::default(),
            move_list_rect: Rectangle::default(),
            curr_move_rect: Rectangle::default(),
            follow_move_list: true,
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
        if wheel_move != 0.0 {
            self.follow_move_list = false;
        }

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

}

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
    let engine_a = engine_a.unwrap();
    let engine_b = engine_b.unwrap();

    let mut manager = GameManager::new(engine_a, engine_b, &zobrist_info);

    // Rendering initializations
    let (mut rl, thread) = raylib::init()
        .size(1000, 600)
        .title("Chess Engine GUI")
        .resizable()
        .msaa_4x()
        .build();

    rl.set_window_min_size(1000, 600);
    rl.set_target_fps(60);

    // Loading all the necessary textures
    let piece_tex = rl.load_texture(&thread, "assets/chesscom-pieces/chesscom_pieces.png")?;
    piece_tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
    let game_end_tex = rl.load_texture(&thread, "assets/chesscom-pieces/game-end-icons.png")?;
    game_end_tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
    let btn_icons = rl.load_texture(&thread, "assets/move-player-icons.png")?;
    btn_icons.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);

    // Load all the needed fonts
    let font = rl.load_font(&thread, "assets/fonts/Inter-Regular.ttf")?;
    let move_list_font = rl.load_font_ex(&thread, "assets/fonts/Inter-Medium.ttf", (rl.get_screen_width() as f32 * 0.02) as i32, FontLoadEx::Default(0))?;
    let bold_font = rl.load_font(&thread, "assets/fonts/Inter-Bold.ttf")?;

    let mut gui = GUI::new();
    gui.init_sections(rl.get_screen_width(), rl.get_screen_height());

    // let mut game = Game::new(engine_a.name(), engine_b.name(), &zobrist_info);
    // Start a new game
    // game.set_start_pos(fen::FEN_POSITIONS[1], &zobrist_info);

    // Engine handle
    // let mut play_game = false;
    let mut is_engine_a = true;
    let mut engine_move_str: Option<String> = None;

    // Move Animations
    let mut anim_start_time = Instant::now();
    let mut anim_mv: Option<Move> = None;
    let mut is_animating = false;
    let mut anim_board = manager.current_game().board_after_last_move().cloned().unwrap();
    let mut anim_target_board = None;
    let anim_duration_secs = 0.2;

    let mut move_index: usize = 0;
    let mut new_input = false;

    let mut source = None;
    let mut target = None;

    while !rl.window_should_close() {
        /* ==================== UPDATE PHASE ==================== */
        let mouse_pos = rl.get_mouse_position();
        let size = Vector2::new(rl.get_screen_width() as f32, rl.get_screen_height() as f32);
        let margin = Vector2::new(size.x * 0.01, size.y * 0.03);
        gui.update_sections(size, margin);
        gui.handle_scrolling(&rl);

        // Move buttons
        let sec = gui.info_sec;
        let sec_margin = sec.width * 0.05;
        let width = (sec.width - 2.0*sec_margin) / 6.5;
        let height = f32::min(45.0, sec.height * 0.1);
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
                    MoveButtonType::First => move_index = 0,
                    MoveButtonType::Previous => move_index = move_index.saturating_sub(1),
                    MoveButtonType::PlayPause => manager.toggle_playing(),
                    MoveButtonType::Next => {
                        move_index += 1;
                        if move_index >= manager.current_move_count() {
                            move_index = manager.current_move_count() - 1;
                        }
                    },
                    MoveButtonType::Last => move_index = manager.current_move_count() - 1,
                }
                new_input = true;
                gui.follow_move_list = true;
            }
        }

        if rl.is_key_pressed(KeyboardKey::KEY_SPACE) {
            manager.toggle_playing();
            if manager.playing() && !gui.follow_move_list {
                gui.follow_move_list = true;
            }
            new_input = true;
        } else if rl.is_key_pressed(KeyboardKey::KEY_LEFT) {
            gui.follow_move_list = true;
            move_index = move_index.saturating_sub(1);
            new_input = true;
        } else if rl.is_key_pressed(KeyboardKey::KEY_RIGHT) {
            gui.follow_move_list = true;
            move_index += 1;
            if move_index >= manager.current_move_count() {
                move_index = manager.current_move_count() - 1;
            }
            new_input = true;
        } else if rl.is_key_pressed(KeyboardKey::KEY_UP) {
            gui.follow_move_list = true;
            move_index = 0;
            new_input = true;
        } else if rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
            gui.follow_move_list = true;
            move_index = manager.current_move_count() - 1;
            new_input = true;
        } else if rl.is_key_pressed(KeyboardKey::KEY_F) {
            let game = manager.current_game();
            let current_fen = game.current_fen();
            if rl.set_clipboard_text(&current_fen).is_err() {
                eprintln!("[ERROR] Failed to copy clipboard to fen");
            }
        } else if rl.is_key_pressed(KeyboardKey::KEY_N) {
            manager.start_new_game(&fens, &zobrist_info);
            move_index = 0;
            // let game = manager.current_game();
            // anim_board = game.board_after_move(move_index).cloned().unwrap();
        }

        manager.check_state();
        manager.update_time_left(rl.get_frame_time());
        /*
        if game.is_ongoing() && play_game {
            let engine = if is_engine_a { &mut engine_a } else { &mut engine_b };
            if !engine.is_searching() {
                engine.fen(&game.current_fen());
                engine.search_movetime((SECONDS_PER_MOVE * 1000.0) as u64);
                is_engine_a = !is_engine_a;
            } else if !engine.search_time_over() {
                engine.update_time_left(rl.get_frame_time());
            } else {
                engine_move_str = get_move_from_engine(rl.get_frame_time(), &game.current_fen(), engine);
            }
        }
        */
        // engine_move_str = manager.comm_with_engine(rl.get_frame_time());

        if let Some(mv) = manager.play(rl.get_frame_time(), &attack_info, &zobrist_info) {
            move_index += 1;

            is_animating = true;
            anim_start_time = Instant::now();
            anim_mv = Some(mv);
            let game = manager.current_game();
            anim_target_board = game.board_after_last_move().cloned();
        }

        /*
        if let Some(ref mv_str) = engine_move_str.take() {
            let mut found_move = None;
            if let Some(board) = game.board_after_last_move() {
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
                let _ = game.make_move(mv, &attack_info, &zobrist_info);
                move_index += 1;

                is_animating = true;
                anim_start_time = Instant::now();
                anim_mv = Some(mv);
                anim_target_board = game.board_after_last_move().cloned();
                // anim_target_board = game.board_after_move(move_index).cloned();
            } else {
                // eprintln!("[ERROR] Couldn't find engine's move in the current position");
                // eprintln!(" - Move: {mv_str}");
                // eprintln!(" -  FEN: {}", game.current_fen());
            }
        }
        */

        /* ==================== RENDER PHASE ==================== */
        fn draw_pieces(d: &mut RaylibDrawHandle, skip_sq: Option<Sq>, tex: &Texture2D, board: &Board, sec: &Rectangle) {
            for r in 0..8 {
                for f in 0..8 {
                    let sq = SQ!(r, f);
                    if let Some(s_sq) = skip_sq {
                        if s_sq as usize == sq {
                            continue;
                        }
                    }
                    if let Some(piece) = board.find_piece(sq) {
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

        let game = manager.current_game();

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(BACKGROUND);

        if !manager.playing() && new_input {
            anim_mv = game.move_at(move_index).copied();
            anim_board = game.board_before_move(move_index).cloned().unwrap();
            anim_target_board = game.board_after_move(move_index).cloned();
            new_input = false;
            is_animating = true;
        }

        if let Some(mv) = anim_mv {
            source = Some(mv.source());
            target = Some(mv.target());
        };
        draw_board(&mut d, &gui.board_sec, source, target);
        draw_coords(&mut d, &bold_font, &gui.board_sec);
        let skip_sq = if is_animating { source } else { None };
        draw_pieces(&mut d, skip_sq, &piece_tex, &anim_board, &gui.board_sec);

        if let Some(mv) = anim_mv {
            // anim_t = (NOW - anim_start_time) / ANIM_DURATION_SECS;
            let elapsed = Instant::now().duration_since(anim_start_time);
            let anim_t = elapsed.div_f32(anim_duration_secs).as_secs_f32();
            if is_animating && anim_t >= 1.0 {
                is_animating = false;
                anim_mv = None;
                if let Some(board) = anim_target_board.take() {
                    anim_board = board;
                }
                // Instantly make the move by drawing the target board
                draw_pieces(&mut d, None, &piece_tex, &anim_board, &gui.board_sec);
            }

            if is_animating {
                anim_piece(&mut d, &gui.board_sec, &piece_tex, mv, anim_t);
            }
        }

        if !game.is_ongoing() && move_index == manager.current_move_count() {
            draw_markers(&mut d, &anim_board, &game_end_tex, &gui.board_sec, game.state());
        }
        draw_players_name(&mut d, &font, &gui.info_sec, game.white_name(), game.black_name());

        for btn in &move_btns {
            btn.draw(&mut d, mouse_pos);
            let min_side = f32::min(btn.content_rect().width, btn.content_rect().height);
            let target = Rectangle {
                x: btn.content_rect().x + btn.content_rect().width / 2.0 - min_side / 2.0,
                y: btn.content_rect().y + btn.content_rect().height / 2.0 - min_side / 2.0,
                width: min_side,
                height: min_side,
            };
            let frame_width = btn_icons.width() as f32 / 6.0;
            let ind = match *btn.kind() {
                MoveButtonType::First     => 4,
                MoveButtonType::Previous  => 2,
                MoveButtonType::Next      => 3,
                MoveButtonType::Last      => 5,
                MoveButtonType::PlayPause => {
                    if manager.playing() { 1 } else { 0 }
                }
            } as f32;
            let source = Rectangle::new(ind*frame_width, 0.0, frame_width, btn_icons.height() as f32);
            d.draw_texture_pro(&btn_icons, source, target, Vector2::zero(), 0.0, Color::WHITE);
        }

        let (white_time, black_time) = manager.time_left();

        fn format_time(time: f32) -> String {
            let seconds = time / 1000.0;
            let (min, spare_seconds) = ((seconds/60.0).trunc(), seconds % 60.0);
            // If the time left is less than 20 seconds, display the tenths decimal place
            if time > 20.0 * 1000.0 {
                format!("{}:{:02}", min, spare_seconds.trunc())
            } else {
                // Padding(prepending) zeros and rounding f32 with decimals
                // Source: https://stackoverflow.com/questions/49778643/how-to-format-an-f32-with-a-specific-precision-and-prepended-zeros
                format!("0:{:04.1}", spare_seconds)
            }
        }
        let (white_time_str, black_time_str) = (format_time(white_time), format_time(black_time));
        d.draw_text_ex(&font, &white_time_str, Vector2::new(1400.0, 300.0), font.baseSize as f32, 0.0, Color::WHITE);
        d.draw_text_ex(&font, &black_time_str, Vector2::new(1600.0, 300.0), font.baseSize as f32, 0.0, Color::WHITE);

        let mut s = d.begin_scissor_mode(
            gui.move_list_sec.x as i32,
            gui.move_list_sec.y as i32,
            gui.move_list_sec.width as i32,
            gui.move_list_sec.height as i32,
        );
        gui.curr_move_rect = draw_moves(&mut s, &mut gui.move_list_rect, &move_list_font, &game, move_index);
        s.draw_rectangle_lines_ex(gui.move_list_sec, 3, Color::RAYWHITE);
    }

    Ok(())
}
