use iced::{
    Center, Color, Element, Fill, Subscription, Task, time
};
use iced::widget::{
    column, container, text, stack, canvas::Canvas
};

use std::time::Duration;

use crate::elements::gstreamer_recipe::{VideoFrame, gstreamer_stream};
use crate::elements::loading_screen::{QuadCanvas, QuadState};
use crate::grid::Grid;


#[derive(Debug)]
pub struct Home {
    title: String,
    processing: bool,
    grid: Grid,
    last_frame: Option<VideoFrame>,
    loading: bool,
    quad_state: QuadState,
    time: f32
}

#[derive(Debug, Clone)]
pub enum Message {
    HomeToggled,
    Refresh,
    Load,
    Tick(Duration),
    FrameReceived(VideoFrame)
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
                time: 0.0
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

                if self.quad_state.is_loading() || self.quad_state.is_finishing() || !self.quad_state.finished_spinning() {
                    self.quad_state.tick(duration.as_secs_f32());
                }
                
                self.time += duration.as_secs_f32();
                if self.time > 3.0 && !self.quad_state.is_finishing() {
                    self.quad_state.set_loaded();
                }

                Action::RedrawWindows
            }
            Message::FrameReceived(frame) => {
                self.last_frame = Some(frame);
                
                if self.quad_state.is_loading() {
                    self.quad_state.set_loaded();
                }
                
                Action::None
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            // tick screen for updates ~120fps
            time::every(Duration::from_millis(8))
                .map(|arg0: std::time::Instant| Message::Tick(arg0.elapsed())),

            // pull frames from camera
            Subscription::run(gstreamer_stream).map(Message::FrameReceived)
        ])        
    }

    pub fn top_view(&self) -> Element<'_, Message> {
        if self.loading {
            QuadCanvas::new(&self.quad_state)
        }
        else if let Some(frame) = &self.last_frame {
            let handle = iced::widget::image::Handle::from_rgba(
                frame.width,
                frame.height,
                frame.data.clone(),
            );
            iced::widget::image(handle).into()
        } else {
            let new_window_button =
                text(format!("Loading video..."));
            column![new_window_button]
                .width(Fill)
                .height(Fill)
                .align_x(Center)
                .into()     
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