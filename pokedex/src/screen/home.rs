use iced::{
    Center, Color, Element, Fill, Subscription, Task, Theme, time
};
use iced::widget::{
    column, container, text, stack, canvas::Canvas
};
use nokhwa::{
    nokhwa_initialize,
    pixel_format::{RgbAFormat, RgbFormat},
    query,
    utils::{ApiBackend, RequestedFormat, RequestedFormatType},
    CallbackCamera,
};

use std::fmt;
use std::ops::Sub;
use std::time::Duration;
use std::sync::{mpsc};

use crate::grid::Grid;
use crate::pipeline;

struct DebugCC(CallbackCamera);

impl fmt::Debug for DebugCC {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Debug called for CallbackCamera")
    }
}

#[derive(Debug)]
pub struct Home {
    title: String,
    processing: bool,
    grid: Grid,
    camera: Option<DebugCC>,
    rx: Option<mpsc::Receiver<Vec<u8>>>,
    last_frame: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    HomeToggled,
    Refresh,
    InitCamera,
    CameraInitialized,
    Tick,
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
        let (mut tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            pipeline::init_pipeline(tx);
        });

        (
            Self { 
                title: String::from("Home page"),
                processing: false,
                grid: Grid { offset: crate::grid::Vector { x: 0.0, y: 0.0 } },
                camera: None,
                rx: rx,
            },
            Task::done(Message::InitCamera)
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
            Message::InitCamera => Action::Run(Task::perform(
                Home::init_camera(),
                |_| {
                    Message::CameraInitialized
                }
            )),
            Message::CameraInitialized => {
                Action::None
            }
        }
    }

    async fn init_camera()  {
        // let (tx, rx) = channel();
        // std::thread::spawn(move || {
        //     pipeline::init_pipeline(tx);
        // });
        

        // let cameras = match query(ApiBackend::Video4Linux) { 
        //     Ok(cams) => cams,
        //     Err(err) => {
        //         println!("Failed to query backend {}", err);
        //         return None;
        //     }
        // };

        // let format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
        // println!("Format selected {:?}", format);

        // let first_camera = match cameras.first() {
        //     Some(cam) => cam,
        //     None => {
        //         println!("No cameras found");
        //         return None;
        //     }
        // };

        // println!("Selected first camera as {:?}", first_camera);

        // let mut threaded = match CallbackCamera::new(first_camera.index().clone(), format, |buffer| {
        //     let image = buffer.decode_image::<RgbFormat>().unwrap();
        //     println!("{}x{} {}", image.width(), image.height(), image.len());
        // }) {
        //     Ok(cam) => cam,
        //     Err(err) => {
        //         println!("Failed to create callback camera {}", err);
        //         return None;
        //     }
        // };

        // Some(DebugCC(threaded))
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            // tick screen for updates
            time::every(Duration::from_millis(16))
                .map(|_| Message::Tick),

            // get video frames
            Subscription::from(value)
        ])
        
    }

    pub fn top_view(&self) -> Element<'_, Message> {
        let window_text  = text("Top screen text from home");
        if self.camera.is_some() {
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