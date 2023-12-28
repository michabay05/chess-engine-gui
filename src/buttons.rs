use raylib::prelude::*;

struct Button {
    kind: ButtonType,
    rect: Rectangle,
    color: Color,
    normal_color: Color,
    hover_color: Color,
}

impl Button {
    const SCALE_FACTOR: f32 = 1.25;
    fn padded_content(kind: ButtonType, content_rect: Rectangle, color: Color) -> Self {
        let (width, height) = (
            Self::SCALE_FACTOR * content_rect.width,
            Self::SCALE_FACTOR * content_rect.height
        );
        let rect = Rectangle {
            x: content_rect.x - (width - content_rect.width)/2.0,
            y: content_rect.y - (height - content_rect.height)/2.0,
            width, height
        };
        Self {
            kind,
            rect,
            color,
            normal_color: color,
            hover_color: color.fade(0.65),
        }
    }

    fn update_color(&mut self, mouse_pos: Vector2) {
        if self.rect.check_collision_point_rec(mouse_pos) {
            self.color = self.hover_color;
        } else {
            self.color = self.normal_color;
        }
    }

    fn is_being_pressed(&self, rl: &RaylibHandle) -> bool {
        let mouse_pos = rl.get_mouse_position();
        if self.rect.check_collision_point_rec(mouse_pos) {
            return rl.is_mouse_button_down(MouseButton::MOUSE_LEFT_BUTTON);
        }
        return false;
    }

    fn is_clicked(&self, rl: &RaylibHandle) -> bool {
        let mouse_pos = rl.get_mouse_position();
        if self.rect.check_collision_point_rec(mouse_pos) {
            return rl.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON);
        }
        return false;
    }
}

enum ButtonType {
    RedDec,
    GreenDec,
    BlueDec,
    RedInc,
    GreenInc,
    BlueInc,
}

const BACKGROUND: Color = Color::new(28, 28, 28, 255);

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1000, 600)
        .title("Button test")
        .build();
    rl.set_target_fps(60);

    let font = rl.load_font(&thread, "assets/fonts/Inter-Regular.ttf").unwrap();

    let mut btns = [
        Button::padded_content(ButtonType::RedDec, Rectangle {
            x: 100.0, y: 100.0, width: 50.0, height: 50.0
        }, Color::MAROON),
        Button::padded_content(ButtonType::GreenDec, Rectangle {
            x: 200.0, y: 100.0, width: 50.0, height: 50.0
        }, Color::DARKGREEN),
        Button::padded_content(ButtonType::BlueDec, Rectangle {
            x: 300.0, y: 100.0, width: 50.0, height: 50.0
        }, Color::DARKBLUE),
        Button::padded_content(ButtonType::RedInc, Rectangle {
            x: 600.0, y: 100.0, width: 50.0, height: 50.0
        }, Color::RED),
        Button::padded_content(ButtonType::GreenInc, Rectangle {
            x: 700.0, y: 100.0, width: 50.0, height: 50.0
        }, Color::GREEN),
        Button::padded_content(ButtonType::BlueInc, Rectangle {
            x: 800.0, y: 100.0, width: 50.0, height: 50.0
        }, Color::BLUE),
    ];

    let rect = Rectangle {
        x: 200.0, y: 200.0, width: 600.0, height: 300.0
    };
    let mut color = Color::new(127, 127, 127, 255);

    while !rl.window_should_close() {
        let mouse_pos = rl.get_mouse_position();
        for btn in &mut btns {
            btn.update_color(mouse_pos);
        }

        let clicked_btn = btns.iter().find(|x| x.is_being_pressed(&rl));
        if let Some(btn) = clicked_btn {
            match btn.kind {
                ButtonType::RedDec => color.r = color.r.saturating_sub(2),
                ButtonType::GreenDec => color.g = color.g.saturating_sub(2),
                ButtonType::BlueDec => color.b = color.b.saturating_sub(2),
                ButtonType::RedInc => color.r = color.r.saturating_add(2),
                ButtonType::GreenInc => color.g = color.g.saturating_add(2),
                ButtonType::BlueInc => color.b = color.b.saturating_add(2),
            }
        }

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(BACKGROUND);

        for btn in &btns {
            d.draw_rectangle_rec(btn.rect, btn.color);
        }

        d.draw_rectangle_rec(rect, color);
        d.draw_text_ex(&font, &format!("  RED: {}", color.r), Vector2::new(25.0, 250.0), font.baseSize as f32, 0.0, Color::RAYWHITE);
        d.draw_text_ex(&font, &format!("GREEN: {}", color.g), Vector2::new(25.0, 300.0), font.baseSize as f32, 0.0, Color::RAYWHITE);
        d.draw_text_ex(&font, &format!(" BLUE: {}", color.b), Vector2::new(25.0, 350.0), font.baseSize as f32, 0.0, Color::RAYWHITE);
    }
}
