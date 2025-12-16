use iced::widget::canvas::{self, Geometry, Frame};
use iced::{Color, Rectangle, Renderer, Theme, mouse, Point};

#[derive(Debug)]
pub struct Grid {
    pub offset: Vector,
}

#[derive(Default, Debug)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
}

impl<Message> canvas::Program<Message> for Grid {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        let spacing = 10.0;

        let x_offset = self.offset.x % spacing;
        let y_offset = self.offset.y % spacing;

        // vertical lines
        let mut x = -x_offset;
        while x < bounds.width {
            frame.stroke(
                &canvas::Path::line(
                    Point::new(x, 0.0),
                    Point::new(x, bounds.height),
                ),
                canvas::Stroke {
                    width: 1.0,
                    ..Default::default()
                },
            );
            x += spacing;
        }

        // horizontal lines
        let mut y = -y_offset;
        while y < bounds.height {
            frame.stroke(
                &canvas::Path::line(
                    Point::new(0.0, y),
                    Point::new(bounds.width, y),
                ),
                canvas::Stroke {
                    width: 1.0,
                    ..Default::default()
                },
            );
            y += spacing;
        }

        vec![frame.into_geometry()]
    }
}
