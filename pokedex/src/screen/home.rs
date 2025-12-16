use iced::{
    Color, Element, Task, Center, Fill, Subscription, time
};
use iced::widget::{
    column, container, text, stack, canvas::Canvas
};
use iced_video_player::{Video, VideoPlayer};
use nokhwa::{
    nokhwa_initialize,
    pixel_format::{RgbAFormat, RgbFormat},
    query,
    utils::{ApiBackend, RequestedFormat, RequestedFormatType},
    CallbackCamera,
};

use std::fmt;
use std::time::Duration;
use std::sync::Arc;

use crate::grid::Grid;

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
    video: Option<Video>,
    camera: Option<DebugCC>,
}

#[derive(Debug, Clone)]
pub enum Message {
    HomeToggled,
    Refresh,
    InitVideo,
    VideoInitialized(Arc<Video>),
    InitCamera,
    CameraInitialized(Arc<DebugCC>),
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
        (
            Self { 
                title: String::from("Home page"),
                processing: false,
                grid: Grid { offset: crate::grid::Vector { x: 0.0, y: 0.0 } },
                video: None,
                camera: None,
            },
            Task::done(Message::InitVideo)
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
            Message::InitVideo => Action::Run(Task::perform(
                Home::init_video(),
             Message::VideoInitialized
            )),
            Message::VideoInitialized(video) => {
                self.video = match Arc::try_unwrap(video) {
                    Ok(vid) => Some(vid),
                    Err(_) => {
                        println!("Could not unwrap, ref count is > 1");
                        None
                    }
                };
                Action::None
            }
            Message::InitCamera => Action::Run(Task::perform(
                Home::init_camera(),
                |arg0: std::option::Option<DebugCC>| {
                    let cam = arg0.expect("Failed to open camera!");
                    Message::CameraInitialized(Arc::from(cam))
                }
            )),
            Message::CameraInitialized(camera) => {
                self.camera = match Arc::try_unwrap(camera) {
                    Ok(cam) => Some(cam),
                    Err(_) => {
                        println!("Couldn't unwrap camera ref");
                        None
                    }
                };

                Action::None
            }
        }
    }

    async fn init_video() -> Arc<Video> {
        let uri = match url::Url::parse("file:///C:/Users/kyure/Videos/Easter Egg/mowzies easter egg.mp4") {
            Ok(success) => success,
            Err(error) => {
                println!("{}", error);
                panic!("Failed to parse url");
            }
        };
        Arc::new(Video::new(&uri).unwrap())
    }

    async fn init_camera() -> Option<DebugCC> {
        let cameras = match query(ApiBackend::Auto) { 
            Ok(cams) => cams,
            Err(err) => {
                println!("Failed to query backend {}", err);
                return None;
            }
        };

        let format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);

        let first_camera = match cameras.first() {
            Some(cam) => cam,
            None => {
                println!("No cameras found");
                return None;
            }
        };

        let mut threaded = match CallbackCamera::new(first_camera.index().clone(), format, |buffer| {
            let image = buffer.decode_image::<RgbFormat>().unwrap();
            println!("{}x{} {}", image.width(), image.height(), image.len());
        }) {
            Ok(cam) => cam,
            Err(err) => {
                println!("Failed to create callback camera {}", err);
                return None;
            }
        };

        Some(DebugCC(threaded))
    }

    pub fn subscription(&self) -> Subscription<Message> {
        time::every(Duration::from_millis(16))
            .map(|_| Message::Tick)
    }

    pub fn top_view(&self) -> Element<'_, Message> {
        // let window_text:  = text("Top screen text from home");
        if self.video.is_some() {
            let vid = VideoPlayer::new(&self.video.as_ref().unwrap());
            column![vid]
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

            main
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