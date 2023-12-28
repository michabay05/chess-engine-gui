mod utils;

use raylib::prelude::*;
use crate::utils::Button;

#[derive(Clone)]
enum ButtonType {
    RedDec,
    GreenDec,
    BlueDec,
    RedInc,
    GreenInc,
    BlueInc
}

const BACKGROUND: Color = Color::new(28, 28, 28, 255);

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1000, 600)
        .title("Button test")
        .build();
    rl.set_target_fps(60);

    let font = rl.load_font(&thread, "assets/fonts/Inter-Regular.ttf").unwrap();
    let icons = rl.load_texture(&thread, "assets/btn-icons.png").unwrap();
    icons.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);

    let mut btns = [
        Button::padded_content(ButtonType::RedDec, Rectangle {
            x: 100.0, y: 100.0, width: 70.0, height: 50.0
        }, Color::MAROON),
        Button::padded_content(ButtonType::GreenDec, Rectangle {
            x: 200.0, y: 100.0, width: 70.0, height: 50.0
        }, Color::DARKGREEN),
        Button::padded_content(ButtonType::BlueDec, Rectangle {
            x: 300.0, y: 100.0, width: 70.0, height: 50.0
        }, Color::DARKBLUE),
        Button::padded_content(ButtonType::RedInc, Rectangle {
            x: 600.0, y: 100.0, width: 70.0, height: 50.0
        }, Color::RED),
        Button::padded_content(ButtonType::GreenInc, Rectangle {
            x: 700.0, y: 100.0, width: 70.0, height: 50.0
        }, Color::GREEN),
        Button::padded_content(ButtonType::BlueInc, Rectangle {
            x: 800.0, y: 100.0, width: 70.0, height: 50.0
        }, Color::BLUE),
    ];

    let rect = Rectangle { x: 200.0, y: 200.0, width: 600.0, height: 300.0 };
    let mut color = Color::new(127, 127, 127, 255);

    while !rl.window_should_close() {
        let mouse_pos = rl.get_mouse_position();
        for btn in &mut btns {
            if btn.is_being_pressed(&rl) {
                match btn.kind() {
                    ButtonType::RedDec => color.r = color.r.saturating_sub(2),
                    ButtonType::GreenDec => color.g = color.g.saturating_sub(2),
                    ButtonType::BlueDec => color.b = color.b.saturating_sub(2),
                    ButtonType::RedInc => color.r = color.r.saturating_add(2),
                    ButtonType::GreenInc => color.g = color.g.saturating_add(2),
                    ButtonType::BlueInc => color.b = color.b.saturating_add(2),
                }
            }
        }


        let mut d = rl.begin_drawing(&thread);
        d.clear_background(BACKGROUND);

        let frame_width = icons.width() as f32 / 3.0;
        let source = Rectangle::new(frame_width, 0.0, frame_width, icons.height() as f32);
        for btn in &btns {
            btn.draw(&mut d, mouse_pos);
            // d.draw_rectangle_rec(btn.content_rect(), Color::BEIGE);

            let min_side = f32::min(btn.content_rect().width, btn.content_rect().height);
            let target = Rectangle {
                x: btn.content_rect().x + btn.content_rect().width / 2.0 - min_side / 2.0,
                y: btn.content_rect().y + btn.content_rect().height / 2.0 - min_side / 2.0,
                width: min_side,
                height: min_side,
            };
            // d.draw_rectangle_rec(target, Color::GRAY);
            let (center, rotation) = match btn.kind() {
                ButtonType::RedDec | ButtonType::GreenDec | ButtonType::BlueDec => {
                    (Vector2::zero(), 0.0)
                },
                ButtonType::RedInc | ButtonType::GreenInc | ButtonType::BlueInc => {
                    (Vector2::new(target.width, target.height), 180.0)
                },
            };
            d.draw_texture_pro(&icons, source, target, center, rotation, Color::WHITE);
        }

        d.draw_rectangle_rec(rect, color);
        d.draw_text_ex(&font, &format!("  RED: {}", color.r), Vector2::new(25.0, 250.0), font.baseSize as f32, 0.0, Color::RAYWHITE);
        d.draw_text_ex(&font, &format!("GREEN: {}", color.g), Vector2::new(25.0, 300.0), font.baseSize as f32, 0.0, Color::RAYWHITE);
        d.draw_text_ex(&font, &format!(" BLUE: {}", color.b), Vector2::new(25.0, 350.0), font.baseSize as f32, 0.0, Color::RAYWHITE);
    }
}
