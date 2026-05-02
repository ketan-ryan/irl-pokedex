use iced::widget::canvas::{self, Cache, Geometry, Path, Stroke};
use iced::{Color, Point, Radians, Rectangle, Renderer, Theme};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IconState {
    Unregistered,
    Registered,
}

pub struct RegisteredIconWidget {
    state: IconState,
    cache: Cache,
}

impl RegisteredIconWidget {
    pub fn new(state: IconState) -> Self {
        Self {
            state,
            cache: Cache::default(),
        }
    }
}

impl<Message> canvas::Program<Message> for RegisteredIconWidget {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            let center = frame.center();
            let radius = bounds.width.min(bounds.height) / 2.0 * 0.8; // 80% to add padding

            match self.state {
                IconState::Unregistered => {
                    // Draw empty circle
                    let circle = Path::circle(center, radius);
                    frame.stroke(
                        &circle,
                        Stroke::default().with_color(Color::BLACK).with_width(2.0),
                    );
                }
                IconState::Registered => {
                    // Draw top semicircle (red, filled)
                    let top_semicircle = Path::new(|builder| {
                        builder.move_to(Point::new(center.x - radius, center.y)); // Left point
                        builder.arc(canvas::path::Arc {
                            center,
                            radius,
                            start_angle: Radians::PI,     // Start at left
                            end_angle: Radians::PI * 2.0, // End at right (top half)
                        });
                        builder.close();
                    });

                    frame.fill(&top_semicircle, Color::from_rgb(1.0, 0.0, 0.0));

                    // Draw bottom semicircle (white, filled)
                    let bottom_semicircle = Path::new(|builder| {
                        builder.move_to(Point::new(center.x + radius, center.y)); // Right point
                        builder.arc(canvas::path::Arc {
                            center,
                            radius,
                            start_angle: iced::Radians(0.0), // Start at right
                            end_angle: Radians::PI,          // End at left (bottom half)
                        });
                        builder.close();
                    });

                    frame.fill(&bottom_semicircle, Color::WHITE);

                    let outline = Path::circle(center, radius);
                    frame.stroke(
                        &outline,
                        Stroke::default().with_color(Color::BLACK).with_width(2.0),
                    );
                }
            }
        });

        vec![geometry]
    }
}
