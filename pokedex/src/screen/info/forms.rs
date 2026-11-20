use iced::{
    Border, Color, Element, Font, Length, Task, Theme, alignment,
    font::Weight,
    widget::{Canvas, Image, Row, Space, column, container, stack, svg, text},
};
use log::warn;

use crate::{
    elements::{
        common::{self, CommonAssets, action_row, primary_button, secondary_button},
        icon_button::{IconButtonColors, IconButtonInteraction, IconButtonSize, icon_button},
    },
    enums::{Gender, IOAction, PokemonEntry, PokemonInfo},
    screen::info::info::{self, Info},
};

#[derive(Debug)]
pub struct Forms {
    pub return_to: Option<Box<Info>>,

    current_entry: PokemonEntry,
    current_info: PokemonInfo,

    shiny: bool,

    arrow_icon: svg::Handle,
    close_icon: svg::Handle,
    missingno: iced::widget::image::Handle,

    male_icon: svg::Handle,
    female_icon: svg::Handle,
    monomorphic_icon: svg::Handle,
    genderless_icon: svg::Handle,
    shiny_icon: svg::Handle,

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

    InfoBtn,
    RelationsBtn,
    FormBtn,
    CryBtn,

    Info(info::Message),
}

pub enum Action {
    None,
    Run(Task<Message>),
    Return(Box<Info>, Task<crate::info::Message>),
    OpenRelations,
}

impl Forms {
    pub fn new(return_to: Box<Info>) -> (Self, Task<Message>) {
        let current = return_to.get_current();
        let info = return_to.get_pokemon_info(Some(current.clone().key));
        (
            Self {
                return_to: Some(return_to),
                current_entry: current,
                current_info: info,
                shiny: false,

                arrow_icon: svg::Handle::from_memory(
                    include_bytes!("../../../assets/info_screen/arrow.svg").as_slice(),
                ),
                close_icon: svg::Handle::from_memory(
                    include_bytes!("../../../assets/x.svg").as_slice(),
                ),

                missingno: iced::widget::image::Handle::from_bytes(
                    include_bytes!("../../../assets/missingno.png").as_slice(),
                ),

                shiny_icon: svg::Handle::from_memory(
                    include_bytes!("../../../assets/forms_screen/shiny.svg").as_slice(),
                ),
                male_icon: svg::Handle::from_memory(
                    include_bytes!("../../../assets/forms_screen/male.svg").as_slice(),
                ),
                female_icon: svg::Handle::from_memory(
                    include_bytes!("../../../assets/forms_screen/female.svg").as_slice(),
                ),
                genderless_icon: svg::Handle::from_memory(
                    include_bytes!("../../../assets/forms_screen/genderless.svg").as_slice(),
                ),
                monomorphic_icon: svg::Handle::from_memory(
                    include_bytes!("../../../assets/forms_screen/monomorphic.svg").as_slice(),
                ),

                up_interaction: IconButtonInteraction::default(),
                down_interaction: IconButtonInteraction::default(),
                close_interaction: IconButtonInteraction::default(),
            },
            Task::none(),
        )
    }

