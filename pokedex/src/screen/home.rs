use iced::{
    Center, Color, Element, Fill, Subscription, Task, Theme, time
};
use iced::widget::{
    column, container, text, stack, canvas::Canvas
};

use std::fmt;
use std::time::Duration;

use crate::elements::gstreamer_recipe::{VideoFrame, gstreamer_stream};
use crate::grid::Grid;
use crate::pipeline;


#[derive(Debug)]
pub struct Home {
    title: String,
    processing: bool,
    grid: Grid,
    last_frame: Option<VideoFrame>,
}

#[derive(Debug, Clone)]
pub enum Message {
    HomeToggled,
    Refresh,
    Tick,
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
                last_frame: None
            },
            Task::done(Message::Tick)
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
            Message::Tick => {
                self.grid.offset.x += 0.5;
                self.grid.offset.y += 0.5;

                Action::RedrawWindows
            }
            Message::FrameReceived(frame) => {
                self.last_frame = Some(frame);

                Action::None
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            // tick screen for updates
            time::every(Duration::from_millis(16))
                .map(|_| Message::Tick),

            Subscription::run(gstreamer_stream).map(Message::FrameReceived)
        ])
        
    }

    pub fn top_view(&self) -> Element<'_, Message> {
        let window_text  = text("Top screen text from home");
        if self.last_frame.is_some() {
            column![window_text]
                .width(Fill)
                .height(Fill)
                .align_x(Center)
                .into()     
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