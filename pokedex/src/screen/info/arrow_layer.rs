use iced::widget::canvas::{self, Frame, Geometry, LineCap, LineJoin, Path, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Theme, mouse};
use std::f32::consts::PI;

const HEAD: f32 = 11.0;
pub const ARROW: f32 = 42.0;

/// Draws arrows as vector paths: (center, angle in radians, 0 = right).
pub struct ArrowLayer {
    pub arrows: Vec<(Point, f32, f32)>,
}

impl<Message> canvas::Program<Message> for ArrowLayer {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let stroke = Stroke::default()
            .with_color(Color::WHITE)
            .with_width(3.0)
            .with_line_cap(LineCap::Round)
            .with_line_join(LineJoin::Round);

        for &(c, angle, len) in &self.arrows {
            let (sin, cos) = angle.sin_cos();
            let half = len / 2.0;
            let tail = Point::new(c.x - cos * half, c.y - sin * half);
            let tip = Point::new(c.x + cos * half, c.y + sin * half);
            let head = |spread: f32| {
                let (s, co) = (angle + PI + spread).sin_cos();
                Point::new(tip.x + co * HEAD, tip.y + s * HEAD)
            };

            let path = Path::new(|b| {
                b.move_to(tail);
                b.line_to(tip);
                b.move_to(head(0.5));
                b.line_to(tip);
                b.line_to(head(-0.5));
            });
            frame.stroke(&path, stroke);
        }
        vec![frame.into_geometry()]
    }
}
