use std::{collections::HashSet, str::FromStr, time::Instant};

use iced::{
    Alignment, Background, Border, Color, Element, Font, Length, Padding, Shadow, Subscription,
    Task, Theme,
    font::Weight,
    widget::{
        Canvas, Space, Stack, button, column, container, image::Handle, mouse_area, row, stack,
        svg, text,
    },
    window,
};

use crate::{
    elements::scanlines::Scanlines,
    enums::{FilterMode, PokemonType, Region, SortDirection, SortKey},
    screen::browse_pokedex::{browse_pokedex::PokedexBrowser, filter_predicate::FilterCriteria},
};

const OPEN_SANS: Font = iced::Font::with_name("Open Sans SemiBold");
const CONDENSED: Font = iced::Font::with_name("Open Sans Condensed");

#[derive(Debug)]
pub struct Filter {
    return_to: Option<Box<PokedexBrowser>>,
    criteria: FilterCriteria,

    scanlines: Scanlines,
    last_tick: Instant,
    pokeball_handle: Handle,
    filter_modal: svg::Handle,

    selected_regions: HashSet<Region>,
    selected_types: HashSet<PokemonType>,
    sort_key: SortKey,
    sort_direction: SortDirection,
    filter_mode: FilterMode,
}

#[derive(Clone, Debug)]
pub enum Message {
    Tick(Instant),
    Apply,
    Cancel,
    RegionToggled(Region),
    TypeToggled(PokemonType),
    SortDirectionToggled,
    SortKeyToggled,
    HeightRowClicked,
    WeightRowClicked,
    FilterModeToggled,
    ClearAllFilters,
    OkPressed,
}

pub enum Action {
    None,
    Run(Task<Message>),
    Return(Box<PokedexBrowser>),
}

mod colors {
    use iced::{Color, color};

    // #B5DAFF
    pub const CARD_BG: Color = color!(0xB5DAFF);
    // #2181E4
    pub const CARD_BORDER: Color = color!(0x2181E4);

    // #D0ECFF
    pub const BUBBLE_BG: Color = color!(0xD0ECFF);
    pub const BUBBLE_HOVER_BG: Color = Color::from_rgb(0.612, 0.796, 0.933);
    pub const BUBBLE_SELECTED_BG: Color = Color::from_rgb(0.106, 0.247, 0.451);

    // #003469
    pub const TEXT_DARK: Color = color!(0x003469);

    pub const CONTROL_HOVER_BG: Color = Color::from_rgb(0.867, 0.925, 0.973);

    pub const PRIMARY_BG: Color = Color::from_rgb(0.310, 0.639, 0.878);
    pub const PRIMARY_HOVER_BG: Color = Color::from_rgb(0.239, 0.561, 0.796);

    pub const TYPE_SELECTED_BORDER: Color = Color::from_rgb(0.106, 0.247, 0.451);
}

impl Filter {
    pub fn new(return_to: Box<PokedexBrowser>) -> (Self, Task<Message>) {
        let criteria = return_to.criteria();
        let regions = criteria.clone().regions;
        let types = criteria.clone().types;
        let sort_key = criteria.clone().sort_key;
        let sort_direction = criteria.clone().sort_order;
        let filter_mode = criteria.clone().filter_mode;

        (
            Self {
                return_to: Some(return_to),
                criteria,

                scanlines: Scanlines::new(),
                last_tick: Instant::now(),

                pokeball_handle: Handle::from_bytes(
                    include_bytes!("../../../assets/background.png").as_slice(),
                ),
                filter_modal: svg::Handle::from_memory(
                    include_bytes!("../../../assets/browse_screen/filter_modal.svg").as_slice(),
                ),

                selected_regions: regions,
                selected_types: types,
                sort_key,
                sort_direction,
                filter_mode,
            },
            Task::none(),
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        window::frames().map(Message::Tick)
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::Tick(now) => {
                let dt = now - self.last_tick;
                self.last_tick = now;
                self.scanlines.tick(dt);
                Action::None
            }
            Message::Apply => match self.return_to.take() {
                Some(mut browser) => {
                    self.criteria.regions = self.selected_regions.clone();
                    self.criteria.types = self.selected_types.clone();
                    self.criteria.sort_key = self.sort_key;
                    self.criteria.sort_order = self.sort_direction;

                    browser.apply_filter(self.criteria.clone());
                    Action::Return(browser)
                }
                None => Action::None,
            },
            Message::Cancel => match self.return_to.take() {
                Some(browser) => Action::Return(browser),
                None => Action::None,
            },
            Message::RegionToggled(region) => {
                if !self.selected_regions.remove(&region) {
                    self.selected_regions.insert(region);
                }
                Action::None
            }
            Message::TypeToggled(pokemon_type) => {
                if !self.selected_types.remove(&pokemon_type) {
                    self.selected_types.insert(pokemon_type);
                }
                Action::None
            }
            Message::SortDirectionToggled => {
                self.sort_direction = self.sort_direction.toggled();
                Action::None
            }
            Message::SortKeyToggled => {
                self.sort_key = self.sort_key.toggled();
                Action::None
            }
            Message::HeightRowClicked => Action::None,
            Message::WeightRowClicked => Action::None,
            Message::FilterModeToggled => {
                self.filter_mode = self.filter_mode.toggled();
                Action::None
            }
            Message::ClearAllFilters => {
                self.criteria = FilterCriteria::default();
                Action::None
            }
            Message::OkPressed => {
                println!("OK pressed (apply/close not implemented yet)");
                Action::None
            }
            _ => Action::None,
        }
    }

