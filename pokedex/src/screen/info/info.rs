use std::collections::{HashMap, HashSet};

use iced::{
    Alignment, Background, Color, Element, Length, Padding, Task, Theme, Vector,
    widget::{Canvas, Space, canvas, column, container, row, stack, svg, text},
};
use log::debug;

use crate::{
    elements::{
        common::{
            CONDENSED, CommonAssets, colors::CARD_BG, default_shadow, holo_header_backdrop_with,
            primary_button, secondary_button,
        },
        icon_button::{IconButtonColors, IconButtonInteraction, IconButtonSize, icon_button},
        message_box::Metrics,
        modal::{modal, shrink_text_to_fit},
        registered_icon::{IconState, RegisteredIconWidget},
    },
    enums::{IOAction, PokemonEntry, PokemonInfo},
    io,
    screen::{
        browse_pokedex::{self, browse_pokedex::PokedexBrowser},
        register::register::to_proper_case,
    },
};

#[derive(Debug)]
pub struct Info {
    return_to: Option<Box<PokedexBrowser>>,
    current_pokemon: PokemonEntry,
    list: Vec<PokemonEntry>,
    pokemon_data: HashMap<String, PokemonInfo>,
    owned_pokemon: HashSet<String>,

    arrow_icon: svg::Handle,
    close_icon: svg::Handle,

    up_interaction: IconButtonInteraction,
    down_interaction: IconButtonInteraction,
    close_interaction: IconButtonInteraction,

    dex_entry_size: f32,
    dex_entry_text: String,
    name_size: f32,
    abilities_size: f32,

    type_images: Vec<iced::widget::image::Handle>,
    info_modal: svg::Handle,
}

#[derive(Clone, Debug)]
pub enum Message {
    IOInput(IOAction),
    UpInteraction(IconButtonInteraction),
    DownInteraction(IconButtonInteraction),
    CloseInteraction(IconButtonInteraction),
    Info,
    Relations,
    Form,
    Cry,
    Browser(browse_pokedex::browse_pokedex::Message),
}

pub enum Action {
    None,
    Run(Task<Message>),
    Return(Box<PokedexBrowser>, Task<crate::browse_pokedex::Message>),
}

impl Info {
    fn update_info(&mut self) {
        let current_info = self.pokemon_data.get(&self.current_pokemon.key).unwrap();
        let random_index = rand::random_range(0..current_info.dex_entries.len());
        let dex_entry = current_info
            .dex_entries
            .iter()
            .nth(random_index)
            .map(|(_, v)| v.clone());

        let failure_text = format!(
            "Failed to find information for Pokémon {:?}!",
            &self.current_pokemon.key
        );

        self.dex_entry_text = dex_entry.unwrap_or(failure_text);

        let avail = Metrics::content_size(iced::Size::new(620.0, 95.0), Metrics::REFERENCE);
        self.dex_entry_size = shrink_text_to_fit(
            &self.dex_entry_text,
            22.0,
            avail.width,
            10.0,
            3,
            avail.height,
        );

        debug!("{} {}", self.dex_entry_size, self.dex_entry_text);
        self.name_size =
            shrink_text_to_fit(&self.current_pokemon.display, 16.0, 154.0, 10.0, 1, 28.0);
        let abilities = current_info.abilities.join(", ");
        self.abilities_size = shrink_text_to_fit(&abilities, 16.0, 200.0, 10.0, 1, 28.0);

        self.type_images = io::resolve_type_images(current_info.types.clone());
    }

