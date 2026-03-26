use iced::event::{self, Status};
use iced::keyboard::{Event::KeyPressed, Key, key::Named};
use iced::widget::{container, mouse_area, stack, text};
use iced::{Color, Element, Event, Subscription, Task, time};

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::elements::gstreamer_stream::{VideoError, VideoFrame, gstreamer_stream};
use crate::elements::loading_screen::{QuadCanvas, QuadState};
use crate::grid::Grid;
use crate::io;

#[derive(Debug, PartialEq)]
enum State {
    Loading, // waiting for camera
    Loaded,  // getting frames
}

impl State {
    fn should_get_frames(&self) -> bool {
        *self == State::Loading || *self == State::Loaded
    }
}

#[derive(Debug)]
pub struct Home {
    state: State,
    bottom_handle: iced::widget::image::Handle,
    bottom_pressed_handle: iced::widget::image::Handle,
    pressed: bool,
    grid: Grid,
    last_frame_handle: Option<iced::widget::image::Handle>,
    last_frame: Option<VideoFrame>,
    quad_state: QuadState,
    time: Instant,
    gstreamer_error: Option<String>,
    frame_save_error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    HomeToggled,
    Refresh, // TODO: use this to try restarting camera if error
    Tick(Duration),
    BottomPressed,
    BottomReleased,
    FrameReceived(VideoFrame),
    GSTError(VideoError),
    IOInput(IOAction),
    FrameSaveError(Option<String>),
    Register(Arc<VideoFrame>),
}

pub enum Action {
    None,
    GoHome,
    Register(Arc<VideoFrame>),
    RedrawWindows,
    Run(Task<Message>),
}

#[derive(Debug, Clone)]
pub enum IOAction {
    TakePicture,
}

impl Home {
    pub fn new(bottom_handle: iced::widget::image::Handle) -> (Self, Task<Message>) {
        println!("New home created");
        (
            Self {
                state: State::Loading,
                bottom_handle: bottom_handle,
                bottom_pressed_handle: iced::widget::image::Handle::from_bytes(
                    include_bytes!("../../assets/bottom_screen_pressed.png").as_slice(),
                ),
                pressed: false,
                grid: Grid::new(),
                last_frame_handle: None,
                last_frame: None,
                quad_state: QuadState::new(),
                time: Instant::now(),
                gstreamer_error: None,
                frame_save_error: None,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::HomeToggled => Action::None,
            Message::Refresh => Action::GoHome,
            Message::Tick(_) => {
                self.grid.tick();

                if self.state == State::Loading
                    || self.quad_state.is_finishing()
                    || !self.quad_state.finished_spinning() && self.gstreamer_error.is_none()
                {
                    self.quad_state.tick();
                }

                Action::RedrawWindows
            }
            Message::BottomPressed => {
                if self.state != State::Loading {
                    self.pressed = true;
                }
                Action::None
            }
            Message::BottomReleased => {
                self.pressed = false;
                Action::None
            }
            Message::FrameReceived(frame) => {
                // TODO move this to async task
                if self.time.elapsed() > Duration::from_secs_f32(0.5) {
                    self.last_frame = Some(frame.clone());

                    self.last_frame_handle = Some(iced::widget::image::Handle::from_rgba(
                        frame.width,
                        frame.height,
                        frame.data,
                    ));

                    if self.state == State::Loading {
                        self.state = State::Loaded;
                        self.quad_state.set_loaded();
                    }
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
            }
            Message::IOInput(action) => {
                match action {
                    IOAction::TakePicture => {
                        if !self.state.should_get_frames() {
                            return Action::None;
                        }
                        if let Some(frame) = self.last_frame.clone() {
                            let res = Arc::new(frame.clone());
                            return Action::Run(Task::batch([
                                // Task::perform(Self::blur_image(frame.clone()), Message::Blurred),
                                Task::perform(
                                    async move {
                                        // Save image to a temp staging area while we classify it
                                        // If classification succeeds: move to appropriate folder
                                        // Else: Do nothing, staging area will be recreated on next capture
                                        io::save_frame(&frame).map_err(|e| e.to_string())
                                    },
                                    move |result| match result {
                                        Ok(..) => Message::Register(Arc::clone(&res)),
                                        Err(e) => Message::FrameSaveError(Some(e)),
                                    },
                                ),
                            ]));
                        };
                    }
                }

                Action::None
            }
            Message::FrameSaveError(err) => {
                if let Some(error) = err {
                    self.frame_save_error = Some(error);
                    eprintln!("{:?}", self.frame_save_error);
                };

                Action::None
            }
            Message::Register(result) => {
                Action::Register(result)
                // Action::Run(Task::perform(
                //     async move {
                //         let mut session = model.lock().unwrap();
                //         ml::classify_image(&mut session, path.to_str().unwrap())
                //     }, |result| Message::ClassificationResult(result.map_err(|e| e.to_string())))
                // )
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let camera_subscription =
            if self.gstreamer_error.is_none() && self.state.should_get_frames() {
                Subscription::run(gstreamer_stream).map(|result| match result {
                    Ok(frame) => Message::FrameReceived(frame),
                    Err(e) => Message::GSTError(e),
                })
            } else {
                Subscription::none()
            };

        Subscription::batch([
            // tick screen for updates ~60fps
            time::every(Duration::from_millis(16))
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
            text(format!(
                "Error opening camera! Try rebooting or check with a developer."
            ))
            .into()
        } else if let Some(handle) = &self.last_frame_handle {
            stack![
                iced::widget::image(handle),
                QuadCanvas::new(&self.quad_state),
            ]
            .into()
        } else {
            QuadCanvas::new(&self.quad_state)
        }
    }

    pub fn bottom_view(&self) -> Element<'_, Message> {
        let opacity = if self.state == State::Loading {
            0.5
        } else {
            1.0
        };
        let handle = if self.pressed {
            &self.bottom_pressed_handle
        } else {
            &self.bottom_handle
        };

        stack![
            // warmup render so there's no flash while it loads the image
            iced::widget::image(&self.bottom_pressed_handle).opacity(0.0),
            mouse_area(
                container(iced::widget::image(handle).opacity(opacity)).style(|_| {
                    iced::widget::container::Style {
                        background: Some(iced::Background::Color(Color::BLACK)),
                        ..Default::default()
                    }
                })
            )
            .on_press(Message::BottomPressed)
            .on_release(Message::BottomReleased)
        ]
        .into()
    }
}
