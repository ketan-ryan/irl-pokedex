use anyhow::anyhow;

use iced::{Center, Color, Element, Event, Fill, Subscription, Task, time};
use iced::event::{self, Status};
use iced::keyboard::{Event::KeyPressed, Key, key::Named};
use iced::animation::Animation;
use iced::widget::{column, container, text, stack, canvas::Canvas};

use image;

use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::sync::Arc;

use crate::elements::register_pokemon::{RegisterCanvas, RegisterPokemonState};
use crate::ml;
use crate::elements::gstreamer_stream::{VideoError, VideoFrame, gstreamer_stream};
use crate::elements::loading_screen::{QuadCanvas, QuadState};
use crate::elements::pokedex_spinner::{SpinnerCanvas, PokedexSpinnerState};
use crate::grid::Grid;
use crate::io::{self, PokedexConfig};


#[derive(Debug, PartialEq)]
enum STATE {
    PROCESSING,
    LOADING,
    LOADED,
    CLASSIFYING,
    REGISTERING
}

impl STATE {
    fn should_get_frames(&self) -> bool {
        *self == STATE::LOADING || *self == STATE::LOADED
    }

    // TODO: Maybe move this to its own Screen?
    fn show_animation(&self) -> bool {
        *self == STATE::CLASSIFYING || *self == STATE::REGISTERING
    }
}

