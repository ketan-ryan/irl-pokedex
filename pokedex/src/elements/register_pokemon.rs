use iced::widget::image;
use iced::widget::Canvas;
use iced::widget::canvas::{self, Geometry, Program};
use iced::{Animation, Element, Point, Radians, Rectangle, Renderer, Theme};
use std::time::{Instant, Duration};

use crate::screen::home::Message;


#[derive(Debug)]
pub struct RegisterPokemonState { 
    pub time: Instant,
    pub white_fade: Animation<f32>,
    pub cache: canvas::Cache,
    pub full_fade: Animation<f32>,
    pub offset: Option<f32>,
    pub white_handle: Option<image::Handle>,
    pub png_handle: Option<image::Handle>,
}

impl RegisterPokemonState {
    pub fn new() -> Self {
        let white_fade = Animation::new(0.0f32)
            .duration(Duration::from_millis(600))
            .easing(iced::animation::Easing::EaseOut);

        let full_fade = Animation::new(0.0f32)
            .duration(Duration::from_millis(800))
            .easing(iced::animation::Easing::EaseOut);
        
        Self {
            time: Instant::now(),
            cache: canvas::Cache::new(),
            white_fade,
            full_fade,
            white_handle: None,
            png_handle: None,
            offset: None
        }
    }

    pub fn init(&mut self, white_handle: image::Handle, png_handle: image::Handle, offset: f32) {
        self.time = Instant::now();
        self.white_handle = Some(white_handle);
        self.png_handle = Some(png_handle);
        self.offset = Some(offset);
        self.white_fade.go_mut(1.0, Instant::now());
    }

    pub fn start_full(&mut self) {
        self.full_fade.go_mut(1.0, Instant::now());
    }

    pub fn current_white(&self) -> f32 {
        self.white_fade.interpolate_with(|v| v, Instant::now())
    }

    pub fn current_full_fade(&self) -> f32 {
        self.full_fade.interpolate_with(|v| v, Instant::now())
    }

    pub fn tick(&mut self) {
        self.cache.clear();
        if self.current_white() == 1.0 && self.current_full_fade() == 0.0 {
            self.start_full();
        }
    }
}

pub struct RegisterCanvas;

impl RegisterCanvas {
    pub fn new<'a>(
        state: &'a RegisterPokemonState,
    ) -> Element<'a, Message> {
        Canvas::new(RegisterCanvasProgram { state })
            .width(iced::Fill)
            .height(iced::Fill)
            .into()
    }
}

struct RegisterCanvasProgram<'a> {
    state: &'a RegisterPokemonState,
}

impl<'a> Program<Message> for RegisterCanvasProgram<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        if self.state.offset.is_none() {
            return vec![];
        }
        const PADDING: f32 = 100.0;

        let x_diff = 0.5 - self.state.offset.unwrap(); // diff to center
        let x_offset = x_diff * bounds.width; // scale to pixels

        // cheap trick - since we are using square images, we can set the width to the height.
        // otherwise, we'd have to calculate the image aspect ratio, since adjusting the x_pos of the
        // top-left coord shifts the bounds width. Also, subtract some padding. 
        let dim_y = bounds.height - PADDING;
  
        // first, center the image
        let px = (bounds.width / 2.0) - (dim_y / 2.0);
        let py = (bounds.height / 2.0) - (dim_y / 2.0);

        // then, apply translation
        let px = px + x_offset;

        let geometry = self.state.cache.draw(renderer, bounds.size(), |frame| {
            if self.state.current_full_fade() < 1.0 {
                frame.draw_image(
            Rectangle::new(Point::new(px, py), iced::Size::new(dim_y, dim_y)),
                    canvas::Image {
                        handle: self.state.white_handle.clone().unwrap(),
                        filter_method: iced::advanced::image::FilterMethod::Linear,
                        rotation: Radians(0.0),
                        border_radius: iced::border::radius(0),
                        opacity: self.state.current_white(),
                        snap: false,
                    },
                );
            }

            frame.draw_image(
        Rectangle::new(Point::new(px, py), iced::Size::new(dim_y, dim_y)),
                canvas::Image {
                    handle: self.state.png_handle.clone().unwrap(),
                    filter_method: iced::advanced::image::FilterMethod::Linear,
                    rotation: Radians(0.0),
                    border_radius: iced::border::radius(0),
                    opacity: self.state.current_full_fade(),
                    snap: false,
                },
            );
        });
        vec![geometry]
    }
}