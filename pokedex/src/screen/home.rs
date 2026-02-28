use iced::{
    Center, Color, Element, Fill, Subscription, Task, time
};
use iced::widget::{
    column, container, text, stack, canvas::Canvas, image
};

use std::time::Duration;

use crate::elements::gstreamer_stream::{VideoError, VideoFrame, gstreamer_stream};
use crate::elements::loading_screen::{QuadCanvas, QuadState};
use crate::grid::Grid;


#[derive(Debug)]
pub struct Home {
    title: String,
    processing: bool,
    grid: Grid,
    last_frame: Option<image::Handle>,
    loading: bool,
    quad_state: QuadState,
    time: f32,
    gstreamer_error: Option<String>
}

#[derive(Debug, Clone)]
pub enum Message {
    HomeToggled,
    Refresh,
    Load,
    Tick(Duration),
    FrameReceived(VideoFrame),
    GSTError(VideoError)
}

pub enum Action {
    None,
    GoHome,
    RedrawWindows,
    Run(Task<Message>),
}

impl Home {
    pub fn new() -> (Self, Task<Message>) {
        println!("New home created");
        (
            Self { 
                title: String::from("Home page"),
                processing: false,
                grid: Grid { offset: crate::grid::Vector { x: 0.0, y: 0.0 } },
                last_frame: None,
                loading: true,
                quad_state: QuadState::new(),
                time: 0.0,
                gstreamer_error: None
            },
            Task::done(Message::Load)
        )
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::HomeToggled => {
                self.processing = true;
                Action::None
            }
            Message::Refresh => {
                Action::GoHome
            }
            Message::Load => {
                Action::None
            }
            Message::Tick(duration) => {
                self.grid.offset.x += 0.5;
                self.grid.offset.y += 0.5;

                if self.quad_state.is_loading() 
                    || self.quad_state.is_finishing() 
                    || !self.quad_state.finished_spinning() 
                    && self.gstreamer_error.is_none()
                {
                    self.quad_state.tick(duration.as_secs_f32());
                } else {
                    self.loading = false;
                }
                
                self.time += duration.as_secs_f32();
                if self.time > 3.0 && !self.quad_state.is_finishing() && self.quad_state.is_loading() {
                    self.quad_state.set_loaded();
                }

                Action::RedrawWindows
            }
            Message::FrameReceived(frame) => {
                self.last_frame = Some(image::Handle::from_rgba(
                    frame.width,
                    frame.height,
                    frame.data
                ));
                
                if self.quad_state.is_loading() {
                    self.quad_state.set_loaded();
                }
                
                Action::None
            }
            Message::GSTError(error) => {
                match error {
                    VideoError::Eos => {
                        eprintln!("stream ended");
                        // restart pipeline, show placeholder, etc
                        self.gstreamer_error = Some("EOS".to_string());
                    }
                    VideoError::PipelineError(msg) => {
                        eprintln!("gstreamer error: {}", msg);
                        // show error state in UI
                        self.gstreamer_error = Some(msg);
                    }
                }
                Action::None
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let camera_subscription = if self.gstreamer_error.is_none() {
            Subscription::run(gstreamer_stream).map(|result| match result {
                Ok(frame) => Message::FrameReceived(frame),
                Err(e) => Message::GSTError(e)
            })
        } else {
            Subscription::none()
        };

        Subscription::batch([
            // tick screen for updates ~120fps
            time::every(Duration::from_millis(8))
                .map(|arg0: std::time::Instant| Message::Tick(arg0.elapsed())),

            camera_subscription            
        ])
    }

    pub fn top_view(&self) -> Element<'_, Message> {
        if self.gstreamer_error.is_some() {
            text(format!("Error opening camera! Try rebooting or check with a developer.")).into()
        }
        else if let Some(handle) = &self.last_frame {
            stack![
                iced::widget::image(handle),
                QuadCanvas::new(&self.quad_state),
            ].into()
        } else {
            QuadCanvas::new(&self.quad_state)
        }
    }

    pub fn bottom_view(&self) -> Element<'_, Message> {
        let new_window_button =
            text(format!("bottom window home screen"));

        let main = column![new_window_button]
            .spacing(50)
            .width(Fill)
            .align_x(Center)
            .width(200);

        let stack = stack![
            Canvas::new(&self.grid)
                .width(Fill)
                .height(Fill),

            main,
        ];

        container(stack)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .style( |_| iced::widget::container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(1.0, 162.0 / 255.0, 0.0))),
            text_color: Some(Color::BLACK),
            border: Default::default(),
            shadow: Default::default(),
            snap: Default::default()
        })
        .into()    
    }
}