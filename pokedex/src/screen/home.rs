use iced::{
    Center, Color, Element, Event, Fill, Length, Subscription, Task, event::{self, Status}, keyboard::{Event::KeyPressed, Key, key::Named}, time
};
use iced::animation::Animation;
use iced::widget::{
    column, container, text, stack, canvas::Canvas
};

use image;

use std::time::{Duration, Instant};

use crate::elements::gstreamer_stream::{VideoError, VideoFrame, gstreamer_stream};
use crate::elements::loading_screen::{QuadCanvas, QuadState};
use crate::elements::pokedex_spinner::{SpinnerCanvas, PokedexSpinnerState};
use crate::grid::Grid;
use crate::io;


#[derive(Debug)]
pub struct Home {
    processing: bool,
    grid: Grid,
    last_frame_handle: Option<iced::widget::image::Handle>,
    last_frame: Option<VideoFrame>,
    loading: bool,
    quad_state: QuadState,
    time: f32,
    gstreamer_error: Option<String>,
    captured_frame: Option<iced::widget::image::Handle>,
    frame_save_error: Option<String>,
    classifying: bool,
    fade: Animation<f32>,
    bg_handle: iced::widget::image::Handle,
    spinner_state: PokedexSpinnerState
}

#[derive(Debug, Clone)]
pub enum Message {
    HomeToggled,
    Refresh, // TODO: use this to try restarting camera if error
    Tick(Duration),
    FrameReceived(VideoFrame),
    GSTError(VideoError),
    IOInput(IOAction),
    FrameSaveError(Option<String>),
    Classify,
    Blurred(iced::widget::image::Handle)
}

pub enum Action {
    None,
    GoHome,
    RedrawWindows,
    Run(Task<Message>),
}

#[derive(Debug, Clone)]
pub enum IOAction {
    TakePicture
}

impl Home {
    pub fn new() -> (Self, Task<Message>) {
        println!("New home created");
        (
            Self {
                processing: false,
                grid: Grid { offset: crate::grid::Vector { x: 0.0, y: 0.0 } },
                last_frame_handle: None,
                last_frame: None,
                loading: true,
                quad_state: QuadState::new(),
                time: 0.0,
                gstreamer_error: None,
                captured_frame: None,
                frame_save_error: None,
                classifying: false,
                fade: Animation::new(0.0)
                    .duration(Duration::from_millis(300))
                    .easing(iced::animation::Easing::EaseInOut),
                bg_handle: iced::widget::image::Handle::from_bytes(
                    include_bytes!("../../assets/background.png").as_slice()
                ),
                spinner_state: PokedexSpinnerState::new()
            },
            Task::none()
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
            Message::Tick(duration) => {
                self.grid.offset.x += 0.5;
                self.grid.offset.y += 0.5;

                if self.quad_state.is_loading() 
                    || self.quad_state.is_finishing() 
                    || !self.quad_state.finished_spinning() 
                    && self.gstreamer_error.is_none()
                {
                    self.quad_state.tick();
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
                self.last_frame = Some(frame.clone());

                self.last_frame_handle = Some(iced::widget::image::Handle::from_rgba(
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
                        // TODO: restart pipeline, show placeholder, etc
                        self.gstreamer_error = Some("EOS".to_string());
                    }
                    VideoError::PipelineError(msg) => {
                        eprintln!("gstreamer error: {}", msg);
                        // TODO: show error state in UI
                        self.gstreamer_error = Some(msg);
                    }
                }
                Action::None
            },
            Message::IOInput(action) => {
                match action {
                    IOAction::TakePicture => {
                        if self.classifying {
                            return Action::None
                        }
                        if let Some(frame) = self.last_frame.clone() {                           
                            return Action::Run(Task::batch([
                                Task::perform(Self::blur_image(frame.clone()), Message::Blurred),
                                Task::perform(
                                    async move {
                                        // Save image to a temp staging area while we classify it
                                        // If classification succeeds: move to appropriate folder
                                        // Else: Do nothing, staging area will be recreated on next capture
                                        
                                        io::save_frame(&frame)
                                            .map_err(|e| e.to_string())
                                    },
                                    |result| match result {
                                        Ok(()) => Message::Classify,
                                        Err(e) => Message::FrameSaveError(Some(e))
                                    },
                                )
                            ])
                        )};
                    },
                }

                Action::None
            },
            Message::FrameSaveError(err) => {
                if let Some(error) = err {
                    self.frame_save_error = Some(error);
                    eprintln!("{:?}", self.frame_save_error);
                };

                Action::None
            },
            Message::Classify => {
                self.classifying = true;
                Action::None
            },
            Message::Blurred(handle) => {
                self.captured_frame = Some(handle);

                self.fade.go_mut(1.0, Instant::now());
                self.spinner_state.set_time();

                Action::None
            }
        }
    }    

    async fn blur_image(frame: VideoFrame) -> iced::widget::image::Handle {
        let buff: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> = image::ImageBuffer::from_vec(
            frame.width, 
            frame.height, 
            frame.data.clone()
        ).unwrap();
        let blurred = image::imageops::fast_blur(&buff, 10.0);
        let pixels = blurred.into_raw();
        
        iced::widget::image::Handle::from_rgba(frame.width, frame.height, pixels)
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

            camera_subscription,

            // TODO: Will need custom subscription / event to handle rpi IO
            event::listen_with(|event, status, _| match (event, status) {
                (
                    Event::Keyboard(KeyPressed {
                        key: Key::Named(Named::Enter),
                        ..
                    }),
                    Status::Ignored,
                ) => Some(Message::IOInput(IOAction::TakePicture)),
                _ => None,
            }),
        ])
    }

    pub fn top_view(&self) -> Element<'_, Message> {
        if self.gstreamer_error.is_some() {
            text(format!("Error opening camera! Try rebooting or check with a developer.")).into()
        }
        else if self.classifying && self.captured_frame.is_some(){
            stack![
                iced::widget::image(self.captured_frame.as_ref().unwrap())
                .opacity(self.fade.interpolate_with(|v|v, Instant::now())),

                iced::widget::image(self.bg_handle.clone()),
                SpinnerCanvas::new(&self.spinner_state)
            ].into()
        }
        else if let Some(handle) = &self.last_frame_handle {
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