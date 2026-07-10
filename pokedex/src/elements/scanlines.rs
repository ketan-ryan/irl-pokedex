use std::time::Duration;

use iced::widget::canvas::{self, Geometry};
use iced::{Color, Point, Rectangle, Renderer, Theme, mouse};

#[derive(Debug)]
pub struct Scanlines {
    pub offset: Vector,
    pub cache: canvas::Cache,
}

impl Scanlines {
    /// Create a new scrolling scanlines window with a zeroed offset and an empty draw cache.
    ///
    /// Returns: A new grid instance ready for animation.
    pub fn new() -> Self {
        Self {
            offset: Vector::new(0.0, 0.0),
            cache: canvas::Cache::new(),
        }
    }

    /// Advance the grid animation by the given elapsed time and invalidate the cached drawing.
    ///
    /// Args:
    /// - dt: The elapsed time since the last update.
    ///
    /// Returns: Nothing.
    pub fn tick(&mut self, dt: Duration) {
        self.cache.clear();

        // px per second
        let speed = 10.0;
        let seconds = dt.as_secs_f32();

        self.offset.y -= speed * seconds;
    }
}

#[derive(Default, Debug)]
pub struct Vector {
    #[allow(unused)]
    pub x: f32,
    pub y: f32,
}

impl Vector {
    /// Create a new vector with the provided x and y components.
    ///
    /// Args:
    /// - x: The horizontal component.
    /// - y: The vertical component.
    ///
    /// Returns: A new vector instance.
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl<Message> canvas::Program<Message> for Scanlines {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let geo = self.cache.draw(renderer, bounds.size(), |frame| {
            let spacing = 10.0;

            let y_offset = self.offset.y % spacing;

            // horizontal lines
            let mut y = -y_offset;
            while y < bounds.height {
                frame.stroke(
                    &canvas::Path::line(Point::new(0.0, y), Point::new(bounds.width, y)),
                    canvas::Stroke {
                        width: 2.0,
                        style: canvas::Style::Solid(Color::from_rgba8(238, 242, 255, 0.2)),
                        ..Default::default()
                    },
                );
                y += spacing;
            }
        });

        vec![geo]
    }
}
