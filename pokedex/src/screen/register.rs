use anyhow::anyhow;

use iced::{Border, Color, Element, Subscription, Task, Theme, Vector, time};
use iced::animation::Animation;
use iced::widget::{button, container};

use image;

use std::time::{Duration, Instant};
use std::sync::Arc;

use crate::elements::gstreamer_stream::VideoFrame;
use crate::elements::register_pokemon::{RegisterCanvas, RegisterPokemonState};
use crate::ml;
use crate::elements::pokedex_spinner::{SpinnerCanvas, PokedexSpinnerState};
use crate::io::{self, PokedexConfig};


#[derive(Debug, PartialEq)]
enum State {
    Registering,        // playing pokedex registration anim
    Registered,         // dex reg anim over
    FailedRegistering,  // show error screen
    Classifying         // running inference
}

impl State {
    fn registering_or_later(&self) -> bool {
        *self == State::Registered || *self == State::Registering || *self == State::FailedRegistering
    }
}

#[derive(Debug)]
pub struct Register {
    config: Arc<PokedexConfig>,
    state: State,
    fade: Animation<f32>,
    captured_frame: Option<iced::widget::image::Handle>,
    blurred_frame: Option<iced::widget::image::Handle>,
    bottom_handle: iced::widget::image::Handle,
    bg_handle: iced::widget::image::Handle,
    ring_handle: iced::widget::image::Handle,
    spinner_state: PokedexSpinnerState,
    failed_classification: Option<String>,
    register_pokemon: RegisterPokemonState
}


#[derive(Debug, Clone)]
pub enum Message {
    Start(Arc<VideoFrame>),
    HomeToggled,
    Tick(Duration),
    Classify(Arc<VideoFrame>),
    Blurred(iced::widget::image::Handle),
    ClassificationResult(Result<(usize, f32), String>),
    FailedClassification(Option<String>),
    Classified((iced::widget::image::Handle, f32, iced::widget::image::Handle))
}

pub enum Action {
    None,
    GoHome,
    Run(Task<Message>),
}

impl Register {
    pub fn new(
        pokedex: Arc<PokedexConfig>,
        frame: Arc<VideoFrame>
    ) -> (Self, Task<Message>) {
        println!("Open register");
        (
            Self {
                state: State::Classifying,
                config: pokedex,
                captured_frame: None,
                blurred_frame: None,
                bottom_handle: iced::widget::image::Handle::from_bytes(
                    include_bytes!("../../assets/bottom_screen.png").as_slice(),
                ),
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
            Task::done(Message::Start(frame))
        )
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::HomeToggled => Action::GoHome,
            Message::Start(frame) => {
                self.captured_frame = Some(
                    iced::widget::image::Handle::from_rgba(frame.width, frame.height, frame.data.clone())
                );
                self.spinner_state.set_time();

                Action::Run(Task::batch([
                        Task::perform(blur_image(Arc::clone(&frame)),Message::Blurred),
                        Task::done(Message::Classify(frame))
                    ])
                )
            },
            Message::Tick(duration) => {       
                self.spinner_state.tick();

                if self.state == State::Registering && self.register_pokemon.current_full_fade() < 1.0 {
                    self.register_pokemon.tick();
                }

                Action::None
            }
            Message::Classify(frame) => {
                self.state = State::Classifying;

                let model = Arc::clone(&self.config.session);
                Action::Run(Task::perform(
                    async move {
                        let mut session = model.lock().unwrap();
                        ml::classify_image(&mut session, frame)
                    }, |result| Message::ClassificationResult(result.map_err(|e| e.to_string())))
                )
            },
            Message::Blurred(handle) => {
                self.blurred_frame = Some(handle);

                self.fade.go_mut(1.0, Instant::now());

                println!("Blurring complete");

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
                if conf < 0.05 {
                    return Action::Run(
                        Task::done(Message::FailedClassification(Some("No Pokemon detected in image.".to_string())))
                    );
                } else {
                    let cfg = self.config.clone();
                    let pokemon: Option<&String> = cfg.classes.get(class_idx);
                    let loc = cfg.sprites_location.clone();

                    if pokemon.is_none() {
                        println!("Index {} OOB!", class_idx);
                        return Action::Run(Task::done(Message::FailedClassification(Some(
                            "Pokemon index out of bounds - likely an issue with the class list.".to_string()))));
                    }

                    let poke = pokemon.unwrap().to_string();

                    return Action::Run(
                        Task::perform(
                            async move {
                                Self::classify(poke, loc, false)
                                    .map_err(|e| e.to_string())
                            }, |result| match result {
                                Ok(res) => {
                                    Message::Classified(res)
                                },
                                Err(e) => Message::FailedClassification(Some(e))
                            },
                        )
                    );
                }
            },
            Message::FailedClassification(err) => {
                // if we are here, err is Some
                self.failed_classification = err;
                self.state = State::FailedRegistering;

                let cfg = self.config.clone();
                let loc = cfg.sprites_location.clone();

                Action::Run(
                    Task::perform(
                        async move {
                            Self::classify("missingno".to_string(), loc, true)
                                .map_err(|e| e.to_string())
                        }, |result| match result {
                            Ok(res) => {
                                Message::Classified(res)
                            },
                            // TODO: handle this differently to avoid recursing
                            Err(e) => {Message::FailedClassification(Some(e))}
                        },
                    )
                )
            },
            Message::Classified(result) => {
                self.state = State::Registering;
                self.spinner_state.start_register();
                
                self.register_pokemon.init(result.0, result.2, result.1);
                
                Action::None
            }
        }
    }

