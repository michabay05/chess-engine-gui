use raylib::prelude::*;

use crate::attack::AttackInfo;
use crate::bb::BBUtil;
use crate::board::Board;
use crate::consts::{Piece, PieceColor, PieceType, Sq};
use crate::fen::{self, FEN_POSITIONS};
use crate::moves::{self, Move, MoveFlag, MoveUtil};
use crate::move_gen::{self, MoveList};
use crate::{COL, ROW, SQ};

const BACKGROUND: Color = Color::new(30, 30, 30, 255);
const PROMOTION_BACKGROUND: Color = Color::new(46, 46, 46, 220);

const LIGHT_SQ_CLR: Color = Color::new(118, 150, 86, 255);
const LIGHT_SELECTED_CLR: Color = Color::new(187, 204, 68, 255);
const DARK_SQ_CLR: Color = Color::new(238, 238, 210, 255);
const DARK_SELECTED_CLR: Color = Color::new(244, 246, 128, 255);

fn draw_board(d: &mut RaylibDrawHandle, sec: &Rectangle, selected: Option<Sq>) {
    assert!(sec.width == sec.height);
    let mut cell_size = Vector2::one();
    cell_size.scale(sec.width / 8.0);

    for r in 0..8 {
        for f in 0..8 {
            let mut sq_clr = if (r + f) % 2 != 0 { LIGHT_SQ_CLR } else { DARK_SQ_CLR };
            if let Some(sq) = selected {
                let sq = sq as usize;
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
    d.draw_rectangle_lines_ex(sec, 1, Color::RED);
}

fn draw_pieces(d: &mut RaylibDrawHandle, tex: &Texture2D, board: &Board, sec: &Rectangle) {
    let min_side = f32::min(sec.width, sec.height);
    let mut cell_size = Vector2::one();
    cell_size.scale(min_side / 8.0);

    for r in 0..8 {
        for f in 0..8 {
            let piece = board.find_piece(SQ!(r, f));
            if piece.is_none() { continue; }
            let (color, kind) = Piece::to_tuple(piece);
            let source_rect = Rectangle::new(
                (kind as i32 * tex.width() / 6) as f32,
                (color as i32 * tex.height() / 2) as f32,
                (tex.width() / 6) as f32,
                (tex.height() / 2) as f32,
            );
            let target_rect = Rectangle::new(
                sec.x + (f as f32) * cell_size.x,
                sec.y + (r as f32) * cell_size.y,
                cell_size.x,
                cell_size.y
            );
            d.draw_texture_pro(
                &tex,
                source_rect,
                target_rect,
                Vector2::zero(),
                0.0,
                Color::WHITE,
            );
        }
    }
}

fn target_is_legal(board: &Board, attack_info: &AttackInfo, source: Sq, target: Sq, promoted: Option<Piece>) -> Option<Move> {
    let mut ml = MoveList::new();
    let piece = board.find_piece(source as usize);
    assert!(piece.is_some());
    move_gen::generate_by_piece(board, attack_info, &mut ml, piece.unwrap());
    ml.print();
    ml.search(source, target, promoted)
}


fn handle_board_selected(
    rl: &RaylibHandle, board: &Board, board_sec: &Rectangle, selected: &mut Option<Sq>
) {
    if rl.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON) {
        let mouse_pos = rl.get_mouse_position();
        // if selected.is_some() { return; }
        let mut temp_selected = None;
        if board_sec.check_collision_point_rec(mouse_pos) {
            let col = ((mouse_pos.x - board_sec.x) / (board_sec.width / 8.0)) as usize;
            let row = ((mouse_pos.y - board_sec.y) / (board_sec.height / 8.0)) as usize;
            temp_selected = Some(Sq::from_num(SQ!(row, col)));
        }
        /* if let Some(sq) = temp_selected {
            if board.find_piece(sq as usize).is_none() { return; }
        } */
        let sq = temp_selected.unwrap();
        if let Some(piece) = board.find_piece(sq as usize) {
            if piece as usize / 6 != board.state.side as usize { return; }
        } else {
            return;
        }
        if temp_selected == *selected {
            *selected = None;
            return;
        }
        *selected = temp_selected;
        /* if selected.is_some() {
            println!("Source: {}", Sq::to_string(selected.unwrap()));
        } else {
            println!("Source: None");
        } */
        // println!("Source: {}", Sq::to_string(selected.unwrap()));
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
        /* if target.is_some() {
            println!("Target: {}\n", Sq::to_string(target.unwrap()));
        } else {
            println!("Target: None");
        } */
        // println!("Target: {}\n", Sq::to_string(target.unwrap()));
        let piece = board.find_piece(selected.unwrap() as usize);
        if piece.is_none() { return; }
        let piece = piece.unwrap();
        let sq = temp_selected.unwrap();
        if (piece == Piece::LP || piece == Piece::DP)
            && (ROW!(sq as usize) == 0 || ROW!(sq as usize) == 7) {
            *is_promotion = true;
        }
        /* if let Some(sq) = temp_selected {
            if (piece == Piece::LP || piece == Piece::DP)
                && (ROW!(sq as usize) == 0 || ROW!(sq as usize) == 7) {
                *is_promotion = true;
            }
        } */
    }
}

pub fn gui_main(engine_a: String, engine_b: Option<String>) -> Result<(), String> {
    let mut board = Board::from_fen(FEN_POSITIONS[3]);
    let attack_info = AttackInfo::new();

    let (mut rl, thread) = raylib::init()
        .size(900, 600)
        .title("Chess Engine GUI")
        .build();

    rl.set_window_min_size(900, 600);

    let piece_tex = rl.load_texture(&thread, "assets/pieceSpriteSheet.png")?;
    piece_tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);

    let mut selected = None;
    let mut target = None;

    let mut is_promotion = false;
    let mut promoted_piece = None;
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

        if is_promotion {
            let mouse_pos = rl.get_mouse_position();
            if rl.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON)
                && promoted_boundary.check_collision_point_rec(mouse_pos) {
                let mut piece = (mouse_pos.x / (promoted_boundary.width / 4.0)).trunc() as usize;
                if board.state.side == PieceColor::Dark {
                    piece += 6;
                }
                promoted_piece = match piece {
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
                is_promotion = false;
            }
        }
        handle_board_selected(&rl, &board, &boundary, &mut selected);
        handle_board_target(&rl, &board, &boundary, &selected, &mut target, &mut is_promotion);

        if selected.is_some() && target.is_some() && !is_promotion {
            // DEBUG INFO
            // println!("Going to make move, here's the info:");
            // println!("           Source: {}", Sq::to_string(selected.unwrap()));
            // println!("           Target: {}", Sq::to_string(target.unwrap()));
            // println!("  Promotion piece: {:?}", promoted_piece);
            let curr_move = target_is_legal(&board, &attack_info, selected.unwrap(), target.unwrap(), promoted_piece);
            // println!("\t = {}", curr_move.unwrap().to_str());
            if let Some(mv) = curr_move {
                // Since the legality of the move has been checked, the return value isn't used
                if !moves::make(&mut board, &attack_info, mv, MoveFlag::AllMoves) {
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
        draw_board(&mut d, &boundary, selected);
        draw_pieces(&mut d, &piece_tex, &board, &boundary);
        if is_promotion {
            d.draw_rectangle_rec(promoted_boundary, PROMOTION_BACKGROUND);
            let color = board.state.side as usize;
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
        let side_to_move_text = if board.state.side as usize == 0 { "White" } else { "Black" };
        d.draw_text(side_to_move_text, 750, 300, 30, Color::WHITE);
    }

    Ok(())
}
