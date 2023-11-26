use raylib::prelude::*;

use crate::attack::AttackInfo;
use crate::bb::BBUtil;
use crate::board::Board;
use crate::consts::{Piece, Sq};
use crate::fen::FEN_POSITIONS;
use crate::moves::{self, Move, MoveFlag, MoveUtil};
use crate::move_gen::{self, MoveList};
use crate::{COL, ROW, SQ};

const BACKGROUND: Color = Color::new(30, 30, 30, 255);

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

fn target_is_legal(board: &Board, attack_info: &AttackInfo, source: Sq, target: Sq) -> Option<Move> {
    let mut ml = MoveList::new();
    let piece = board.find_piece(source as usize);
    assert!(piece.is_some());
    move_gen::generate_by_piece(board, attack_info, &mut ml, piece.unwrap());
    // TODO: handle the promotion piece
    //                        vvvv 
    ml.search(source, target, None)
}

pub fn gui_main() -> Result<(), String> {
    let (mut rl, thread) = raylib::init()
        .size(900, 600)
        .title("Chess Engine GUI")
        .build();

    rl.set_window_min_size(900, 600);

    let mut board = Board::from_fen(FEN_POSITIONS[2]);
    let attack_info = AttackInfo::new();
    let piece_tex = rl.load_texture(&thread, "assets/pieceSpriteSheet.png")?;

    let mut selected = None;
    let mut target = None;
    let mut curr_move = None;
    while !rl.window_should_close() {
        /* ========== UPDATE PHASE ========== */
        let size = Vector2::new(rl.get_screen_width() as f32, rl.get_screen_height() as f32);
        let margin = Vector2::new(size.x * 0.01, size.y * 0.03);
        let min_side = f32::min((size.x - 2.0*margin.x) * 0.7, size.y - 2.0*margin.y);
        let boundary = Rectangle {
            x: margin.x,
            y: margin.y,
            width: min_side,
            height: min_side
        };

        if rl.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON) {
            let mouse_pos = rl.get_mouse_position();
            if boundary.check_collision_point_rec(mouse_pos) {
                let col = ((mouse_pos.x - boundary.x) / (boundary.width / 8.0)) as usize;
                let row = ((mouse_pos.y - boundary.y) / (boundary.height / 8.0)) as usize;
                let sq = Sq::from_num(SQ!(row, col));
                if selected.is_none() {
                    if let Some(piece) = board.find_piece(SQ!(row, col)) {
                        if (piece as usize / 6) == board.state.side as usize {
                            selected = Some(sq);
                        }
                    } else {
                        selected = None;
                    }
                } else {
                    /* if let Some(piece) = board.find_piece(SQ!(row, col)) {
                        if (piece as usize / 6) == board.state.side as usize {
                            selected = Some(sq);
                        }
                    } */
                    // target square selection
                    curr_move = target_is_legal(&board, &attack_info, selected.unwrap(), sq);
                    target = if curr_move.is_some() { Some(sq) } else { None };

                    selected = None;
                }
            } else {
                selected = None;
            }
        }

        if let Some(mv) = curr_move {
            let _ = moves::make(&mut board, &attack_info, mv, MoveFlag::AllMoves);
            curr_move = None;
        }

        /* ========== RENDER PHASE ========== */
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(BACKGROUND);
        draw_board(&mut d, &boundary, selected);
        draw_pieces(&mut d, &piece_tex, &board, &boundary);
    }

    Ok(())
}