    pub fn new(
        return_to: Box<PokedexBrowser>,
        current_pokemon: PokemonEntry,
        list: Vec<PokemonEntry>,
        pokemon_data: HashMap<String, PokemonInfo>,
        owned: HashSet<String>,
    ) -> (Self, Task<Message>) {
        debug!("Info opened with pokemon {:?}", current_pokemon,);

        let current_info = pokemon_data.get(&current_pokemon.key).unwrap();
        let random_index = rand::random_range(0..current_info.dex_entries.len());
        let dex_entry = current_info
            .dex_entries
            .iter()
            .nth(random_index)
            .map(|(_, v)| v.clone());

        let failure_text = format!(
            "Failed to find information for Pokémon {:?}!",
            &current_pokemon.key
        );

        let dex_entry_text = dex_entry.unwrap_or(failure_text);

        let avail = Metrics::content_size(iced::Size::new(620.0, 95.0), Metrics::REFERENCE);
        let dex_entry_size =
            shrink_text_to_fit(&dex_entry_text, 22.0, avail.width, 10.0, 3, avail.height);

        debug!("{} {}", dex_entry_size, dex_entry_text);
        let name_size = shrink_text_to_fit(&current_pokemon.display, 16.0, 154.0, 10.0, 1, 28.0);
        let abilities = current_info.abilities.join(", ");
        let abilities_size = shrink_text_to_fit(&abilities, 16.0, 200.0, 10.0, 1, 28.0);

        let type_images = io::resolve_type_images(current_info.types.clone());

        (
            Self {
                return_to: Some(return_to),
                current_pokemon,
                list,
                pokemon_data,
                owned_pokemon: owned,

                arrow_icon: svg::Handle::from_memory(
                    include_bytes!("../../../assets/info_screen/arrow.svg").as_slice(),
                ),
                close_icon: svg::Handle::from_memory(
                    include_bytes!("../../../assets/x.svg").as_slice(),
                ),
                info_modal: svg::Handle::from_memory(
                    include_bytes!("../../../assets/info_screen/info_modal.svg").as_slice(),
                ),

                up_interaction: IconButtonInteraction::default(),
                down_interaction: IconButtonInteraction::default(),
                close_interaction: IconButtonInteraction::default(),

                dex_entry_size,
                dex_entry_text: dex_entry_text.to_string(),
                name_size,
                abilities_size,

                type_images,
            },
            Task::none(),
        )
    }

