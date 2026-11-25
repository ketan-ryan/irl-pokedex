use std::collections::{HashMap, HashSet};

use iced::{
    Alignment::Center,
    Border, Color, ContentFit, Element, Font, Length, Size, Task, Theme, alignment,
    font::Weight,
    widget::{
        Canvas, Column, Image, Row, Space, column, container, responsive, scrollable, stack, svg,
        text,
    },
};
use log::{debug, warn};

use crate::{
    elements::{
        common::{self, CommonAssets, action_row, primary_button, secondary_button},
        icon_button::{IconButtonColors, IconButtonInteraction, IconButtonSize, icon_button},
    },
    enums::{FormInfo, Gender, IOAction, PokemonEntry, PokemonInfo},
    screen::info::{
        arrow_layer::ARROW,
        info::{self, Info},
        relations::{LABEL_H, ROW_SPACING, SCROLL_ROW_FRACTION, capitalize},
    },
};

struct Derived {
    entry: PokemonEntry,
    info: PokemonInfo,
    relations: Option<Vec<String>>,
    is_mega: bool,
    is_gmax: bool,
}

fn derive(info_ptr: &Info) -> Derived {
    let current = info_ptr.get_current();
    let info = info_ptr.get_pokemon_info(Some(current.key.clone()));

    let parent = info
        .evolves_from
        .as_ref()
        .and_then(|from| from.first())
        .map(|p| info_ptr.get_pokemon_info(Some(p.clone())));

    let is_mega = parent
        .as_ref()
        .and_then(|p| p.mega.as_ref())
        .is_some_and(|m| m.first() == Some(&current.key));
    let is_gmax = parent
        .as_ref()
        .and_then(|p| p.gmax.as_ref())
        .is_some_and(|g| g == &current.key);

    Derived {
        relations: info.relations.clone(),
        entry: current,
        info,
        is_mega,
        is_gmax,
    }
}

type RenderKey = (String, FormInfo);

#[derive(Debug)]
pub struct Forms {
    pub return_to: Option<Box<Info>>,

    current_entry: PokemonEntry,
    current_info: PokemonInfo,

    relations: Option<Vec<String>>,

    form_location: String,
    renders: HashMap<RenderKey, iced::widget::image::Handle>,
    pending: HashSet<RenderKey>,

    selected_pos: (usize, usize),

    shiny: bool,
    is_mega: bool,
    is_gmax: bool,

    arrow_icon: svg::Handle,
    close_icon: svg::Handle,
    missingno: iced::widget::image::Handle,
    outline_svg: svg::Handle,

    male_icon: svg::Handle,
    female_icon: svg::Handle,
    monomorphic_icon: svg::Handle,
    genderless_icon: svg::Handle,
    shiny_icon: svg::Handle,

    up_interaction: IconButtonInteraction,
    down_interaction: IconButtonInteraction,
    close_interaction: IconButtonInteraction,

    male_interaction: IconButtonInteraction,
    female_interaction: IconButtonInteraction,
    shiny_interaction: IconButtonInteraction,
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

    ToggleShiny,
    RenderLoaded(RenderKey, Result<Vec<iced::widget::image::Handle>, String>),

    Info(info::Message),
}

pub enum Action {
    None,
    Run(Task<Message>),
    Return(Box<Info>, Task<crate::info::Message>),
    OpenRelations,
}

