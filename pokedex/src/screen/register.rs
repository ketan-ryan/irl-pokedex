use anyhow::anyhow;

use iced::Length::{self, Fill};
use iced::animation::Animation;
use iced::widget::{Space, button, column, container, row, stack, text};
use iced::{Alignment, Color, Element, Subscription, Task, time};
use iced_gif::Gif;

use iced::{Background, ContentFit, Padding};

use image;

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::elements::gstreamer_stream::VideoFrame;
use crate::elements::modal::{adaptive_text, modal, shrink_text_to_fit};
use crate::elements::pokedex_spinner::{PokedexSpinnerState, SpinnerCanvas};
use crate::elements::pokemon_details::PokemonDetailsState;
use crate::elements::register_pokemon::{RegisterCanvas, RegisterPokemonState};
use crate::io::{self, PokedexConfig, PokemonType};
use crate::ml;

#[derive(Debug, PartialEq)]
enum State {
    Registering,       // blurring complete
    Registered,        // playing pokedex registration anim
    FailedRegistering, // show error screen
    Classifying,       // running inference
    ReadingEntry,      // reading the dex entry - show detailed view
}

impl State {
    fn registering(&self) -> bool {
        *self == State::Registered
            || *self == State::Registering
            || *self == State::FailedRegistering
    }

    fn transitioning(&self) -> bool {
        *self == State::Registered
            || *self == State::FailedRegistering
            || *self == State::ReadingEntry
    }
}

#[derive(Debug)]
pub struct Register {
    config: Arc<PokedexConfig>,
    state: State,
    unown_handle: iced_gif::Frames,
    fade: Animation<f32>,
    captured_frame: Option<iced::widget::image::Handle>,
    blurred_frame: Option<iced::widget::image::Handle>,
    bottom_handle: iced::widget::image::Handle,
    bg_handle: iced::widget::image::Handle,
    ring_handle: iced::widget::image::Handle,
    spinner_state: PokedexSpinnerState,
    failed_classification: Option<String>,
    register_pokemon: RegisterPokemonState,
    reading_timer: Duration,
    failed_anim: Animation<f32>,
    pokemon_details: PokemonDetailsState,
    dex_entry_size: f32,
    found_pokemon: Option<String>,
    type_images: Vec<iced::widget::image::Handle>,
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
    Classified(ClassificationResults),
    ReadEntry,
    NoiseReady(Option<iced::widget::image::Handle>),
    Quantized(Vec<[f64; 3]>),
}

pub enum Action {
    None,
    GoHome,
    Run(Task<Message>),
}

#[derive(Clone, Debug)]
pub struct ClassificationResults {
    white_handle: iced::widget::image::Handle,
    png_handle: iced::widget::image::Handle,
    offset: f32,
    img_bytes: Vec<u8>,
}

const FONT: iced::Font = iced::Font::with_name("Open Sans Condensed");