    fn classify(pokemon: String, sprite_folder: String, is_error: bool) -> Result<(
        iced::widget::image::Handle,    // white_handle
        f32,                            // offset
        iced::widget::image::Handle     // png_handle
    ), anyhow::Error> {        
        // grab png
        let img: Result<Vec<u8>, anyhow::Error> = if is_error {
            Ok(include_bytes!("../../assets/missingno.png").to_vec())
        } else {
            io::load_png(sprite_folder, &pokemon)
        };
        if img.is_ok() {
            let white_handle = Self::make_white_mask(&img.as_ref().unwrap());
            let offset = Self::find_image_com(&img.as_ref().unwrap());
            let png_handle = iced::widget::image::Handle::from_bytes(img.unwrap());
            return Ok((white_handle, offset, png_handle));
        } else {
            return Err(anyhow!("Failed to grab sprite for {}", pokemon));
        }
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
        // tick screen for updates ~60fps
        time::every(Duration::from_millis(16))
            .map(|arg0: std::time::Instant| Message::Tick(arg0.elapsed()))
    }

    pub fn top_view(&self) -> Element<'_, Message> {
        // const FONT: iced::Font = iced::Font::with_name("Open Sans Light");

        // items in the vec are drawn in order
        // items drawn first will be on the bottom
        let mut elements: Vec<Element<Message>> = vec![];

        if self.captured_frame.is_some() && self.blurred_frame.is_none(){
            elements = vec![
                // captured image, un-blurred
                iced::widget::image(self.captured_frame.as_ref().unwrap())
                    .into(),

                // spinner 
                iced::widget::image(&self.bg_handle)
                    .scale(self.spinner_state.current_scale())
                    .into(),

                SpinnerCanvas::new(&self.spinner_state),
            ];
        }
        else if self.blurred_frame.is_some() {
            elements = vec![
                // captured image, un-blurred
                iced::widget::image(self.captured_frame.as_ref().unwrap())
                    .into(),

                // fade in blurred image
                iced::widget::image(self.blurred_frame.as_ref().unwrap())
                    .opacity(self.fade.interpolate_with(|v|v, Instant::now()))
                    .into(),

                // still show spinner
                iced::widget::image(&self.bg_handle)
                    .scale(self.spinner_state.current_scale())
                    .into(),
                SpinnerCanvas::new(&self.spinner_state),
            ];

            if self.state == State::Registering {
                // ring pulses in
                elements.push(
                    iced::widget::image(&self.ring_handle)
                        .scale(self.spinner_state.current_register_scale())
                        .opacity(1.2 - self.spinner_state.current_register_scale())
                        .into(),
                );
            }
            if self.state.registering_or_later() {
                // show pokemon sprite
                elements.push(
                    RegisterCanvas::new(&self.register_pokemon)
                );
            }
        }
        iced::widget::Stack::with_children(elements).into()
    }

    pub fn bottom_view(&self) -> Element<'_, Message> {
        container(iced::widget::image(&self.bottom_handle).opacity(0.5)).style(|_| {
            iced::widget::container::Style {
                background: Some(iced::Background::Color(Color::BLACK)),
                ..Default::default()
            }
        })
        .into()  
    }
}

async fn blur_image(frame: Arc<VideoFrame>) -> iced::widget::image::Handle {
    let buff: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> = image::ImageBuffer::from_vec(
        frame.width, 
        frame.height, 
        frame.data.clone()
    ).unwrap();
    let blurred = image::imageops::fast_blur(&buff, 10.0);
    let pixels = blurred.into_raw();
    
    iced::widget::image::Handle::from_rgba(frame.width, frame.height, pixels)
}

fn custom_button_style(theme: &Theme, status: button::Status) -> button::Style {

    // Define style based on state (e.g., pressed, hovered)
    match status {
        button::Status::Active | button::Status::Pressed => button::Style {
            background: Some(Color::from_rgba(0.2, 0.2, 0.2, 0.6).into()),
            border: Border { color: Color::WHITE, width: 1.0, radius: 5.0.into() },
            text_color: Color::WHITE,
            ..Default::default()
        },
        button::Status::Hovered => button::Style {
            background: Some(Color::from_rgba(0.0, 0.3, 1.0, 0.6).into()),
            shadow: iced::Shadow {
                color: Color::from_rgba(0.0, 0.5, 0.8, 0.4),
                offset: Vector::new(0.0, 0.0),
                blur_radius: 8.0
            },
            ..custom_button_style(theme, button::Status::Active) // Reuse active
        },
        _ => button::Style {
            background: Some(Color::from_rgba(0.05, 0.05, 0.05, 0.6).into()),
            border: Border { color: Color::BLACK, width: 1.0, radius: 5.0.into() },
            text_color: Color::WHITE,
            ..Default::default()
        },
    }
}