impl Forms {
    pub fn new(return_to: Box<Info>, form_location: String) -> (Self, Task<Message>) {
        let d = derive(&return_to);

        let mut this = Self {
            return_to: Some(return_to),
            current_entry: d.entry,
            current_info: d.info,
            relations: d.relations,

            form_location,
            renders: HashMap::new(),
            pending: HashSet::new(),

            selected_pos: (0,0),

            shiny: false,
            is_mega: d.is_mega,
            is_gmax: d.is_gmax,

            arrow_icon: svg::Handle::from_memory(
                include_bytes!("../../../assets/info_screen/arrow.svg").as_slice(),
            ),
            close_icon: svg::Handle::from_memory(
                include_bytes!("../../../assets/x.svg").as_slice(),
            ),

            missingno: iced::widget::image::Handle::from_bytes(
                include_bytes!("../../../assets/missingno.png").as_slice(),
            ),
            outline_svg: svg::Handle::from_memory(
                include_bytes!("../../../assets/info_screen/sprite_outline.svg").as_slice(),
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

            male_interaction: IconButtonInteraction::default(),
            female_interaction: IconButtonInteraction::default(),
            shiny_interaction: IconButtonInteraction::default(),
        };
        let task = this.load_renders();
        (this, task)
    }

    fn refresh(&mut self) {
        let Some(ptr) = self.return_to.as_ref() else {
            return;
        };
        let d = derive(ptr);
        self.selected_pos = (0,0);
        self.current_entry = d.entry;
        self.current_info = d.info;
        self.relations = d.relations;
        self.is_mega = d.is_mega;
        self.is_gmax = d.is_gmax;
    }

    fn step(&mut self, delta: isize) -> Action {
        let Some(info) = self.return_to.as_mut() else {
            warn!("No info to return to from step!");
            return Action::None;
        };
        let step_task = info.step(delta).map(Message::Info);
        self.refresh();
        let render_task = self.load_renders();
        Action::Run(Task::batch([step_task, render_task]))
    }

    fn load_renders(&mut self) -> Task<Message> {
        let missing: Vec<RenderKey> = self
            .form_rows()
            .into_iter()
            .flatten()
            .filter(|k| !self.renders.contains_key(k) && !self.pending.contains(k))
            .collect();

        debug!("Load renders called with missing {:?} and renders {:?}", missing, self.renders);

        let tasks: Vec<_> = missing
            .into_iter()
            .map(|key| {
                self.pending.insert(key.clone());
                let dir = self.form_location.clone();
                let (name, form) = key.clone();
                debug!("Load renders called with {:?}", key);
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::io::get_form_renders(dir, name, form)
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r.map_err(|e| e.to_string()))
                    },
                    move |res| Message::RenderLoaded(key.clone(), res),
                )
            })
            .collect();

        Task::batch(tasks)
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
            Message::RenderLoaded(key, res) => {
                self.pending.remove(&key);
                match res.map(|h| h.into_iter().next()) {
                    Ok(Some(handle)) => {
                        self.renders.insert(key, handle);
                    }
                    Ok(None) => warn!("No render returned for {key:?}"),
                    Err(e) => warn!("Render load failed for {key:?}: {e}"),
                }
                Action::None
            }
            Message::ToggleShiny => {
                self.shiny = !self.shiny;
                Action::Run(self.load_renders())
            }
        }
    }

    fn form_cell<'a>(&'a self, key: &RenderKey, w: f32, sprite_h: f32, selected: bool) -> Element<'a, Message> {
        let buff_h = sprite_h * 0.10;
        let buff_w = w * 0.10;
        let outline_color = 
        // if unfocused {
            Color::from_rgb8(180, 180, 180)
        // } else {
            // common::colors::SECONDARY_FOCUS
        // }
        ;
        let outline = svg(self.outline_svg.clone())
            .width(Length::Fixed(w + buff_w))
            .height(Length::Fixed(sprite_h + buff_h))
            .style(move |_, _| svg::Style {
                color: Some(outline_color),
            });
        let sprite: Element<_> = match self.renders.get(key) {
            Some(h) => Image::new(h.clone())
                .width(Length::Fill)
                .height(Length::Fixed(sprite_h))
                .content_fit(ContentFit::Contain)
                .into(),
            None => Space::new().height(sprite_h).into(),
        };

        let content: Element<_> = if self.renders.get(key).is_some() && 
            selected {
            stack![sprite, outline].into()
        } else {
            sprite.into()
        };

        let (name, form) = key;
        let label = self
            .return_to
            .as_ref()
            .unwrap()
            .get_pokemon_info(Some(name.clone()))
            .display_name
            .unwrap_or_else(|| capitalize(name));

        container(
            column![content, text(label).size(14).color(Color::WHITE)]
                .align_x(Center)
                .spacing(4),
        )
        .width(w)
        .height(sprite_h + LABEL_H)
        .into()
    }

    fn form_row<'a>(
        &'a self,
        row_idx: usize,
        cells: &[RenderKey],
        width: f32,
        height: f32,
    ) -> Element<'a, Message> {
        let n = cells.len().max(1) as f32;
        let col_w = width / n;
        // same gutter rule as evo_fit, so sizes match the evolution screen
        let gutter = if n > 1.0 {
            (col_w * 0.30).max(ARROW + 16.0)
        } else {
            0.0
        };
        let node_w = col_w - gutter;
        let sprite_h = (height - LABEL_H - 8.0).max(40.0);

        let r = Row::with_children(cells.iter().enumerate().map(|(col, k)| {
            container(self.form_cell(k, node_w, sprite_h, self.selected_pos == (row_idx, col)))
                .width(col_w)
                .center_x(col_w)
                .into()
        }));
        container(r)
            .width(Length::Fill)
            .height(height)
            .center_y(height)
            .center_x(Length::Fill)
            .into()
    }

    fn form_rows(&self) -> Vec<Vec<RenderKey>> {
        let Some(info) = self.return_to.as_ref() else {
            return vec![];
        };
        let current = &self.current_entry.key;

        std::iter::once(current.clone())
            .chain(self.relations.iter().flatten().cloned())
            .map(|name| {
                let is_current = &name == current;
                let gender = if is_current {
                    self.current_info.gender.clone()
                } else {
                    info.get_pokemon_info(Some(name.clone())).gender
                };
                let genders = match gender {
                    Gender::Both | Gender::Genderless => vec![gender],
                    _ => vec![Gender::Female, Gender::Male],
                };
                genders
                    .into_iter()
                    .map(|gender| {
                        let form = FormInfo {
                            shiny: self.shiny,
                            // mega/gmax only apply to the current Pokémon
                            mega: is_current && self.is_mega,
                            gmax: is_current && self.is_gmax,
                            gender,
                        };
                        (name.clone(), form)
                    })
                    .collect()
            })
            .collect()
    }

    fn forms_view<'a>(&'a self, size: Size) -> Element<'a, Message> {
        let rows = self.form_rows();

        // no related forms: a single row that fills the area
        if rows.len() <= 1 {
            return match rows.first() {
                Some(cells) => self.form_row(0,cells, size.width, size.height),
                None => Space::new().into(),
            };
        }

        // related forms: scrollable rows, each slightly shorter than the viewport
        let row_h = size.height * SCROLL_ROW_FRACTION;
        let width = size.width - 12.0; // scrollbar
        scrollable(
            Column::with_children(rows.iter().enumerate().map(|(i, cells)| self.form_row(i, cells, width, row_h)))
                .spacing(ROW_SPACING)
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
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

        let chain_background = container(responsive(move |size| self.forms_view(size)))
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
