use std::collections::HashMap;

use iced::{
    Alignment, Color, Element, Length, Task,
    widget::{Canvas, Space, column, container, row, stack, svg},
};
use log::debug;

use crate::{
    elements::{
        common::{
            CommonAssets, default_shadow, holo_header_backdrop, primary_button, secondary_button,
        },
        icon_button::{IconButtonColors, IconButtonInteraction, IconButtonSize, icon_button},
    },
    enums::{IOAction, PokemonEntry, PokemonInfo},
    screen::browse_pokedex::browse_pokedex::PokedexBrowser,
};

#[derive(Debug)]
pub struct Info {
    return_to: Option<Box<PokedexBrowser>>,
    current_pokemon: PokemonEntry,
    list: Vec<PokemonEntry>,
    pokemon_data: HashMap<String, PokemonInfo>,

    arrow_icon: svg::Handle,
    close_icon: svg::Handle,

    up_interaction: IconButtonInteraction,
    down_interaction: IconButtonInteraction,
    close_interaction: IconButtonInteraction,
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
}

pub enum Action {
    None,
    Run(Task<Message>),
    Return(Box<PokedexBrowser>, Task<crate::browse_pokedex::Message>),
}

impl Info {
    pub fn new(
        return_to: Box<PokedexBrowser>,
        current_pokemon: PokemonEntry,
        list: Vec<PokemonEntry>,
        pokemon_data: HashMap<String, PokemonInfo>,
    ) -> (Self, Task<Message>) {
        debug!(
            "Info opened with pokemon {:?} and list {:?} and data {:?}",
            current_pokemon,
            list,
            pokemon_data.get(&current_pokemon.key)
        );
        (
            Self {
                return_to: Some(return_to),
                current_pokemon,
                list,
                pokemon_data,

                arrow_icon: svg::Handle::from_memory(
                    include_bytes!("../../../assets/info_screen/arrow.svg").as_slice(),
                ),
                close_icon: svg::Handle::from_memory(
                    include_bytes!("../../../assets/x.svg").as_slice(),
                ),

                up_interaction: IconButtonInteraction::default(),
                down_interaction: IconButtonInteraction::default(),
                close_interaction: IconButtonInteraction::default(),
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::IOInput(input) => match input {
                _ => Action::None,
            },
            Message::UpInteraction(i) => {
                if i == IconButtonInteraction::Released {
                    self.up_interaction = IconButtonInteraction::Hovered;
                    return Action::None;
                } else {
                    self.up_interaction = i;
                }
                Action::None
            }
            Message::DownInteraction(i) => {
                if i == IconButtonInteraction::Released {
                    self.down_interaction = IconButtonInteraction::Hovered;
                    return Action::None;
                } else {
                    self.down_interaction = i;
                }
                Action::None
            }
            Message::CloseInteraction(i) => {
                if i == IconButtonInteraction::Released {
                    self.close_interaction = IconButtonInteraction::Hovered;
                    match self.return_to.take() {
                        Some(mut browser) => Action::Return(browser, Task::none()),
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
        }
    }

    pub fn top_view<'a>(&'a self, common: &'a CommonAssets) -> Element<'a, Message> {
        holo_header_backdrop(common, "National Pokédex", "Info Mode")
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