    pub fn top_view(&self) -> Element<'_, Message> {
        let mut font = Font::with_name("Open Sans SemiBold");
        font.weight = Weight::Semibold;
        container(stack![
            column![
                // push it down a bit for visual rather than true centering
                Space::new().height(Length::Fixed(50.0)),
                iced::widget::image(self.pokeball_handle.clone())
                    .opacity(0.2)
                    .scale(0.95)
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center),
            Canvas::new(&self.scanlines)
                .width(Length::Fill)
                .height(Length::Fill),
            container(svg(self.filter_modal.clone()).opacity(1.0)).padding(Padding {
                top: 15.0,
                ..Default::default()
            }),
            column![
                text("National Pokédex")
                    .font(font)
                    .size(22.0)
                    .color(Color::from_str("#003469").unwrap()),
                text("Filter Mode")
                    .font(CONDENSED)
                    .size(18.0)
                    .color(Color::from_str("#1867B8").unwrap())
            ]
            .spacing(8.0)
            .padding(Padding {
                top: 24.0,
                left: 22.0,
                ..Default::default()
            })
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| iced::widget::container::Style {
            background: Some(iced::Background::Color(Color::from_rgb8(140, 213, 229))),
            ..Default::default()
        })
        .into()
    }

    pub fn bottom_view(&self) -> Element<'_, Message> {
        let one: Vec<Region> = self.criteria.regions.clone().into_iter().collect();

        let controls_column = column![
            sort_order_row(self.sort_direction, self.sort_key),
            height_row(),
            weight_row(),
        ]
        .spacing(6)
        .width(Length::Fill);

        let top_row = row![region_card(&self.selected_regions, one), controls_column]
            .spacing(20)
            .width(Length::Fill);

        stack![
            // scanlines
            container(
                Canvas::new(&self.scanlines)
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(140, 213, 229))),
                ..Default::default()
            }),
            column![
                top_row,
                type_card(&self.selected_types),
                action_row(self.filter_mode),
            ]
            .spacing(20)
            .padding(24)
            .width(Length::Fill)
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

// ---------------------------------------------------------------------
// Shared building blocks
// ---------------------------------------------------------------------
fn underline<'a>() -> Element<'a, Message> {
    container(Space::new().height(Length::Fixed(2.0)))
        .width(Length::Fill)
        .height(Length::Fixed(2.0))
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(colors::CARD_BORDER)),
            ..Default::default()
        })
        .into()
}

fn section_title<'a>(label: &'static str) -> Element<'a, Message> {
    column![text(label).size(20).color(colors::TEXT_DARK), underline()]
        .spacing(8)
        .width(Length::Fill)
        .into()
}

fn section_card<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content)
        .padding(12)
        .width(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(colors::CARD_BG)),
            border: Border {
                color: colors::CARD_BORDER,
                width: 2.0,
                radius: 16.0.into(),
            },
            ..Default::default()
        })
        .into()
}