#[derive(Debug)]
pub struct Home {
    config: Arc<PokedexConfig>,
    state: STATE,
    grid: Grid,
    last_frame_handle: Option<iced::widget::image::Handle>,
    last_frame: Option<VideoFrame>,
    quad_state: QuadState,
    time: f32,
    gstreamer_error: Option<String>,
    captured_frame: Option<iced::widget::image::Handle>,
    frame_save_error: Option<String>,
    fade: Animation<f32>,
    bg_handle: iced::widget::image::Handle,
    ring_handle: iced::widget::image::Handle,
    spinner_state: PokedexSpinnerState,
    failed_classification: Option<String>,
    register_pokemon: RegisterPokemonState
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
    Classify(PathBuf),
    Blurred(iced::widget::image::Handle),
    ClassificationResult(Result<(usize, f32), String>),
    FailedClassification(Option<String>),
    Classified((iced::widget::image::Handle, f32, iced::widget::image::Handle))
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
    pub fn new(
        pokedex: Arc<PokedexConfig>
    ) -> (Self, Task<Message>) { 
        println!("New home created");
        (
            Self {
                config: pokedex,
                state: STATE::LOADING,
                grid: Grid::new(),
                last_frame_handle: None,
                last_frame: None,
                quad_state: QuadState::new(),
                time: 0.0,
                gstreamer_error: None,
                captured_frame: None,
                frame_save_error: None,
                fade: Animation::new(0.0)
                    .duration(Duration::from_millis(300))
                    .easing(iced::animation::Easing::EaseInOut),
                bg_handle: iced::widget::image::Handle::from_bytes(
                    include_bytes!("../../assets/background.png").as_slice()
                ),
                ring_handle: iced::widget::image::Handle::from_bytes(
                    include_bytes!("../../assets/ring.png").as_slice()
                ),
                spinner_state: PokedexSpinnerState::new(),
                failed_classification: None,
                register_pokemon: RegisterPokemonState::new(),
            },
            Task::none()
        )
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::HomeToggled => {
                self.state = STATE::PROCESSING;
                Action::None
            }
            Message::Refresh => {
                Action::GoHome
            }
            Message::Tick(duration) => {
                self.grid.tick();

                if self.state == STATE::LOADING 
                    || self.quad_state.is_finishing() 
                    || !self.quad_state.finished_spinning() 
                    && self.gstreamer_error.is_none()
                {
                    self.quad_state.tick();
                }

                self.time += duration.as_secs_f32();

                if self.state.show_animation() {
                    self.spinner_state.tick();
                }

                if self.state == STATE::REGISTERING && self.register_pokemon.current_full_fade() < 1.0 {
                    self.register_pokemon.tick();
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
                
                if self.state == STATE::LOADING {
                    self.state = STATE::LOADED;
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
                        if self.state.show_animation() {
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
                                        Ok(result) => Message::Classify(result),
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
            Message::Classify(path) => {
                self.state = STATE::CLASSIFYING;

                let model = Arc::clone(&self.config.session);
                Action::Run(Task::perform(
                    async move {
                        let mut session = model.lock().unwrap();
                        ml::classify_image(&mut session, path.to_str().unwrap())
                    }, |result| Message::ClassificationResult(result.map_err(|e| e.to_string())))
                )
            },
            Message::Blurred(handle) => {
                self.captured_frame = Some(handle);

                self.fade.go_mut(1.0, Instant::now());
                self.spinner_state.set_time();

                Action::None
            },
            Message::ClassificationResult(result) => {
                if result.is_err() {
                    println!("{:?}", &result);

                    return Action::Run(
                        Task::done(Message::FailedClassification(Some(result.as_ref().err().unwrap().to_string())))
                    );
                }
                println!("{:?}", result.as_ref());
                let (class_idx, conf) = result.unwrap();
                if conf < 0.5 {
                    return Action::Run(
                        Task::done(Message::FailedClassification(Some("No Pokemon detected in image.".to_string())))
                    );
                } else {
                    let cfg = self.config.clone();
                    
                    let cls = rand::random_range(0..cfg.classes.len());

                    let pokemon: Option<&String> = cfg.classes.get(cls);
                    let loc = cfg.sprites_location.clone();

                    if pokemon.is_none() {
                        // TODO error handling - missingno?
                        println!("Index {} OOB!", class_idx);
                        return Action::Run(Task::done(Message::FrameSaveError(Some("terror".to_string()))));
                    }

                    let poke = pokemon.unwrap().to_string();

                    return Action::Run(
                        Task::perform(
                            async move {
                                Self::classify(poke, loc)
                                    .map_err(|e| e.to_string())
                            }, |result| match result {
                                Ok(res) => {
                                    Message::Classified(res)
                                },
                                Err(e) => Message::FrameSaveError(Some(e))
                            },
                        )
                    );
                }
            },
            Message::FailedClassification(err) => {
                if let Some(error) = err {
                    self.failed_classification = Some(error);
                    println!("Failed classification: {:?}", self.failed_classification);
                }
                Action::None
            },
            Message::Classified(result) => {
                self.state = STATE::REGISTERING;
                self.spinner_state.start_register();
                
                self.register_pokemon.init(result.0, result.2, result.1);
                
                Action::None
            }
        }
    }

    fn classify(pokemon: String, sprite_folder: String) -> Result<(
        iced::widget::image::Handle,    // white_handle
        f32,                            // offset
        iced::widget::image::Handle     // png_handle
    ), anyhow::Error> {        
        // grab png
        let img = io::load_png(sprite_folder, &pokemon);
        if img.is_ok() {
            let white_handle = Self::make_white_mask(&img.as_ref().unwrap());
            let offset = Self::find_image_com(&img.as_ref().unwrap());
            let png_handle = iced::widget::image::Handle::from_bytes(img.unwrap());
            return Ok((white_handle, offset, png_handle));
        } else {
            return Err(anyhow!("Failed to grab sprite for {}", pokemon));
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

    /**
     * Given a PNG, loops over its pixels and returns a mask
     * where every non-tranparent pixel is full white
     */
    fn make_white_mask(bytes: &[u8]) -> iced::widget::image::Handle {
        let img = image::load_from_memory(bytes)
            .unwrap()
            .into_rgba8();

        let result = image::ImageBuffer::from_fn(img.width(), img.height(), |x, y| {
            let pixel = img.get_pixel(x, y);
            if pixel[3] > 0 {
                image::Rgba([255, 255, 255, pixel[3]])
            } else {
                image::Rgba([0, 0, 0, 0])
            }
        });

        iced::widget::image::Handle::from_rgba(result.width(), result.height(), result.into_raw())
    }

    /**
     * Finds center of mass for a png across rows and columns
     * A pixel is considered to have mass if it has a > 0 alpha
     */
    fn find_image_com(bytes: &[u8]) -> f32{
        let img = image::load_from_memory(bytes).unwrap().into_rgba8();
        let (width, height) = img.dimensions();

        // row-based CoM
        let mut row_weighted_sum = 0.0;
        let mut row_total_weight = 0.0;

        for row in 0..height {
            let mut row_mass = 0.0;
            let mut row_x_sum = 0.0;

            for col in 0..width {
                let pixel = img.get_pixel(col, row);
                if pixel[3] > 0 {
                    row_mass += 1.0;
                    row_x_sum += col as f32;
                }
            }

            if row_mass > 0.0 {
                // give more weight to dense areas
                row_weighted_sum += (row_x_sum / row_mass) * row_mass;
                row_total_weight += row_mass;
            }
        }

        let row_com = (row_weighted_sum / row_total_weight) / width as f32;

        // col-based CoM
        let mut col_weighted_sum = 0.0;
        let mut col_total_weight = 0.0;

        for col in 0..width {
            let mut col_mass = 0.0;

            for row in 0..height {
                let pixel = img.get_pixel(col, row);
                if pixel[3] > 0 {
                    col_mass += 1.0;
                }
            }

            if col_mass > 0.0 {
                col_weighted_sum += (col as f32 / width as f32) * col_mass;
                col_total_weight += col_mass;
            }
        }

        let col_com = col_weighted_sum / col_total_weight;

        (row_com + col_com) / 2.0
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let camera_subscription = if self.gstreamer_error.is_none() && self.state.should_get_frames() {
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
        else if self.state.show_animation() && self.captured_frame.is_some() {
            let mut elements: Vec<Element<Message>> = vec![
                iced::widget::image(self.captured_frame.as_ref().unwrap())
                    .opacity(self.fade.interpolate_with(|v|v, Instant::now()))
                    .into(),

                iced::widget::image(self.ring_handle.clone())
                    .scale(self.spinner_state.current_register_scale())
                    .opacity(1.2 - self.spinner_state.current_register_scale())
                    .into(),

                iced::widget::image(self.bg_handle.clone())
                    .scale(self.spinner_state.current_scale())
                    .into(),
                SpinnerCanvas::new(&self.spinner_state),
            ];
            if self.register_pokemon.offset.is_some() {
                elements.push(
                    RegisterCanvas::new(&self.register_pokemon)
                );
            }
            iced::widget::Stack::with_children(elements).into()

                // cutout: centered 8px strip of the blurred bg drawn over everything
                // TODO: test clipping when iced updates with tinyskia fix
                // iced::widget::container(
                //     iced::widget::container(
                //         iced::widget::image(self.captured_frame.as_ref().unwrap())
                //             .content_fit(iced::ContentFit::Cover)
                //             .width(iced::Fill)
                //             .height(iced::Fill)
                //     )
                //     .width(iced::Fill)
                //     .height(40)
                //     .clip(true)
                // )
                // .width(iced::Fill)
                // .height(iced::Fill)
                // .align_y(iced::Center)

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