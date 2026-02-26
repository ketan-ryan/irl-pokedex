use iced::border;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Program};
use iced::widget::canvas::path::Builder;
use iced::{Color, Element, Point, Radians, Rectangle, Renderer, Theme, Vector};
use std::f32::consts::PI;
use std::time::{Duration, Instant};
use iced::widget::{image};
use iced::Animation;

use crate::screen::home::Message;

#[derive(Debug)]
pub struct SpinnerState {
    rotation: Animation<f32>,
    finishing: bool,
    finished: bool,
}

impl SpinnerState {
    pub fn new() -> Self {
        Self {
            rotation: Animation::new(0.0f32)
                .duration(Duration::from_millis(300))
                .easing(iced::animation::Easing::EaseInOut)
                .repeat_forever(),
            finishing: false,
            finished: false
        }
    }

    pub fn start(&mut self) {
        self.rotation.go_mut(std::f32::consts::TAU, Instant::now());
    }

    pub fn angle(&self) -> f32 {
        self.rotation.interpolate_with(|v| v, Instant::now())
    }

    pub fn go_to_baseline(&mut self) {
        // setup for transition out
        self.finishing = true;
        let angle = self.angle();
        self.rotation = Animation::new(angle);
        self.rotation.go_mut(PI / 4.0, Instant::now())
    }

    pub fn is_animating(&self) -> bool {
        self.rotation.is_animating(Instant::now())
    }
}

// The state that lives inside the canvas
#[derive(Debug)]
pub struct QuadState { 
    pub time: f32, // seconds, drives animation
    cache: canvas::Cache,
    half_ball_handle: image::Handle,
    ball_handle: image::Handle,
    spinner: SpinnerState,
    loading: bool,
    loaded_time: f32
}

impl QuadState {
    pub fn new() -> Self {
        let mut spinner = SpinnerState::new();
        spinner.start();

        Self {
            time: 0.0,
            cache: canvas::Cache::new(),
            half_ball_handle: image::Handle::from_bytes(
                include_bytes!("../../assets/pokeball_half.png").as_slice()
            ),
            ball_handle: image::Handle::from_bytes(
                include_bytes!("../../assets/pokeball.png").as_slice()
            ),
            spinner: spinner,
            loading: true,
            loaded_time: 0.0
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.time += dt;
        self.cache.clear(); // invalidate so canvas redraws

        if self.spinner.finishing {
            if !self.spinner.is_animating() {
                self.spinner.finished = true;
                println!("Finished spinning");
            }
        }
    }

    pub fn is_loading(&self) -> bool {
        return self.loading;
    }

    pub fn set_loaded(&mut self) {
        self.loading = false;
        self.loaded_time = self.time;
        self.spinner.go_to_baseline();
    }

    pub fn finished_spinning(&self) -> bool {
        return self.spinner.finished;
    }

    pub fn is_finishing(&self) -> bool {
        return self.spinner.finishing;
    }
}

// The widget itself
pub struct QuadCanvas<'a> {
    state: &'a QuadState,
}

impl<'a> QuadCanvas<'a> {
    pub fn new(state: &'a QuadState) -> Element<'a, Message> {
        Canvas::new(QuadCanvasProgram { state })
            .width(iced::Fill)
            .height(iced::Fill)
            .into()
    }
}

struct QuadCanvasProgram<'a> {
    state: &'a QuadState,
}

impl<'a> Program<Message> for QuadCanvasProgram<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let geometry = self.state.cache.draw(renderer, bounds.size(), |frame| {
            // draw_quads(frame, bounds, self.state.time);
            draw_img(frame, 80.0, 0.0, self.state);
        });

        vec![geometry]
    }
}

fn draw_img(
    frame: &mut Frame,
    cx: f32,
    cy: f32,
    state: &QuadState
) {
    let time = state.time;
    if time <= 0.25 {
        let handle = &state.half_ball_handle;
        frame.draw_image(
        Rectangle::new(
            Point::new(-600.0 + time.min(0.25) * 2700.0, cy),
                iced::Size::new(480.0, 480.0),
            ),
            canvas::Image {
                handle: handle.clone(),
                rotation: Radians(7.0 * PI / 4.0),
                opacity: 1.0,
                filter_method: iced::advanced::image::FilterMethod::Linear,
                snap: false,
                border_radius: border::radius(0)
            },
        );
        frame.draw_image(
        Rectangle::new(
            Point::new(1200.0 - time.min(0.25) * 4500.0, cy),
                iced::Size::new(480.0, 480.0),
            ),
            canvas::Image {
                handle: handle.clone(),
                rotation: Radians(3.0 * PI / 4.0),
                opacity: 1.0,
                filter_method: iced::advanced::image::FilterMethod::Linear,
                snap: false,
                border_radius: border::radius(0)
            },
        );
    } 
    if time >= 0.24 {
        let ball_handle = &state.ball_handle;
        let angle = state.spinner.angle();

        frame.draw_image(
        Rectangle::new(
            Point::new(cx, cy),
                iced::Size::new(480.0, 480.0),
            ),
            canvas::Image {
                handle: ball_handle.clone(),
                rotation: iced::Radians(angle),
                opacity: 1.0,
                filter_method: iced::advanced::image::FilterMethod::Linear,
                snap: false,
                border_radius: border::radius(0)
            },
        );
    }
}

fn draw_quads(frame: &mut Frame, bounds: Rectangle, time: f32) {
    let cx = bounds.width / 2.0;
    let cy = bounds.height / 2.0;

    let quads = [
        (cx - 480.0, cy, 10000.0, 80.0 + (time.min(0.25) * 3000.0), -45.0, Color::from_rgb(1.0, 0.0, 0.0)),
        (cx + 480.0, cy, 10000.0, 80.0 + (time.min(0.25) * 3000.0), -45.0, Color::from_rgb(0.0, 0.0, 0.0)),
    ];

    for (x, y, w, h, angle, color) in quads {
        let hw = w / 2.0;
        let hh = h / 2.0;

        frame.with_save(|frame| {
            frame.translate(Vector::new(x, y));
            frame.rotate(angle);

            let mut builder = Builder::new();
            builder.move_to(Point::new(-hw, -hh));
            builder.line_to(Point::new( hw, -hh));
            builder.line_to(Point::new( hw,  hh));
            builder.line_to(Point::new(-hw,  hh));
            builder.close();

            frame.fill(&builder.build(), color);
        });
    }
}
