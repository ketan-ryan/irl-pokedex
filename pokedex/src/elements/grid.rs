use iced::widget::canvas::{self, Geometry};
use iced::{Rectangle, Renderer, Theme, mouse, Point};

#[derive(Debug)]
pub struct Grid {
    pub offset: Vector,
    pub cache: canvas::Cache,
}

impl Grid {
    pub fn new() -> Self {
        Self {
            offset: Vector::new(0.0, 0.0),
            cache: canvas::Cache::new()
        }
    }

    pub fn tick(&mut self) {
        self.cache.clear();
        self.offset.x += 0.5;
        self.offset.y += 0.5;
    }
}

#[derive(Default, Debug)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
}

impl Vector {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,y
        }
    }
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
        let geo = self.cache.draw(renderer, bounds.size(), |frame| {
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
        });


        vec![geo]
    }
}
