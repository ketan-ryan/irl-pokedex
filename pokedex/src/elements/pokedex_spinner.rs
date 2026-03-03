use iced::widget::Canvas;
use iced::widget::canvas::{self, Frame, Geometry, Program};
use iced::widget::canvas::path::{Arc, Builder};
use iced::{Color, Element, Point, Radians, Rectangle, Renderer, Theme, Vector};
use std::f32::consts::TAU;
use std::time::Instant;

use crate::screen::home::Message;

// The state that lives inside the canvas
#[derive(Debug)]
pub struct PokedexSpinnerState { 
    pub time: Instant
}

impl PokedexSpinnerState {
    pub fn new() -> Self {
        Self {
            time: Instant::now()
        }
    }

    pub fn set_time(&mut self) {
        self.time = Instant::now();
    }
}

pub struct SpinnerCanvas;

impl SpinnerCanvas {
    pub fn new<'a>(
        state: &'a PokedexSpinnerState,
    ) -> Element<'a, Message> {
        Canvas::new(SpinnerCanvasProgram { state })
            .width(iced::Fill)
            .height(iced::Fill)
            .into()
    }
}

struct SpinnerCanvasProgram<'a> {
    state: &'a PokedexSpinnerState,
}

impl<'a> Program<Message> for SpinnerCanvasProgram<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let cx = bounds.width / 2.0;
        let cy = bounds.height / 2.0;

        const CUTOUT_WIDTH: f32 = 20.0;
        let cutout_top = cy - CUTOUT_WIDTH;
        let cutout_bottom = cy + CUTOUT_WIDTH;

        // above cutout
        frame.with_clip(Rectangle::new(Point::ORIGIN, iced::Size::new(bounds.width, cutout_top)), |frame| {
            draw_spinner(frame, cx, cy, bounds.width.min(bounds.height) / 2.0, &self.state);
        });

        // below cutout
        frame.with_clip(Rectangle::new(Point::new(0.0, cutout_bottom), iced::Size::new(bounds.width, bounds.height - cutout_bottom)), |frame| {
            draw_spinner(frame, cx, cy, bounds.width.min(bounds.height) / 2.0, &self.state);
        });
        
        vec![frame.into_geometry()]
    }
}

fn draw_spinner(frame: &mut Frame, cx: f32, cy: f32, radius: f32, state: &PokedexSpinnerState) {
    let angle = state.time.elapsed().as_secs_f32();

    // arc length: TAU / n. Each arc is 1/n circle
    // gap between: TAU / n. n gaps evenly spaced
    let mut opacity = ((angle - 0.0) / 0.5).clamp(0.0, 1.0);
    opacity = 0.7 * opacity.clamp(0.0, 1.0);
    draw_arcs(
        frame, cx, cy, 
        radius - 60.0, 
        [TAU / 4.0; 3].as_slice(), 
        TAU / 3.0, 
        angle * 2.0, 
        Color::from_rgba(1.0, 1.0, 1.0, opacity)
    );
    if angle > 0.5 {
        let mut opacity = ((angle - 0.5) / 0.5).clamp(0.0, 1.0);
        opacity = 0.7 * opacity.clamp(0.0, 1.0);
        draw_arcs(
            frame, cx, cy, 
            radius - 87.0, 
            [TAU / 6.0, TAU / 16.0, TAU / 20.0].as_slice(), 
            TAU / 3.0, 
            -angle * 1.8, 
            Color::from_rgba(140.0 / 255.0, 213.0 / 255.0, 229.0 / 255.0, opacity)
        );
    }
    if angle > 1.0 {
        let mut opacity = ((angle - 1.0) / 0.5).clamp(0.0, 1.0);
        opacity = 0.7 * opacity.clamp(0.0, 1.0);
        draw_arcs(
            frame, cx, cy, 
            radius - 114.0, 
            [TAU / 8.0, TAU / 14.0].as_slice(), 
            TAU / 2.0, 
            angle * 3.0,
            Color::from_rgba(0.0, 156.0 / 255.0, 195.0 / 255.0, opacity)
        );
    }

}

fn draw_arcs(frame: &mut Frame, cx: f32, cy: f32, radius: f32, arc_lengths: &[f32], gap_between: f32, angle: f32, color: Color) {
    const STROKE_WIDTH: f32 = 25.0;

    let layers: &[(f32, f32)] = &[
        (STROKE_WIDTH + 0.0, color.a),  // core
        (STROKE_WIDTH + 5.0, color.a * 0.3),      // soft glow
        (STROKE_WIDTH + 10.0, color.a * 0.1),     // outer glow
    ];

    for (stroke_width, opacity) in layers {
        frame.with_save(|frame| {
            frame.translate(Vector::new(cx, cy));
            for i in 0..arc_lengths.len() {
                let arc_start = angle + (i as f32 * gap_between);
                let arc_end = arc_start + arc_lengths[i];
                let segments = create_segments(arc_start, arc_end);
                for (start, end) in segments {
                    let mut builder = Builder::new();
                    builder.arc(Arc {
                        center: Point::ORIGIN,
                        radius,
                        start_angle: Radians(start),
                        end_angle: Radians(end),
                    });
                    frame.stroke(
                        &builder.build(),
                        canvas::Stroke::default()
                            .with_width(*stroke_width)
                            .with_color(Color { r: color.r, g: color.g, b: color.b, a: *opacity })
                            .with_line_cap(canvas::LineCap::Round),
                    );
                }
            }
        });
    }
}

fn create_segments(
    start: f32,
    end: f32,
) -> Vec<(f32, f32)> {
    let arc_length = end - start;
    let start = start.rem_euclid(std::f32::consts::TAU);
    let end = start + arc_length;

    vec![(start, end)]
}