// ---------------------------------------------------------------------
// Region card
// ---------------------------------------------------------------------
fn region_card<'a>(
    selected: &'a HashSet<Region>,
    all_regions: Vec<Region>,
) -> Element<'a, Message> {
    let grid_rows: Vec<Element<'a, Message>> = all_regions
        .chunks(3)
        .map(|chunk| {
            let bubbles: Vec<Element<'a, Message>> = chunk
                .iter()
                .map(|region| region_bubble(region.clone(), selected.contains(region)))
                .collect();
            row(bubbles).spacing(12).width(Length::Fill).into()
        })
        .collect();

    let content = column![
        section_title("Region"),
        column(grid_rows).spacing(6).width(Length::Fill),
    ]
    .spacing(6)
    .width(Length::Fill)
    .height(Length::Fixed(150.0));

    section_card(content.into())
}

fn region_bubble<'a>(region: Region, selected: bool) -> Element<'a, Message> {
    let label = text(region.label())
        .size(16)
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center);

    button(label)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([8, 4])
        .style(move |_theme: &Theme, status: button::Status| {
            let (background, text_color) = if selected {
                (colors::BUBBLE_SELECTED_BG, Color::WHITE)
            } else {
                match status {
                    button::Status::Hovered => (colors::BUBBLE_HOVER_BG, colors::TEXT_DARK),
                    _ => (colors::BUBBLE_BG, colors::TEXT_DARK),
                }
            };

            button::Style {
                background: Some(Background::Color(background)),
                text_color,
                border: Border {
                    color: colors::CARD_BORDER,
                    width: 2.0,
                    radius: 999.0.into(),
                },
                shadow: Shadow::default(),
                ..Default::default()
            }
        })
        .on_press(Message::RegionToggled(region))
        .into()
}

// ---------------------------------------------------------------------
// Sort order / height / weight rows
// ---------------------------------------------------------------------
fn sort_order_row<'a>(direction: SortDirection, key: SortKey) -> Element<'a, Message> {
    let control = container(
        row![sort_direction_button(direction), sort_key_button(key)]
            .spacing(4)
            .align_y(Alignment::Center),
    )
    .padding([4, 10])
    .style(|_theme: &Theme| container::Style {
        background: Some(Background::Color(Color::WHITE)),
        border: Border {
            color: colors::CARD_BORDER,
            width: 2.0,
            radius: 999.0.into(),
        },
        ..Default::default()
    });

    let content = row![
        text("Sort Order")
            .size(16)
            .align_y(Alignment::Center)
            .color(colors::TEXT_DARK),
        Space::new().width(Length::Fill),
        control,
    ]
    .align_y(Alignment::Center)
    .height(Length::Fixed(30.0))
    .width(Length::Fill);

    section_card(content.into())
}

fn sort_direction_button<'a>(direction: SortDirection) -> Element<'a, Message> {
    button(text(direction.glyph()).align_y(Alignment::Center).size(14))
        .padding(6)
        .style(control_button_style)
        .on_press(Message::SortDirectionToggled)
        .into()
}