impl Register {
    pub fn new(
        pokedex: Arc<PokedexConfig>,
        frame: Arc<VideoFrame>,
        bottom_handle: iced::widget::image::Handle,
    ) -> (Self, Task<Message>) {
        println!("Open register");
        (
            Self {
                state: State::Classifying,
                config: pokedex,
                captured_frame: None,
                blurred_frame: None,
                bottom_handle: bottom_handle,
                fade: Animation::new(0.0)
                    .duration(Duration::from_millis(300))
                    .easing(iced::animation::Easing::EaseInOut),
                bg_handle: iced::widget::image::Handle::from_bytes(
                    include_bytes!("../../assets/background.png").as_slice(),
                ),
                ring_handle: iced::widget::image::Handle::from_bytes(
                    include_bytes!("../../assets/ring.png").as_slice(),
                ),
                unown_handle: iced_gif::Frames::from_bytes(
                    include_bytes!("../../assets/unown-interrogation.gif").to_vec(),
                )
                .unwrap(),
                spinner_state: PokedexSpinnerState::new(),
                failed_classification: None,
                failed_anim: Animation::new(0.0).duration(Duration::from_millis(200)),
                register_pokemon: RegisterPokemonState::new(),
                reading_timer: Duration::from_millis(0),
                pokemon_details: PokemonDetailsState::new(),
                dex_entry_size: 16.0,
                found_pokemon: None,
                type_images: vec![],
            },
            Task::done(Message::Start(frame)),
        )
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::HomeToggled => Action::GoHome,
            Message::Start(frame) => {
                self.state = State::Classifying;
                self.captured_frame = Some(iced::widget::image::Handle::from_rgba(
                    frame.width,
                    frame.height,
                    frame.data.clone(),
                ));
                self.spinner_state.set_time();

                Action::Run(Task::batch([
                    Task::perform(blur_image(Arc::clone(&frame)), Message::Blurred),
                    Task::done(Message::Classify(frame)),
                ]))
            }
            Message::Tick(duration) => {
                self.spinner_state.tick();

                if self.state.transitioning() {
                    if self.register_pokemon.current_full_fade() < 1.0 {
                        self.register_pokemon.tick();
                    }
                    if self.spinner_state.current_register_scale() > 0.0
                        && self.state == State::FailedRegistering
                        && self.failed_anim.interpolate_with(|v| v, Instant::now()) == 0.0
                    {
                        self.failed_anim.go_mut(1.0, Instant::now());
                    }

                    if self.state == State::Registered {
                        self.reading_timer += duration;
                    }

                    // TODO: Replace with time of audio clip
                    // ex: "Pikachu, the electric mouse pokemon"
                    if self.state != State::ReadingEntry
                        && self.reading_timer > Duration::from_millis(1000)
                    {
                        return Action::Run(Task::done(Message::ReadEntry));
                    }
                }

                if self.state == State::ReadingEntry {
                    let mut details = self.pokemon_details.clone();
                    return Action::Run(Task::perform(async move { details.tick() }, |handle| {
                        Message::NoiseReady(handle.clone())
                    }));
                }

                Action::None
            }
            Message::Classify(frame) => {
                let model = Arc::clone(&self.config.session);
                Action::Run(Task::perform(
                    async move {
                        let mut session = model.lock().unwrap();
                        ml::classify_image(&mut session, frame)
                    },
                    |result| Message::ClassificationResult(result.map_err(|e| e.to_string())),
                ))
            }
            Message::Blurred(handle) => {
                self.blurred_frame = Some(handle);
                self.state = State::Registering;

                self.fade.go_mut(1.0, Instant::now());
                Action::None
            }
            Message::ClassificationResult(result) => {
                if result.is_err() {
                    println!("{:?}", &result);

                    return Action::Run(Task::done(Message::FailedClassification(Some(
                        result.as_ref().err().unwrap().to_string(),
                    ))));
                }
                println!("{:?}", result.as_ref());
                let (class_idx, conf) = result.unwrap();
                if conf < self.config.confidence {
                    return Action::Run(Task::done(Message::FailedClassification(Some(
                        "No Pokémon detected in image.".to_string(),
                    ))));
                } else {
                    let cfg = self.config.clone();
                    // TODO: change to class_idx
                    let class_idx = rand::random_range(0..1136);
                    let pokemon: Option<&String> = cfg.classes.get(class_idx);

                    if pokemon.is_none() {
                        println!("Index {} OOB!", class_idx);
                        return Action::Run(Task::done(Message::FailedClassification(Some(
                            "Pokémon index out of bounds - likely an issue with the class list."
                                .to_string(),
                        ))));
                    }

                    self.found_pokemon = pokemon.cloned();

                    let loc = cfg.sprites_location.clone();
                    let dex_entry = cfg.pokedex_json.get(pokemon.unwrap()).cloned();

                    let current_dex = self
                        .pokemon_details
                        .set_current_pokemon(cfg.pokedex_json.get(pokemon.unwrap()).cloned());

                    let failure_text = format!(
                        "Failed to find information for Pokémon {}!",
                        pokemon.unwrap_or(&"Unknown".to_string())
                    );

                    // Precalculate width of text once
                    self.dex_entry_size = shrink_text_to_fit(
                        &current_dex.unwrap_or(failure_text),
                        16.0,
                        300.0,
                        8.0,
                        8,
                        100.0,
                    );

                    println!("{}", self.dex_entry_size);

                    let poke = pokemon.unwrap().to_string();
                    let type_images = io::get_type_images(
                        self.config
                            .pokedex_json
                            .get(&poke)
                            .cloned()
                            .unwrap_or_default()
                            .types
                            .clone(),
                    );

                    self.type_images = type_images
                        .iter()
                        .map(|path| {
                            iced::widget::image::Handle::from_bytes(
                                std::fs::read(path).unwrap_or_else(|_| {
                                    panic!("Failed to read type image at path: {}", path)
                                }),
                            )
                        })
                        .collect();

                    return Action::Run(Task::perform(
                        async move { Self::classify(poke, loc, false).map_err(|e| e.to_string()) },
                        |result| match result {
                            Ok(res) => Message::Classified(res),
                            Err(e) => Message::FailedClassification(Some(e)),
                        },
                    ));
                }
            }
            Message::FailedClassification(err) => {
                let is_error_set = self.failed_classification.is_some();
                // if we are here, err is Some
                self.failed_classification = err;
                self.state = State::FailedRegistering;

                let cfg = self.config.clone();
                let loc = cfg.sprites_location.clone();

                if !is_error_set {
                    return Action::Run(Task::perform(
                        async move {
                            Self::classify("missingno".to_string(), loc, true)
                                .map_err(|e| e.to_string())
                        },
                        |result| match result {
                            Ok(res) => Message::Classified(res),
                            Err(e) => Message::FailedClassification(Some(e)),
                        },
                    ));
                }
                Action::None
            }
            Message::Classified(result) => {
                self.spinner_state.start_register();
                self.register_pokemon
                    .init(result.white_handle, result.png_handle, result.offset);

                if self.state != State::FailedRegistering {
                    self.state = State::Registered;
                    return Action::Run(Task::perform(
                        async move { PokemonDetailsState::quantize(&result.img_bytes) },
                        |res| Message::Quantized(res),
                    ));
                }

                Action::None
            }
            Message::ReadEntry => {
                self.state = State::ReadingEntry;

                self.fade.go_mut(0.0, Instant::now());
                self.spinner_state.end_register();
                self.register_pokemon.fade_out();

                Action::None
            }
            Message::NoiseReady(handle) => {
                self.pokemon_details.update_noise_handle(handle);

                Action::None
            }
            Message::Quantized(colors) => {
                println!("Quantized to {} buckets", &colors.len());
                self.pokemon_details.set_palette(colors);

                Action::None
            }
        }
    }

    /// Given a pokemon's name, this
    /// * Finds and loads the pokemon's sprite via [load_png](`crate::io::load_png`)
    /// * Generates a mask for the sprite where every non-transparent
    /// pixel is white via [make_white_mask](`Self::make_white_mask`)
    /// * Finds the sprite's center of mass via [find_image_com](`Self::find_image_com`)
    ///
    /// # Arguments
    /// * `pokemon` The pokemon's name, as a string. Must match an entry in the [classes dict](`crate::io::PokedexConfig::classes`)
    /// * `sprite_folder` The directory where all Pokemon sprites are stored. Retrieved from the [config](`crate::io::PokedexConfig::sprites_location`)
    /// * `is_error` Whether to show a Missingno sprite indicating an error with classification.
    ///
    /// # Returns
    ///
    /// A [`ClassificationResults`] struct containing:
    /// * `white_handle` An iced [handle](iced::widget::image::Handle) to the all-white mask
    /// * `offset` The x-coord of the image's center of mass
    /// * `png_handle` An iced [handle](iced::widget::image::Handle) to the pokemon sprite
    /// * `img_bytes` The raw image bytes
    ///
    /// # Errors
    ///
    /// Returns [`anyhow::Error`] if anything fails
    fn classify(
        pokemon: String,
        sprite_folder: String,
        is_error: bool,
    ) -> Result<ClassificationResults, anyhow::Error> {
        // grab png
        let img: Result<Vec<u8>, anyhow::Error> = if is_error {
            Ok(include_bytes!("../../assets/missingno.png").to_vec())
        } else {
            io::load_png(sprite_folder, &pokemon)
        };
        if let Ok(img) = img {
            let white_handle = Self::make_white_mask(&img);
            let offset = Self::find_image_com(&img);
            let img_bytes = img.clone();
            let png_handle = iced::widget::image::Handle::from_bytes(img);
            return Ok(ClassificationResults {
                white_handle,
                offset,
                png_handle,
                img_bytes,
            });
        } else {
            return Err(anyhow!("Failed to grab sprite for {}", pokemon));
        }
    }

    /// Given a PNG, loops over its pixels and returns a mask
    /// where every non-tranparent pixel is full white
    ///
    /// # Arguments
    /// * `bytes` the raw bytes making up the rgba8 image
    ///
    /// # Returns
    /// An [`iced::widget::image::Handle`] to the newly created mask
    fn make_white_mask(bytes: &[u8]) -> iced::widget::image::Handle {
        let img = image::load_from_memory(bytes).unwrap().into_rgba8();

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

    /// Finds center of mass for a png across its rows and columns
    ///
    /// A pixel is considered to have mass if its alpha is > 0
    ///
    /// # Arguments
    ///
    /// * `bytes` the raw bytes making up the rgba8 image
    ///
    /// # Returns
    /// The x-position of the image's center of mass
    fn find_image_com(bytes: &[u8]) -> f32 {
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
        // items in the vec are drawn in order
        // items drawn first will be on the bottom
        let mut elements: Vec<Element<Message>> = vec![];

        if self.captured_frame.is_some() && self.blurred_frame.is_none() {
            elements = vec![
                // captured image, un-blurred
                iced::widget::image(self.captured_frame.as_ref().unwrap()).into(),
                // spinner
                iced::widget::image(&self.bg_handle)
                    .scale(self.spinner_state.current_scale())
                    .into(),
                SpinnerCanvas::new(&self.spinner_state),
            ];
        } else if self.state.registering() {
            elements = vec![
                // captured image, un-blurred
                iced::widget::image(self.captured_frame.as_ref().unwrap()).into(),
                // fade in blurred image
                iced::widget::image(self.blurred_frame.as_ref().unwrap())
                    .opacity(self.fade.interpolate_with(|v| v, Instant::now()))
                    .into(),
                // still show spinner
                iced::widget::image(&self.bg_handle)
                    .scale(self.spinner_state.current_scale())
                    .into(),
                SpinnerCanvas::new(&self.spinner_state),
            ];

            if self.state == State::Registered || self.state == State::FailedRegistering {
                // ring pulses in
                elements.push(
                    iced::widget::image(&self.ring_handle)
                        .scale(self.spinner_state.current_register_scale())
                        .opacity(1.2 - self.spinner_state.current_register_scale())
                        .into(),
                );
                // show pokemon sprite
                elements.push(RegisterCanvas::new(&self.register_pokemon));
            }
        } else if self.state == State::ReadingEntry {
            elements = vec![
                // captured image, un-blurred
                iced::widget::image(self.captured_frame.as_ref().unwrap()).into(),
                // fade out blurred image, spinner, sprite
                iced::widget::image(self.blurred_frame.as_ref().unwrap())
                    .opacity(self.fade.interpolate_with(|v| v, Instant::now()))
                    .into(),
                iced::widget::image(&self.bg_handle)
                    .scale(self.spinner_state.current_scale())
                    .opacity(self.fade.interpolate_with(|v| v, Instant::now()))
                    .into(),
                RegisterCanvas::new(&self.register_pokemon),
                SpinnerCanvas::new(&self.spinner_state),
            ];
        } else {
            println!("{:?}", self.state);
        }
        iced::widget::Stack::with_children(elements).into()
    }

    pub fn bottom_view(&self) -> Element<'_, Message> {
        let opacity = if self.state != State::ReadingEntry {
            0.5
        } else {
            self.fade.interpolate_with(|v| v, Instant::now()) - 0.5
        };

        let mut elements: Vec<Element<Message>> = vec![
            container(iced::widget::image(&self.bottom_handle).opacity(opacity))
                .style(|_| iced::widget::container::Style {
                    background: Some(iced::Background::Color(Color::BLACK)),
                    ..Default::default()
                })
                .into(),
        ];

        if self.state == State::FailedRegistering {
            //TODO: Investigate stutter
            let dex_but = button("Go to Pokédex")
                .padding(10)
                .on_press(Message::HomeToggled);

            let ret_but = button("Retry Photo")
                .padding(10)
                .on_press(Message::HomeToggled);

            let t = self.failed_classification.as_ref().unwrap();

            let modal_width = self.failed_anim.interpolate_with(|v| v, Instant::now()) * 600.0;
            if modal_width > 0.0 {
                elements.push(
                    container(modal(
                        Some("Pokédex Registration Failed".to_string()),
                        row![
                            container(Gif::new(&self.unown_handle))
                                .width(120)
                                .height(120)
                                .padding(iced::Padding {
                                    top: 30.0,
                                    bottom: 6.0,
                                    left: 16.0,
                                    right: 16.0
                                }),
                            text(t)
                                .size(10)
                                .width(iced::Fill)
                                .align_x(iced::alignment::Horizontal::Right),
                        ]
                        .spacing(12)
                        .align_y(iced::Center)
                        .into(),
                        vec![dex_but, ret_but],
                        modal_width,
                        220.0,
                        None,
                    ))
                    .center(Length::Fill)
                    .into(),
                );
            }
        } else if self.state == State::ReadingEntry {
            if self.pokemon_details.noise_image.is_some() {
                // Background
                elements.push(
                    iced::widget::image(self.pokemon_details.noise_image.as_ref().unwrap())
                        .width(Fill)
                        .height(Fill)
                        .content_fit(iced::ContentFit::Cover)
                        .into(),
                );

                // sprite
                let pokemon_image =
                    iced::widget::image(self.register_pokemon.png_handle.as_ref().unwrap().clone())
                        .width(Length::Fixed(300.0))
                        .height(Length::Fixed(400.0))
                        .content_fit(ContentFit::Contain);

                // info modal
                // background color
                let light_blue = Color::from_rgba8(199, 243, 255, 0.7);
                const FONT_SIZE: f32 = 16.0;

                let info = self
                    .pokemon_details
                    .current_pokemon()
                    .cloned()
                    .unwrap_or_default();

                // name and number
                let number = info.number.clone();
                let pokemon_name = capitalize_first_letter(
                    self.found_pokemon.as_ref().unwrap_or(&"???".to_string()),
                );

                let top_section = container(
                    row![
                        // Number is always going to be 4 digits, give it a fixed width
                        container(text(number).font(FONT).size(FONT_SIZE))
                            .width(Length::Fixed(60.0))
                            .align_x(iced::Alignment::Start),
                        // Take all remaining space and centers the text
                        container(
                            text(pokemon_name)
                                .font(FONT)
                                .size(FONT_SIZE)
                                // Disallow names with spaces from overflowing to next line
                                .wrapping(iced::widget::text::Wrapping::None)
                        )
                        .width(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Center),
                        // Duplicate the number container on the right for visual balance but keep it empty
                        Space::new().width(Length::Fixed(60.0)),
                    ]
                    .align_y(iced::alignment::Vertical::Center)
                    .padding(iced::Padding::new(0.0).top(10.0)),
                )
                .width(Length::Fill)
                .height(Length::Fixed(35.0))
                .padding(iced::Padding {
                    top: 0.0,
                    left: 6.0,
                    right: 6.0,
                    bottom: 0.0,
                })
                .style(move |_| container::Style {
                    background: Some(Background::Color(light_blue)),
                    ..Default::default()
                });

                // species and type images
                let species = info.species.clone();

                // Create images from handles, spacing them apart if necessary
                let image_row = iced::widget::Row::with_children(
                    self.type_images
                        .iter()
                        .map(|handle| {
                            iced::widget::image(handle.clone())
                                .width(Length::Fixed(48.0))
                                .height(Length::Fixed(48.0))
                                .into()
                        })
                        .collect::<Vec<Element<Message>>>(),
                )
                .spacing(10)
                .align_y(iced::Alignment::Center);

                let middle_section = column![
                    container(
                        text(species)
                            .size(FONT_SIZE)
                            .width(Length::Fill)
                            .align_x(iced::alignment::Horizontal::Left)
                            .font(FONT),
                    )
                    .width(Length::Fill)
                    .padding(4),
                    container(image_row)
                        .width(Length::Fill)
                        .align_x(Alignment::Center),
                ]
                .spacing(8)
                .align_x(Alignment::Center)
                .width(Length::Fill)
                .height(Length::Fixed(58.0));

                // height and weight
                let height = info.height.clone();
                let weight = info.weight.clone();
                let bottom_section = column![
                    // height row
                    container(
                        container(
                            row![
                                text("HT").size(FONT_SIZE).font(FONT),
                                Space::new().width(Length::Fill),
                                text(height).size(FONT_SIZE).font(FONT),
                            ]
                            .align_y(Alignment::Center),
                        )
                        .width(Length::Fill)
                        .padding(Padding {
                            top: -5.0,
                            bottom: 2.0,
                            left: 0.0,
                            right: 0.0,
                        })
                        .style(move |_| container::Style {
                            background: Some(Background::Color(light_blue)),
                            border: iced::Border {
                                color: Color::TRANSPARENT,
                                width: 0.0,
                                radius: 7.0.into(),
                            },
                            ..Default::default()
                        }),
                    )
                    .width(Length::Fill)
                    .height(Length::Fixed(8.0))
                    .align_y(Alignment::Center),
                    // weight row
                    container(
                        container(
                            row![
                                text("WT").size(FONT_SIZE).font(FONT),
                                Space::new().width(Length::Fill),
                                text(weight).size(FONT_SIZE).font(FONT),
                            ]
                            .align_y(Alignment::Start),
                        )
                        .width(Length::Fill)
                        .padding(Padding {
                            top: -5.0,
                            bottom: 0.0,
                            left: 0.0,
                            right: 0.0,
                        })
                        .style(move |_| container::Style {
                            background: Some(Background::Color(light_blue)),
                            border: iced::Border {
                                color: Color::TRANSPARENT,
                                width: 0.0,
                                radius: 7.0.into(),
                            },
                            ..Default::default()
                        }),
                    )
                    .width(Length::Fill)
                    .height(Length::Fixed(8.0))
                    .align_y(Alignment::Start),
                ]
                .spacing(12)
                .width(Length::Fill)
                .height(Length::Fixed(52.0))
                .padding(iced::Padding {
                    top: 16.0,
                    left: 6.0,
                    right: 6.0,
                    bottom: 0.0,
                });

                let modal_body = column![top_section, middle_section, bottom_section]
                    .spacing(8)
                    .width(Length::Fill)
                    .height(Length::Fill);

                let info_with_ball = row![
                    iced::widget::image(self.captured_frame.as_ref().unwrap().clone())
                        .width(Length::Fixed(36.0))
                        .height(Length::Fixed(36.0)),
                    modal(
                        None,
                        modal_body.into(),
                        vec![],
                        250.0,
                        175.0,
                        Some(iced::Padding {
                            top: 4.0,
                            bottom: 0.0,
                            left: 14.5,
                            right: 14.5,
                        })
                    )
                ]
                .spacing(8)
                .align_y(Alignment::Start);

                let right_column = column![info_with_ball,]
                    .spacing(12)
                    .width(Length::Fill)
                    .align_x(Alignment::End);

                // place sprite next to info modal
                let main_content = container(
                    row![pokemon_image, right_column,]
                        .spacing(16)
                        .padding(iced::Padding {
                            top: 20.0,
                            bottom: 120.0,
                            left: 20.0,
                            right: 12.0,
                        })
                        .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_| container::Style {
                    background: None,
                    ..Default::default()
                });

                // navbar
                const BOTTOM_BAR_HEIGHT: f32 = 46.0;
                const BOTTOM_BAR_MARGIN: f32 = 20.0;

                let bottom_bar = container(
                    row![
                        button("✕").on_press(Message::HomeToggled),
                        Space::new().width(iced::Fill),
                        button("←").on_press(Message::HomeToggled),
                        button("→").on_press(Message::HomeToggled),
                    ]
                    .padding(Padding::from([8, 16]))
                    .align_y(Alignment::Center)
                    .width(Length::Fill),
                )
                .width(Length::Fill)
                .height(Length::Fixed(BOTTOM_BAR_HEIGHT))
                .style(|_| container::Style {
                    background: Some(Background::Color(Color::from_rgb(0.25, 0.25, 0.28))),
                    ..Default::default()
                });

                // get pokedex entry
                let pokedex_string = self
                    .pokemon_details
                    .current_pokedex()
                    .cloned()
                    .unwrap_or_else(|| {
                        "Error: Unable to retrieve details for this Pokémon".to_string()
                    });

                // pokedex description modal
                let description_text = if pokedex_string.starts_with("Error") {
                    text(pokedex_string).size(FONT_SIZE).width(iced::Fill)
                } else {
                    // magic numbers determined via trial and error
                    text(pokedex_string)
                        .size(self.dex_entry_size)
                        .font(FONT)
                        .width(iced::Fill)
                };

                let description_overlay = container(modal(
                    None,
                    row![
                        description_text
                            .font(FONT)
                            .align_x(iced::alignment::Horizontal::Left),
                    ]
                    .spacing(12)
                    .align_y(iced::Alignment::Center)
                    .into(),
                    vec![],
                    420.0,
                    110.0,
                    None,
                ))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::Right)
                .align_y(iced::Bottom)
                .padding(Padding {
                    top: 300.0,
                    bottom: BOTTOM_BAR_HEIGHT + BOTTOM_BAR_MARGIN,
                    left: 300.0,
                    right: 5.0,
                });

                let content =
                    stack![column![main_content, bottom_bar,], description_overlay].into();

                elements.push(content)
            } else {
                println!("Still waiting")
            }
            // elements.push(
            //     DetailsCanvas::new(&self.pokemon_details)
            //     // column![
            //     //     // main content area
            //     //     row![
            //     //         // left - sprite, full column height
            //     //         container(
            //     //             Gif::new(&self.unown_handle)
            //     //                 .width(iced::Fill)
            //     //                 .height(iced::Fill)
            //     //         )
            //     //         .width(iced::FillPortion(1))
            //     //         .height(iced::Fill),
            //     //         // middle - empty top, dex entry bottom
            //     //         column![
            //     //             Space::new().height(iced::FillPortion(2)),
            //     //             container(text("&self.dex_text"))
            //     //                 .width(iced::Fill)
            //     //                 .height(iced::FillPortion(1))
            //     //                 .padding(12)
            //     //                 .style(|_| container::Style {
            //     //                     border: iced::Border {
            //     //                         color: Color::BLACK,
            //     //                         width: 2.0,
            //     //                         radius: 4.0.into(),
            //     //                     },
            //     //                     background: Some(iced::Background::Color(Color::WHITE)),
            //     //                     ..Default::default()
            //     //                 }),
            //     //         ]
            //     //         .width(iced::FillPortion(1))
            //     //         .height(iced::Fill),
            //     //         // right - info box centered vertically
            //     //         container(
            //     //             container(text("&self.info_text"))
            //     //                 .width(iced::Fill)
            //     //                 .padding(12)
            //     //                 .style(|_| container::Style {
            //     //                     border: iced::Border {
            //     //                         color: Color::BLACK,
            //     //                         width: 2.0,
            //     //                         radius: 4.0.into(),
            //     //                     },
            //     //                     background: Some(iced::Background::Color(Color::WHITE)),
            //     //                     ..Default::default()
            //     //                 })
            //     //         )
            //     //         .width(iced::FillPortion(1))
            //     //         .height(iced::Fill)
            //     //         .align_y(iced::Center),
            //     //     ]
            //     //     .height(iced::FillPortion(6)),
            //     //     // bottom bar - half height
            //     //     container(
            //     //         row![
            //     //             button("X").on_press(Message::HomeToggled),
            //     //             Space::new().width(iced::Fill),
            //     //             button("A").on_press(Message::HomeToggled),
            //     //             button("B").on_press(Message::HomeToggled),
            //     //         ]
            //     //         .spacing(8)
            //     //         .padding(8)
            //     //         .align_y(iced::Center)
            //     //     )
            //     //     .width(iced::Fill)
            //     //     .height(iced::FillPortion(1))
            //     //     .style(|_| container::Style {
            //     //         background: Some(iced::Background::Color(Color::from_rgb(
            //     //             0.15, 0.15, 0.15
            //     //         ))),
            //     //         ..Default::default()
            //     //     }),
            //     // ]
            //     // .height(iced::Fill)
            //     // .width(iced::Fill)
            //     // .into(),
            // );
        }
        iced::widget::Stack::with_children(elements).into()
    }
}

fn capitalize_first_letter(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Blurs an image using [fast_blur](image::imageops::fast_blur)
///
/// # Arguments
///
/// * `frame` An Arc containing the image the user captured as a [VideoFrame](crate::elements::gstreamer_stream::VideoFrame)
///
/// # Returns
///
/// * An iced [handle](iced::widget::image::Handle) to the blurred image
async fn blur_image(frame: Arc<VideoFrame>) -> iced::widget::image::Handle {
    let buff: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
        image::ImageBuffer::from_vec(frame.width, frame.height, frame.data.clone()).unwrap();
    let blurred = image::imageops::fast_blur(&buff, 10.0);
    let pixels = blurred.into_raw();

    iced::widget::image::Handle::from_rgba(frame.width, frame.height, pixels)
}
