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

const BACKGROUND: Color = Color::new(30, 30, 30, 255);
const PROMOTION_BACKGROUND: Color = Color::new(46, 46, 46, 220);

const LIGHT_SQ_CLR: Color = Color::new(118, 150, 86, 255);
const LIGHT_SELECTED_CLR: Color = Color::new(187, 204, 68, 255);
const DARK_SQ_CLR: Color = Color::new(238, 238, 210, 255);
const DARK_SELECTED_CLR: Color = Color::new(244, 246, 128, 255);

fn draw_board(d: &mut RaylibDrawHandle, sec: &Rectangle, b_ui: &BoardUI) {
    assert!(sec.width == sec.height);
    let mut cell_size = Vector2::one();
    cell_size.scale(sec.width / 8.0);

    for r in 0..8 {
        for f in 0..8 {
            let light_sq = (r + f) % 2 != 0;
            let mut sq_clr = if light_sq { LIGHT_SQ_CLR } else { DARK_SQ_CLR };
            if let Some(sq) = b_ui.selected {
                if sq == SQ!(r, f) {
                    sq_clr = if (ROW!(sq) + COL!(sq)) % 2 != 0 { LIGHT_SELECTED_CLR } else { DARK_SELECTED_CLR };
                }
            }
            if let Some(sq) = b_ui.target {
                if sq == SQ!(r, f) {
                    sq_clr = if (ROW!(sq) + COL!(sq)) % 2 != 0 { LIGHT_SELECTED_CLR } else { DARK_SELECTED_CLR };
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

fn draw_pieces(d: &mut RaylibDrawHandle, tex: &Texture2D, board: &Board, sec: &Rectangle) {
    let min_side = f32::min(sec.width, sec.height);
    let mut cell_size = Vector2::one();
    cell_size.scale(min_side / 8.0);

    for r in 0..8 {
        for f in 0..8 {
            if let Some(piece) = board.find_piece(SQ!(r, f)) {
                let target = Rectangle::new(
                    sec.x + (f as f32) * cell_size.x,
                    sec.y + (r as f32) * cell_size.y,
                    cell_size.x,
                    cell_size.y
                );
                draw_piece(d, tex, target, piece);
            }
        }
    }
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


fn update_engine(frame_time: f32, engine: &mut EngineComm, fen: &str,
    eval: &mut i32, is_mate: &mut bool, selected: &mut Option<Sq>,
    target: &mut Option<Sq>, promoted_piece: &mut Option<Piece>
) {
    if !engine.is_searching() {
        engine.fen(fen);
        engine.search_movetime((SECONDS_PER_MOVE * 1000.0) as u64);
    } else {
        if !engine.search_time_over() {
            engine.update_time_left(frame_time);
        } else {
            if let Some(best_move) = engine.best_move(eval, is_mate) {
                assert!(best_move.len() == 4 || best_move.len() == 5, "Length: {}", best_move.len());
                println!("[{}] '{}'", best_move.len(), &best_move);
                // Move from engine: "b7b8q"
                *selected = Some(Sq::from_str(&best_move[0..2])); // "b7"
                *target = Some(Sq::from_str(&best_move[2..4]));   // "b8"
                *promoted_piece = if best_move.len() == 5 {       // 'q'
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

#[derive(Clone, Copy, Debug, PartialEq)]
enum GameState {
    Ongoing,
    LightWinByCheckmate,
    DarkWinByCheckmate,
    DrawByStalemate,
    DrawByFiftyMoveRule,
    DrawByThreefoldRepetition,
    DrawByKingVsKing,
    // TODO: missing draw by insufficient material
}

fn update_game_state(board: &mut Board, attack_info: &AttackInfo, zobrist_info: &ZobristInfo, boards: &Vec<BoardInfo>) -> GameState {
    // units[0] -> all the white pieces
    // units[1] -> all the black pieces
    // Since kings can't be captured, if both sides only have one piece
    // then that means that only kings are left on the board
    if board.pos.units[0].count_ones() == 1 && board.pos.units[1].count_ones() == 1 {
        return GameState::DrawByKingVsKing;
    }
    if board.state.half_moves >= 100 {
        return GameState::DrawByFiftyMoveRule;
    }
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
    // Check for checkmate or stalemate
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
    for pos in boards {
        let b = &pos.b_ui.board;
        if board.state.key == b.state.key && board.state.lock == b.state.lock {
            repetition_count += 1;
            if repetition_count == 3 {
                return GameState::DrawByThreefoldRepetition;
            }
        }
    }

    GameState::Ongoing
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
    selected: Option<usize>,
    target: Option<usize>,
    check: Option<usize>,
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

        self.selected = Some(sq);
    }

    fn highlight_target(&mut self, sq: usize) {
        self.target = Some(sq);
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

const SECONDS_PER_MOVE: f32 = 0.3;

pub fn gui_main(engine_a_path: String, engine_b_path: Option<String>) -> Result<(), String> {
    let attack_info = AttackInfo::new();
    let zobrist_info = ZobristInfo::new();

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
        .size(900, 600)
        .title("Chess Engine GUI")
        .resizable()
        .build();

    rl.set_window_min_size(900, 600);

    let piece_tex = rl.load_texture(&thread, "assets/pieceSpriteSheet.png")?;
    piece_tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);

    let font = rl.load_font(&thread, "assets/fonts/Inter-Regular.ttf")?;

    // Handle player input
    let mut selected = None;
    let mut target = None;
    let mut is_promotion = false;
    let mut promoted_piece = None;

    let mut play_game = false;
    let mut game_state = GameState::Ongoing;
    let mut is_engine_a = true;
    let mut eval = 0;
    let mut is_mate = false;

    let original_fen = FEN_POSITIONS[2];
    let mut fen = String::from(original_fen);
    fen = String::from("rnb1kb1r/p2p1ppp/4p3/2PpP3/q2P4/5Q2/P2B1PPP/RN2K1NR w KQkq - 3 12");
    let mut b_ui = BoardUI::from(&Board::from_fen(&fen, &zobrist_info));

    let mut history = Vec::<BoardInfo>::new();
    history.push(BoardInfo::from(&b_ui));
    let mut index = history.len() - 1;

    while !rl.window_should_close() {
        /* ==================== UPDATE PHASE ==================== */
        let size = Vector2::new(rl.get_screen_width() as f32, rl.get_screen_height() as f32);
        let margin = Vector2::new(size.x * 0.01, size.y * 0.03);
        let min_side = f32::min((size.x - 2.0*margin.x) * 0.7, size.y - 2.0*margin.y);
        let boundary = Rectangle {
            x: margin.x,
            y: margin.y,
            width: min_side,
            height: min_side
        };
        let promoted_height = boundary.height * 0.15;
        let promoted_width = 4.0 * promoted_height;
        let promoted_boundary = Rectangle {
            x: boundary.x + (boundary.width / 2.0) - (promoted_width / 2.0),
            y: boundary.y + (boundary.height / 2.0) - (promoted_height / 2.0),
            width: promoted_width,
            height: promoted_height,
        };

        let right_boundary = Rectangle {
            x: boundary.x + boundary.width + margin.x,
            y: boundary.y,
            width: size.x - (boundary.x + boundary.width + 2.0*margin.x),
            height: min_side,
        };

        if rl.is_key_pressed(KeyboardKey::KEY_SPACE) {
            if index == history.len() - 1 {
                play_game = !play_game;
            } else {
                eprintln!(
                    "[WARN] The board must be at the current position. You are {} moves behind the current.",
                    (history.len() - 1) - index
                );
            }
        } else if rl.is_key_pressed(KeyboardKey::KEY_LEFT) {
            if !play_game {
                index = index.saturating_sub(1);
                b_ui = history[index].b_ui.clone();
            }
        } else if rl.is_key_pressed(KeyboardKey::KEY_RIGHT) {
            if !play_game {
                index += 1;
                if index >= history.len() { index = history.len() - 1; };
                b_ui = history[index].b_ui.clone();
            }
        } else if rl.is_key_pressed(KeyboardKey::KEY_UP) {
            if !play_game {
                index = 0;
                b_ui = history[index].b_ui.clone();
            }
        } else if rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
            if !play_game {
                index = history.len() - 1;
                b_ui = history[index].b_ui.clone();
            }
        } else if rl.is_key_pressed(KeyboardKey::KEY_F) {
            let current_fen = fen::gen_fen(&b_ui.board);
            if rl.set_clipboard_text(&current_fen).is_err() {
                eprintln!("[ERROR] Failed to copy clipboard to fen");
            }
        } else if rl.is_key_pressed(KeyboardKey::KEY_S) {
            if !play_game {
                if save_game("game.pgn", engine_a.name(), engine_b.name(), &original_fen, &game_state, &history).is_err() {
                    return Err("Couldn't save game to file 'game.pgn'".to_string());
                }
            }
        }

        game_state = update_game_state(&mut b_ui.board, &attack_info, &zobrist_info, &history);

        if play_game && game_state != GameState::Ongoing {
            play_game = false;
        }

        if play_game {
            let engine = if is_engine_a { &mut engine_a } else { &mut engine_b };
            update_engine(rl.get_frame_time(), engine, &fen, &mut eval, &mut is_mate, &mut selected, &mut target, &mut promoted_piece);
            if let Some(sq) = selected {
                b_ui.highlight_selected(sq as usize);
            }
            if let Some(sq) = target {
                b_ui.highlight_target(sq as usize);
            }
            // Correct the promotion piece from the UCI string
            if let Some(piece) = promoted_piece {
                let mut piece_num = piece as usize % 6;
                if b_ui.board.state.side == PieceColor::Dark { piece_num += 6 };
                promoted_piece = Piece::from_num(piece_num);
            }
            if !engine.is_searching() {
                is_engine_a = !is_engine_a;
            }
        }
        // update_player(&rl, &mut board, &attack_info, &boundary, &promoted_boundary, &mut selected, &mut target,
        //     &mut is_promotion, &mut promoted_piece);

        if selected.is_some() && target.is_some() && !is_promotion {
            // DEBUG INFO
            // println!("Going to make move, here's the info:");
            // println!("           Source: {}", Sq::to_string(selected.unwrap()));
            // println!("           Target: {}", Sq::to_string(target.unwrap()));
            // println!("  Promotion piece: {:?}", promoted_piece);
            let curr_move = move_is_legal(&b_ui.board, &attack_info, selected.unwrap(), target.unwrap(), promoted_piece);
            // println!("\t = {}", curr_move.unwrap().to_str());
            if let Some(mv) = curr_move {
                if moves::make(&mut b_ui.board, &attack_info, &zobrist_info, mv, MoveFlag::AllMoves) {
                    fen = fen::gen_fen(&b_ui.board);
                    if let Some(b_info) = history.last_mut() {
                        b_info.update(mv, game_state);
                    }
                    history.push(BoardInfo::from(&b_ui));
                    index += 1;
                } else {
                    eprintln!("[ERROR] Illegal move! {}", mv.to_str());
                }
            }
            selected = None;
            target = None;
            promoted_piece = None;
            is_promotion = false;
        }

        /* ==================== RENDER PHASE ==================== */
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(BACKGROUND);
        draw_board(&mut d, &boundary, &b_ui);
        draw_pieces(&mut d, &piece_tex, &b_ui.board, &boundary);
        if is_promotion {
            d.draw_rectangle_rec(promoted_boundary, PROMOTION_BACKGROUND);
            let color = b_ui.board.state.side as usize;
            for i in (PieceType::Knight as usize)..=(PieceType::Queen as usize) {
                let kind = i;
                let source_rect = Rectangle::new(
                    (kind as i32 * piece_tex.width() / 6) as f32,
                    (color as i32 * piece_tex.height() / 2) as f32,
                    (piece_tex.width() / 6) as f32,
                    (piece_tex.height() / 2) as f32,
                );
                let target_rect = Rectangle::new(
                    promoted_boundary.x + ((i-1) as f32) * promoted_boundary.width / 4.0,
                    promoted_boundary.y,
                    promoted_boundary.width / 4.0,
                    promoted_boundary.height
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

        let side_to_move_text = if b_ui.board.state.side as usize == 0 { "White" } else { "Black" };
        d.draw_text_ex(&font, &side_to_move_text, Vector2::new(850.0, 600.0), font.baseSize as f32, 0.0, Color::WHITE);
        // let eval_text = if !is_mate {
        //     format!("{}{}",
        //         if eval > 0 { '+' } else { ' ' },
        //         eval as f32 / 100.0
        //     )
        // } else {
        //     format!("mate {}", eval)
        // };
        // // d.draw_text(&eval_text, 750, 500, 30, Color::WHITE);
        // d.draw_text_ex(&font, &eval_text, Vector2::new(750.0, 500.0), font.baseSize as f32, 0.0, Color::WHITE);
        d.draw_rectangle_lines_ex(right_boundary, 1, Color::RED);
        let players = &format!("{} vs {}", engine_a.name(), engine_b.name());
        let text_dim = measure_text_ex(&font, &players, font.baseSize as f32, 0.0);
        d.draw_text_ex(&font, &players,
            Vector2::new(right_boundary.x + (right_boundary.width / 2.0) - (text_dim.x / 2.0), right_boundary.height * 0.2),
            font.baseSize as f32, 0.0, Color::WHITE);

        let eval_text = if !is_mate {
            format!("{}{}",
                if eval > 0 { '+' } else { ' ' },
                eval as f32 / 100.0
            )
        } else {
            format!("mate {}", eval)
        };
        // d.draw_text(&eval_text, 750, 500, 30, Color::WHITE);
        d.draw_text_ex(&font, &eval_text, Vector2::new(750.0, 500.0), font.baseSize as f32, 0.0, Color::WHITE);
        d.draw_text_ex(&font, &format!("{:?}", game_state), Vector2::new(800.0, 400.0), font.baseSize as f32, 0.0, Color::WHITE);
    }

    Ok(())
}