fn sort_key_button<'a>(key: SortKey) -> Element<'a, Message> {
    button(
        row![
            text(key.label()).size(16).align_y(Alignment::Center),
            text("⌄").align_y(Alignment::Center).size(13)
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding([6, 10])
    .style(control_button_style)
    .on_press(Message::SortKeyToggled)
    .into()
}

fn control_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => Some(Background::Color(colors::CONTROL_HOVER_BG)),
        _ => None,
    };

    button::Style {
        background,
        text_color: colors::TEXT_DARK,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 8.0.into(),
        },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

fn height_row<'a>() -> Element<'a, Message> {
    let content = row![
        text("Height").size(16).color(colors::TEXT_DARK),
        Space::new().width(Length::Fill),
        range_display("0'00\"", "99'99\"", None),
    ]
    .align_y(Alignment::Center)
    .height(Length::Fixed(30.0))
    .width(Length::Fill);

    mouse_area(section_card(content.into()))
        .on_press(Message::HeightRowClicked)
        .into()
}

fn weight_row<'a>() -> Element<'a, Message> {
    let content = row![
        text("Weight").size(16).color(colors::TEXT_DARK),
        Space::new().width(Length::Fill),
        range_display("0.0", "9999.0", Some("lbs")),
    ]
    .align_y(Alignment::Center)
    .height(Length::Fixed(30.0))
    .width(Length::Fill);

    mouse_area(section_card(content.into()))
        .on_press(Message::WeightRowClicked)
        .into()
}

fn range_display<'a>(min_value: &str, max_value: &str, unit: Option<&str>) -> Element<'a, Message> {
    let value_box = |value: String| {
        container(text(value).size(15).color(colors::TEXT_DARK))
            .padding([6, 12])
            .style(|_theme: &Theme| container::Style {
                background: Some(Background::Color(Color::WHITE)),
                border: Border {
                    color: colors::CARD_BORDER,
                    width: 2.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            })
    };

    let mut content = row![
        value_box(min_value.to_string()),
        text("~").size(15).color(colors::TEXT_DARK),
        value_box(max_value.to_string()),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    if let Some(unit) = unit {
        content = content.push(text(unit.to_string()).size(15).color(colors::TEXT_DARK));
    }

    content.into()
}

// ---------------------------------------------------------------------
// Type card
// ---------------------------------------------------------------------
fn type_card<'a>(selected: &'a HashSet<PokemonType>) -> Element<'a, Message> {
    let grid_rows: Vec<Element<'a, Message>> = PokemonType::ALL
        .chunks(6)
        .map(|chunk| {
            let badges: Vec<Element<'a, Message>> = chunk
                .iter()
                .map(|pokemon_type| type_badge(*pokemon_type, selected.contains(pokemon_type)))
                .collect();
            row(badges)
                .spacing(2)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        })
        .collect();

    let content = column![
        section_title("Type"),
        column(grid_rows)
            .spacing(3)
            .width(Length::Fill)
            .height(Length::Fill),
    ]
    .spacing(2)
    .height(Length::Fixed(140.0))
    .width(Length::Fill);

    section_card(content.into())
}

fn type_badge<'a>(pokemon_type: PokemonType, selected: bool) -> Element<'a, Message> {
    let icon = svg(svg::Handle::from_path(pokemon_type.asset_path())).height(Length::Fixed(24.0));
    let mut elements: Vec<Element<Message>> = vec![icon.into()];
    if !selected {
        elements.push(
            svg(svg::Handle::from_path(pokemon_type.overlay_path()))
                .height(Length::Fixed(24.0))
                .opacity(0.85)
                .into(),
        );
    }

    button(Stack::with_children(elements))
        .width(Length::Fill)
        .padding(2)
        .style(
            move |_theme: &Theme, _status: button::Status| button::Style {
                background: None,
                text_color: Color::TRANSPARENT,
                ..Default::default()
            },
        )
        .on_press(Message::TypeToggled(pokemon_type))
        .into()
}

// ---------------------------------------------------------------------
// Bottom action row
// ---------------------------------------------------------------------
fn action_row<'a>(filter_mode: FilterMode) -> Element<'a, Message> {
    row![
        secondary_button(
            format!("Filter Mode: {}", filter_mode.label()),
            Message::FilterModeToggled,
        ),
        secondary_button("Clear all filters".to_string(), Message::ClearAllFilters),
        primary_button("OK".to_string(), Message::OkPressed),
    ]
    .width(Length::Fill)
    .spacing(40)
    .into()
}

fn secondary_button<'a>(label: String, message: Message) -> Element<'a, Message> {
    button(
        text(label)
            .align_y(Alignment::Center)
            .size(18)
            .color(colors::TEXT_DARK),
    )
    .height(Length::Fixed(50.0))
    .padding([12, 42])
    .style(|_theme: &Theme, status: button::Status| {
        let background = match status {
            button::Status::Hovered => colors::CONTROL_HOVER_BG,
            _ => Color::WHITE,
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: colors::TEXT_DARK,
            border: Border {
                color: colors::CARD_BORDER,
                width: 2.0,
                radius: 20.0.into(),
            },
            shadow: default_shadow(),
            ..Default::default()
        }
    })
    .on_press(message)
    .into()
}

fn primary_button<'a>(label: String, message: Message) -> Element<'a, Message> {
    button(
        text(label)
            .align_y(Alignment::Center)
            .size(18)
            .color(Color::WHITE),
    )
    .height(Length::Fixed(50.0))
    .padding([12, 30])
    .style(|_theme: &Theme, status: button::Status| {
        let background = match status {
            button::Status::Hovered => colors::PRIMARY_HOVER_BG,
            _ => colors::PRIMARY_BG,
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: Color::WHITE,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 20.0.into(),
            },
            shadow: default_shadow(),
            ..Default::default()
        }
    })
    .on_press(message)
    .into()
}

fn default_shadow() -> Shadow {
    Shadow {
        blur_radius: 4.0,
        color: Color::BLACK,
        offset: iced::Vector { x: 1.0, y: 3.0 },
    }
}