    fn step(&mut self, delta: isize) -> Action {
        let len = self.list.len() as isize;
        let Some(index) = self.list.iter().position(|e| *e == self.current_pokemon) else {
            return Action::None;
        };
        let new_index = (index as isize + delta).rem_euclid(len) as usize;
        self.current_pokemon = self.list[new_index].clone();
        self.update_info();

        match self.return_to.as_mut() {
            Some(browser) => {
                let task = browser.load_around_entry(&self.current_pokemon);
                Action::Run(task.map(Message::Browser))
            }
            None => Action::None,
        }
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::IOInput(input) => match input {
                _ => Action::None,
            },
            Message::UpInteraction(i) => {
                if i == IconButtonInteraction::Released {
                    self.up_interaction = IconButtonInteraction::Hovered;
                    return self.step(1);
                }
                self.up_interaction = i;
                Action::None
            }
            Message::DownInteraction(i) => {
                if i == IconButtonInteraction::Released {
                    self.down_interaction = IconButtonInteraction::Hovered;
                    return self.step(-1);
                }
                self.down_interaction = i;
                Action::None
            }
            Message::CloseInteraction(i) => {
                if i == IconButtonInteraction::Released {
                    self.close_interaction = IconButtonInteraction::Hovered;
                    match self.return_to.take() {
                        Some(mut browser) => {
                            let task = browser.jump_to_entry(self.current_pokemon.clone());
                            Action::Return(browser, task)
                        }
                        None => Action::None,
                    }
                } else {
                    self.close_interaction = i;
                    Action::None
                }
            }
            Message::Info => Action::None,
            Message::Relations => Action::None,
            Message::Form => Action::None,
            Message::Cry => Action::None,
            Message::Browser(m) => match self.return_to.as_mut() {
                Some(browser) => {
                    let task = browser.update_background(m);
                    Action::Run(task.map(Message::Browser))
                }
                None => Action::None,
            },
        }
    }

    pub fn top_view<'a>(&'a self, common: &'a CommonAssets) -> Element<'a, Message> {
        const FONT_SIZE: f32 = 16.0;

        // get pokedex entry. by this point, we know it exists, so unwrap
        let desc_text = text(self.dex_entry_text.clone())
            .size(self.dex_entry_size)
            .font(CONDENSED)
            .width(iced::Fill)
            .color(Color::BLACK);

        let description_overlay = container(modal(
            None,
            container(
                desc_text
                    .font(CONDENSED)
                    .color(Color::BLACK)
                    .align_x(iced::alignment::Horizontal::Left),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(iced::Alignment::Center)
            .into(),
            vec![],
            600.0,
            110.0,
            Metrics {
                inset: Vector::new(15.0, 7.0),
                ..Metrics::REFERENCE
            },
            None,
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::Center)
        .align_y(iced::Bottom)
        .padding(iced::Padding {
            bottom: 20.0,
            ..iced::Padding::ZERO
        });

        let mut contents = column![
            Space::new()
                .width(Length::Fill)
                .height(Length::FillPortion(1)),
        ]
        .spacing(1)
        .padding(5);

        // sprite
        let image = self
            .return_to
            .as_ref()
            .and_then(|b| b.image_cache.get(&self.current_pokemon.key));

        let thumb: Element<_> = match image {
            Some(h) => iced::widget::Image::new(h)
                .width(Length::Fill)
                .scale(0.85)
                .into(),
            None => Space::new().into(),
        };

        // ============info box=================
        let info = self.pokemon_data.get(&self.current_pokemon.key).unwrap();

        // Owned indicator
        let is_owned = self.owned_pokemon.contains(&self.current_pokemon.key);
        let state = if is_owned {
            IconState::Registered
        } else {
            IconState::Unregistered
        };
        let shape = RegisteredIconWidget::new(state, 1.0);
        let item_container = container(
            canvas::Canvas::new(shape)
                .width(Length::Fixed(20.0))
                .height(Length::Fixed(20.0)),
        );

        // name and number
        let number = info.number.clone();
        let pokemon_name = to_proper_case(&self.current_pokemon.display);

        let top_section = container(
            row![
                item_container,
                // Number is always going to be 4 digits, give it a fixed width
                container(
                    text(number)
                        .font(CONDENSED)
                        .size(FONT_SIZE)
                        .color(Color::BLACK)
                )
                .width(Length::Fixed(60.0))
                .align_x(iced::Alignment::Start),
                // Take all remaining space and centers the text
                container(
                    text(pokemon_name)
                        .font(CONDENSED)
                        .color(Color::BLACK)
                        .size(self.name_size) // Disallow names with spaces from overflowing to next line
                        .wrapping(iced::widget::text::Wrapping::None)
                )
                .width(Length::Fill)
                .align_y(iced::alignment::Vertical::Top)
                .align_x(iced::alignment::Horizontal::Left),
                // Duplicate the number container on the right for visual balance but keep it empty
                Space::new().width(Length::Fixed(60.0)),
            ]
            .spacing(10.0)
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
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(CARD_BG)),
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
                    .font(CONDENSED)
                    .color(Color::BLACK),
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
        let height = info.height.raw.clone();
        let weight = info.weight.raw.clone();
        let abilities = info.abilities.join(", ");

        // Distance from a line box's bottom edge to its baseline, as a fraction of font size.
        // Depends on the font's metrics and line height; ~0.25 for CONDENSED at the default
        // 1.3 line height is a starting guess, so tune it by eye.
        const BASELINE_RATIO: f32 = 0.15;

        let max_size = FONT_SIZE.max(self.abilities_size);
        let lift = |size: f32| (max_size - size) * BASELINE_RATIO;

        let bottom_section = column![
            // height row
            container(
                container(
                    row![
                        text("HT")
                            .size(FONT_SIZE)
                            .font(CONDENSED)
                            .color(Color::BLACK),
                        Space::new().width(Length::Fill),
                        text(height)
                            .size(FONT_SIZE)
                            .font(CONDENSED)
                            .color(Color::BLACK),
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
                    background: Some(Background::Color(CARD_BG)),
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
                        text("WT")
                            .size(FONT_SIZE)
                            .font(CONDENSED)
                            .color(Color::BLACK),
                        Space::new().width(Length::Fill),
                        text(weight)
                            .size(FONT_SIZE)
                            .font(CONDENSED)
                            .color(Color::BLACK),
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
                    background: Some(Background::Color(CARD_BG)),
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
            // abilities row
            container(
                container(
                    row![
                        container(
                            text("AB")
                                .size(FONT_SIZE)
                                .font(CONDENSED)
                                .color(Color::BLACK)
                        )
                        .padding(Padding {
                            bottom: lift(FONT_SIZE),
                            ..Default::default()
                        }),
                        Space::new().width(Length::Fill),
                        container(
                            text(abilities)
                                .size(self.abilities_size)
                                .font(CONDENSED)
                                .color(Color::BLACK)
                        )
                        .padding(Padding {
                            top: lift(self.abilities_size),
                            ..Default::default()
                        }),
                    ]
                    .align_y(Alignment::End),
                )
                .width(Length::Fill)
                .padding(Padding {
                    top: -5.0,
                    bottom: 6.0,
                    ..iced::Padding::ZERO
                }),
            )
            .width(Length::Fill)
            .height(Length::Fixed(8.0))
            .align_y(Alignment::Start),
        ]
        .spacing(12)
        .width(Length::Fill)
        .height(Length::Fill)
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

        contents = contents.push(
            row![
                container(thumb)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center),
                container(modal(
                    None,
                    modal_body.into(),
                    vec![],
                    250.0,
                    185.0,
                    Metrics {
                        margin: Vector::new(0.0, 0.0),
                        ..Metrics::REFERENCE
                    },
                    None
                ))
                .padding(iced::Padding {
                    top: 70.0,
                    ..iced::Padding::ZERO
                })
            ]
            .height(Length::Fixed(400.0))
            .spacing(70.0)
            .padding(iced::Padding {
                bottom: 120.0,
                right: 20.0,
                ..iced::Padding::ZERO
            }),
        );

        stack![
            holo_header_backdrop_with(common, &self.info_modal, "Pokémon Info", ""),
            contents,
            description_overlay
        ]
        .into()
    }

    pub fn bottom_view<'a>(&'a self, common: &'a CommonAssets) -> Element<'a, Message> {
        let size = IconButtonSize::large();
        let colors = IconButtonColors {
            default_shadow: default_shadow(),
            ..Default::default()
        };
        let up = icon_button(
            self.arrow_icon.clone(),
            IconButtonSize {
                rotation: Some(iced::Rotation::Floating(iced::Radians::PI)),
                ..IconButtonSize::large()
            },
            None,
            &self.up_interaction,
            colors,
            Message::UpInteraction,
        );
        let down = icon_button(
            self.arrow_icon.clone(),
            size,
            None,
            &self.down_interaction,
            colors,
            Message::DownInteraction,
        );
        let close = icon_button(
            self.close_icon.clone(),
            size,
            None,
            &self.close_interaction,
            colors,
            Message::CloseInteraction,
        );
        let info_button = primary_button("Info".to_string(), Message::Info, false);
        let relations_button =
            secondary_button("Relations".to_string(), Message::Relations, None, false);
        let forms_button = secondary_button("Forms".to_string(), Message::Form, None, false);
        let cry_button = secondary_button("Cry".to_string(), Message::Cry, None, false);
        let elements = vec![
            up,
            down,
            info_button,
            relations_button,
            forms_button,
            cry_button,
            close.into(),
        ];
        let row = action_row(elements);
        stack![
            container(
                Canvas::new(&common.scanlines)
                    .width(Length::Fill)
                    .height(Length::Fill)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(140, 213, 229))),
                ..Default::default()
            }),
            column![Space::new().height(Length::FillPortion(2)), row,]
                .width(Length::Fill)
                .height(Length::Fill)
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

fn action_row<'a>(elements: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    row::Row::with_children(elements)
        .width(Length::Fill)
        .height(Length::Fixed(80.0))
        .spacing(15)
        .padding(7)
        .align_y(Alignment::Center)
        .into()
}