    fn step(&mut self, delta: isize) -> Action {
        let Some(info) = self.return_to.as_mut() else {
            warn!("No info to return to from step!");
            return Action::None;
        };
        let step_task = info.step(delta).map(Message::Info);
        // self.refresh();
        // let sprite_task = self.load_graph_sprites();
        Action::Run(Task::batch([
            step_task,
            // sprite_task
        ]))
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
                        Some(info) => Action::Return(info, Task::none()),
                        None => Action::None,
                    }
                } else {
                    self.close_interaction = i;
                    Action::None
                }
            }
            Message::Info(msg) => match self.return_to.as_mut().unwrap().update(msg) {
                info::Action::Run(task) => Action::Run(task.map(Message::Info)),
                _ => Action::None,
            },
            Message::InfoBtn => match self.return_to.take() {
                Some(info) => Action::Return(info, Task::none()),
                None => Action::None,
            },
            Message::RelationsBtn => Action::OpenRelations,
            Message::FormBtn => Action::None,
            Message::CryBtn => Action::None,
        }
    }

    pub fn top_view<'a>(&'a self, common: &'a CommonAssets) -> Element<'a, Message> {
        let background = container(
            Canvas::new(&common.scanlines)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| iced::widget::container::Style {
            background: Some(iced::Background::Color(common::colors::BACKGROUND)),
            ..Default::default()
        });

        let mut font = Font::with_name("Open Sans SemiBold");
        font.weight = Weight::Semibold;

        let header = container(
            column![
                text!("Pokémon Info")
                    .size(24.0)
                    .color(common::colors::TEXT_DARK)
                    .font(font),
                text!("Forms view")
                    .size(16.0)
                    .font(common::CONDENSED)
                    .color(common::colors::TEXT_LIGHT)
            ]
            .spacing(5.0)
            .padding(10.0),
        )
        .width(Length::Fill)
        .height(Length::Fixed(80.0))
        .style(|_| iced::widget::container::Style {
            background: Some(iced::Background::Color(common::colors::CARD_BG)),
            border: Border {
                radius: 10.0.into(),
                width: 1.0,
                color: common::colors::TEXT_DARK,
            },
            shadow: common::default_shadow(),
            ..Default::default()
        });

        let info = self.return_to.as_ref().unwrap();
        let image = info.get_sprite(&self.current_entry.key);
        let handle = match image {
            Some(h) => h,
            None => self.missingno.clone(),
        };
        let sprite: Element<_> = container(Image::new(handle).width(Length::Fill))
            .width(Length::Fixed(320.0))
            .into();

        stack![
            background,
            column![container(header).padding(20.0)],
            container(sprite)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Bottom)
                .padding(8)
        ]
        .into()
    }

    pub fn bottom_view<'a>(&'a self, common: &'a CommonAssets) -> Element<'a, Message> {
        // let info_focused = is_focused_at(focus, section::BUTTONS, 0, 2);

        let icon = |handle: &svg::Handle| -> Element<'a, Message> {
            svg(handle.clone())
                .width(Length::Fixed(48.0))
                .height(Length::Fixed(48.0))
                .into()
        };

        let gender_icons = match self.current_info.gender {
            Gender::Both => vec![icon(&self.monomorphic_icon)],
            Gender::Genderless => vec![icon(&self.genderless_icon)],
            _ => vec![icon(&self.female_icon), icon(&self.male_icon)],
        };

        let icon_row = Row::with_children(gender_icons)
            .push(icon(&self.shiny_icon))
            .spacing(12)
            .padding(20.0);

        let chain_background = container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .style(|_: &Theme| container::Style {
                border: iced::Border {
                    color: Color::WHITE,
                    width: 2.0,
                    radius: 20.0.into(),
                },
                background: Some(iced::Background::Color(common::colors::CARD_BORDER)),
                ..Default::default()
            });

        // let up_focused = is_focused_at(focus, section::BUTTONS, 0, 0);
        // let down_focused = is_focused_at(focus, section::BUTTONS, 0, 1);
        // let info_focused = is_focused_at(focus, section::BUTTONS, 0, 2);
        // let relations_focused = is_focused_at(focus, section::BUTTONS, 0, 3);
        // let forms_focused = is_focused_at(focus, section::BUTTONS, 0, 4);
        // let cry_focused = is_focused_at(focus, section::BUTTONS, 0, 5);
        // let close_focused = is_focused_at(focus, section::BUTTONS, 0, 6);
        let size = IconButtonSize::large();
        let colors = IconButtonColors {
            default_shadow: common::default_shadow(),
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
            false,
        );
        let down = icon_button(
            self.arrow_icon.clone(),
            size,
            None,
            &self.down_interaction,
            colors,
            Message::DownInteraction,
            false,
        );
        let close = icon_button(
            self.close_icon.clone(),
            size,
            None,
            &self.close_interaction,
            colors,
            Message::CloseInteraction,
            false,
        );
        let info_button =
            secondary_button("Info".to_string(), Message::InfoBtn, None, false, false);
        let relations_button = secondary_button(
            "Relations".to_string(),
            Message::RelationsBtn,
            None,
            false,
            false,
        );
        let forms_button = primary_button("Forms".to_string(), Message::FormBtn, false);
        let cry_button = secondary_button("Cry".to_string(), Message::CryBtn, None, false, false);
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
                    .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(common::colors::BACKGROUND)),
                ..Default::default()
            }),
            column![icon_row, chain_background, row].padding(8)
        ]
        .into()
    }
}
