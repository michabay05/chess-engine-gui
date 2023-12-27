use raylib::prelude::*;


struct Button {
    rect: Rectangle,
    color: Color,
    normal_color: Color,
    hover_color: Color,
}

impl Button {
    const SCALE_FACTOR: f32 = 1.25;
    fn from_content_rect(content_rect: Rectangle, color: Color) -> Self {
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
            rect,
            color,
            normal_color: color,
            hover_color: color.fade(0.8),
        }
    }

    fn update(&mut self, mouse_pos: Vector2) {
        if self.rect.check_collision_point_rec(mouse_pos) {
            self.color = self.hover_color;
        } else {
            self.color = self.normal_color;
        }
    }

    fn on_click<F>(&self, rl: &RaylibHandle, func: F)
        where F: Fn() {
        let mouse_pos = rl.get_mouse_position();
        if self.rect.check_collision_point_rec(mouse_pos) {
            if rl.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON) {
                func();
            }
        }
    }

    fn draw(&self, d: &mut RaylibDrawHandle) {
        d.draw_rectangle_rec(self.rect, self.color);
    }
}

const BACKGROUND: Color = Color::new(28, 28, 28, 255);

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1000, 600)
        .title("Button test")
        .build();
    rl.set_target_fps(60);

    let font = rl.load_font(&thread, "assets/fonts/Inter-Regular.ttf").unwrap();

    let test_rect = Rectangle {
        x: 100.0,
        y: 100.0,
        width: 140.0,
        height: 140.0
    };
    let mut btn = Button::from_content_rect(test_rect, Color::DARKGRAY);

    /* let mut btn_rect = Rectangle {
        x: 100.0,
        y: 100.0,
        width: 150.0,
        height: 110.0
    };
    let mut btn_clr = Color::DARKGRAY;
    let mut count = 0; */

    while !rl.window_should_close() {
        /*
        let mouse_pos = rl.get_mouse_position();
        if btn_rect.check_collision_point_rec(mouse_pos) {
            btn_clr = Color::GRAY;
            if rl.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON) {
                count += 1;
            }
        } else {
            btn_clr = Color::DARKGRAY;
        }

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(BACKGROUND);
        d.draw_rectangle_rec(btn_rect, btn_clr);
        let text_dim = text::measure_text_ex(&font, &count.to_string(), font.baseSize as f32, 0.0);
        let text_pos = Vector2::new(
            btn_rect.x + btn_rect.width/2.0 - text_dim.x / 2.0,
            btn_rect.y + btn_rect.height/2.0 - text_dim.y / 2.0
        );
        d.draw_text_ex(&font, &count.to_string(), text_pos, font.baseSize as f32, 0.0, Color::RAYWHITE); */

        let mouse_pos = rl.get_mouse_position();
        btn.update(mouse_pos);

        btn.on_click(&rl, || println!("Clicked!"));

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(BACKGROUND);
        btn.draw(&mut d);
        d.draw_rectangle_rec(test_rect, Color::RED);
    }
}
