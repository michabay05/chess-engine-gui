use raylib::prelude::*;

use crate::bb::BBUtil;
use crate::board::Board;
use crate::consts::Piece;
use crate::fen::FEN_POSITIONS;
use crate::SQ;

const BACKGROUND: Color = Color::new(30, 30, 30, 255);

const LIGHT_SQ_CLR: Color = Color::new(118, 150, 86, 255);
const LIGHT_SELECTED_CLR: Color = Color::new(244, 246, 128, 255);
const DARK_SQ_CLR: Color = Color::new(238, 238, 210, 255);
const DARK_SELECTED_CLR: Color = Color::new(187, 204, 68, 255);

fn draw_board(d: &mut RaylibDrawHandle, sec: &Rectangle) {
    let min_side = f32::min(sec.width, sec.height);
    let mut cell_size = Vector2::one();
    cell_size.scale(min_side / 8.0);

    for r in 0..8 {
        for f in 0..8 {
            let sq_clr = if (r + f) % 2 != 0 { LIGHT_SQ_CLR } else { DARK_SQ_CLR };
            d.draw_rectangle_v(
                Vector2::new(
                    (sec.width - min_side) / 2.0 + (sec.x + (f as f32) * cell_size.x),
                    sec.y + (r as f32) * cell_size.y
                ),
                cell_size,
                sq_clr
            );
        }
    }
    // d.draw_rectangle_lines_ex(sec, 2, Color::RED);
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
                (sec.width - min_side) / 2.0 + (sec.x + (f as f32) * cell_size.x),
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
    // d.draw_rectangle_lines_ex(sec, 2, Color::RED);
}

pub fn gui_main() -> Result<(), String> {
    let (mut rl, thread) = raylib::init()
        .size(900, 600)
        .title("Chess Engine GUI")
        .build();

    let board = Board::from_fen(FEN_POSITIONS[1]);
    let piece_tex = rl.load_texture(&thread, "assets/pieceSpriteSheet.png")?;

    while !rl.window_should_close() {
        let size = Vector2::new(rl.get_screen_width() as f32, rl.get_screen_height() as f32);
        let margin = Vector2::new(size.x * 0.01, size.y * 0.03);
        let mut boundary = Rectangle {
            x: margin.x,
            y: margin.y,
            width: (size.x - 2.0*margin.x) * 0.7,
            height: size.y - 2.0*margin.y
        };
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(BACKGROUND);
        draw_board(&mut d, &boundary);
        draw_pieces(&mut d, &piece_tex, &board, &boundary);
    }

    Ok(())
}
