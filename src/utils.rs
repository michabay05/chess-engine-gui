use raylib::prelude::*;

pub struct Button<T> {
    kind: T,
    content_rect: Rectangle,
    boundary: Rectangle,
    color: Color,
    cooldown: f32,
}

impl<T: Clone> Button<T> {
    const SCALE_FACTOR: f32 = 1.25;
    const COOLDOWN_TIME: f32 = 0.05;

    pub fn padded_content(kind: T, content_rect: Rectangle, color: Color) -> Self {
        let (width, height) = (
            Self::SCALE_FACTOR * content_rect.width,
            Self::SCALE_FACTOR * content_rect.height
        );
        let boundary = Rectangle {
            x: content_rect.x - (width - content_rect.width)/2.0,
            y: content_rect.y - (height - content_rect.height)/2.0,
            width, height
        };
        Self {
            kind,
            content_rect,
            boundary,
            color,
            cooldown: 0.0,
        }
    }

    pub fn kind(&self) -> &T {
        &self.kind
    }

    pub fn content_rect(&self) -> Rectangle {
        self.content_rect
    }

    pub fn set_content_rect(&mut self, new_rect: Rectangle) {
        *self = Self::padded_content(self.kind.clone(), new_rect, self.color);
    }

    pub fn draw(&self, d: &mut RaylibDrawHandle, mouse_pos: Vector2) {
        let color = if self.boundary.check_collision_point_rec(mouse_pos) {
            self.color.fade(0.6)
        } else {
            self.color
        };
        // d.draw_rectangle_rec(self.boundary, color);
        d.draw_rectangle_rounded(self.boundary, 0.15, 10, color);
    }

    pub fn is_being_pressed(&mut self, rl: &RaylibHandle) -> bool {
        let mouse_pos = rl.get_mouse_position();
        if self.boundary.check_collision_point_rec(mouse_pos) {
            if rl.is_mouse_button_down(MouseButton::MOUSE_LEFT_BUTTON) {
                self.cooldown += rl.get_frame_time();
                if self.cooldown >= Self::COOLDOWN_TIME {
                    self.cooldown = 0.0;
                    return true;
                }
            }
        }
        false
    }

    pub fn is_clicked(&self, rl: &RaylibHandle) -> bool {
        let mouse_pos = rl.get_mouse_position();
        if self.boundary.check_collision_point_rec(mouse_pos) {
            return rl.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON);
        }
        return false;
    }
}
