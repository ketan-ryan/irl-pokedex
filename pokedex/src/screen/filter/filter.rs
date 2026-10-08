use std::format;

use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Shadow, Task, Theme,
    alignment::Vertical::Center,
    widget::{Canvas, Space, Stack, button, column, container, mouse_area, row, stack, svg, text},
};

use crate::{
    elements::selectable::{SectionShape, SelectionDirection, SelectionGrid, SelectionPosition},
    enums::{FilterMode, IOAction, PokemonType, Region, SortDirection, SortKey},
    screen::{
        browse_pokedex::{
            browse_pokedex::PokedexBrowser,
            filter_predicate::{FilterCriteria, RangeOriginator},
        },
        common::{
            CONDENSED, CommonAssets, colors, default_shadow, holo_header_backdrop, primary_button,
            secondary_button,
        },
    },
};

impl SelectionGrid {
    fn new(sections: Vec<SectionShape>) -> Self {
        Self {
            sections,
            position: None,
        }
    }

    fn position(&self) -> Option<SelectionPosition> {
        self.position
    }

    fn navigate(&mut self, direction: SelectionDirection) {
        let Some(current) = self.position else {
            // Nothing focused: any direction lands on the top-left region.
            self.position = Some(SelectionPosition {
                section: section::REGION,
                row: 0,
                col: 0,
            });
            return;
        };

        let shape = self.sections[current.section];
        let SelectionPosition { section, row, col } = current;
        let mut next_position = current;

        let moved_within_section = match direction {
            SelectionDirection::Right if col + 1 < shape.cols => {
                next_position.col += 1;
                true
            }
            SelectionDirection::Left if col > 0 => {
                next_position.col -= 1;
                true
            }
            SelectionDirection::Down if row + 1 < shape.rows => {
                next_position.row += 1;
                true
            }
            SelectionDirection::Up if row > 0 => {
                next_position.row -= 1;
                true
            }
            _ => false,
        };

        self.position = if moved_within_section {
            Some(next_position)
        } else {
            // escape() returning None means "no neighbor that way" -- i.e.
            // off the edge of the page, so focus clears entirely rather
            // than clamping in place.
            escape(&self.sections, section, row, col, direction)
        };
    }
}

/// Layout-specific cross-section jumps. Returns where focus lands when a
/// move would leave the current section's bounds, or `None` if there's no
/// neighbor in that direction (edge of the page -- stay put).
fn escape(
    sections: &[SectionShape],
    section: usize,
    row: usize,
    col: usize,
    direction: SelectionDirection,
) -> Option<SelectionPosition> {
    use SelectionDirection::*;

    let at = |section: usize, row: usize, col: usize| Some(SelectionPosition { section, row, col });
    let rows_of = |s: usize| sections[s].rows;
    let cols_of = |s: usize| sections[s].cols;

    match (section, direction) {
        // --- Region (3x3), top-left ---
        (s, Right) if s == section::REGION => match row {
            0 => at(section::SORT_ORDER, 0, 0),
            1 => at(section::HEIGHT, 0, 0),
            _ => at(section::WEIGHT, 0, 0),
        },
        (s, Down) if s == section::REGION => {
            at(section::TYPE, 0, col.min(cols_of(section::TYPE) - 1))
        }
        (s, Up) | (s, Left) if s == section::REGION => None,

        // --- Sort Order (1x2), top-right, aligned with Region row 0 ---
        (s, Left) if s == section::SORT_ORDER => {
            at(section::REGION, 0, cols_of(section::REGION) - 1)
        }
        (s, Down) if s == section::SORT_ORDER => {
            at(section::HEIGHT, 0, col.min(cols_of(section::HEIGHT) - 1))
        }
        (s, Up) | (s, Right) if s == section::SORT_ORDER => None,

        // --- Height (1x1), top-right, aligned with Region row 1 ---
        (s, Left) if s == section::HEIGHT => at(section::REGION, 1, cols_of(section::REGION) - 1),
        (s, Up) if s == section::HEIGHT => at(
            section::SORT_ORDER,
            0,
            col.min(cols_of(section::SORT_ORDER) - 1),
        ),
        (s, Down) if s == section::HEIGHT => {
            at(section::WEIGHT, 0, col.min(cols_of(section::WEIGHT) - 1))
        }
        (s, Right) if s == section::HEIGHT => None,

        // --- Weight (1x1), top-right, aligned with Region row 2 ---
        (s, Left) if s == section::WEIGHT => at(section::REGION, 2, cols_of(section::REGION) - 1),
        (s, Up) if s == section::WEIGHT => {
            at(section::HEIGHT, 0, col.min(cols_of(section::HEIGHT) - 1))
        }
        (s, Down) if s == section::WEIGHT => {
            let targ_col = if col == 1 { 5 } else { 4 };
            at(section::TYPE, 0, targ_col)
        }
        (s, Right) if s == section::WEIGHT => None,

        // --- Type (3x6), full width, below Region & the right column ---
        (s, Up) if s == section::TYPE => {
            if col < cols_of(section::REGION) {
                at(section::REGION, rows_of(section::REGION) - 1, col)
            } else {
                let targ_col = if col == 5 { 1 } else { 0 };
                at(section::WEIGHT, 0, targ_col)
            }
        }
        (s, Down) if s == section::TYPE => {
            at(section::ACTIONS, 0, col.min(cols_of(section::ACTIONS) - 1))
        }
        (s, Left) | (s, Right) if s == section::TYPE => None,

        // --- Actions (1x3), bottom, full width ---
        (s, Up) if s == section::ACTIONS => {
            let type_rows = rows_of(section::TYPE);
            at(
                section::TYPE,
                type_rows - 1,
                col.min(cols_of(section::TYPE) - 1),
            )
        }
        (s, Down) | (s, Left) | (s, Right) if s == section::ACTIONS => None,

        _ => None,
    }
}

/// Section indices, in the fixed page (tab) order focus travels through.
mod section {
    pub const REGION: usize = 0;
    pub const SORT_ORDER: usize = 1;
    pub const HEIGHT: usize = 2;
    pub const WEIGHT: usize = 3;
    pub const TYPE: usize = 4;
    pub const ACTIONS: usize = 5;
}

const REGION_GRID_COLS: usize = 3;
const TYPE_GRID_COLS: usize = 6;

fn build_selection_grid() -> SelectionGrid {
    SelectionGrid::new(vec![
        SectionShape {
            rows: Region::ALL.len().div_ceil(REGION_GRID_COLS),
            cols: REGION_GRID_COLS,
        },
        SectionShape { rows: 1, cols: 2 }, // Sort Order: direction, key
        SectionShape { rows: 1, cols: 2 }, // Height
        SectionShape { rows: 1, cols: 2 }, // Weight
        SectionShape {
            rows: PokemonType::ALL.len().div_ceil(TYPE_GRID_COLS),
            cols: TYPE_GRID_COLS,
        },
        SectionShape { rows: 1, cols: 4 }, // Actions: Select all, Filter Mode, Clear, OK
    ])
}

#[derive(Debug)]
pub struct Filter {
    return_to: Option<Box<PokedexBrowser>>,
    criteria: FilterCriteria,

    selected_regions: Vec<Region>,
    selected_types: Vec<PokemonType>,
    sort_key: SortKey,
    sort_direction: SortDirection,
    filter_mode: FilterMode,

    selection: SelectionGrid,
}

#[derive(Clone, Debug)]
pub enum Message {
    Apply,

    RegionToggled(Region),
    TypeToggled(PokemonType),
    SortDirectionToggled,
    SortKeyToggled,
    HeightRowClicked,
    WeightRowClicked,
    FilterModeToggled,

    SelectAllToggle,
    ClearAllFilters,
    OkPressed,

    IOInput(IOAction),
}

pub enum Action {
    None,
    Run(Task<Message>),
    Return(Box<PokedexBrowser>, Task<crate::browse_pokedex::Message>),
    OpenSlider(RangeOriginator),
}

impl Filter {
    pub fn new(return_to: Box<PokedexBrowser>) -> (Self, Task<Message>) {
        let criteria = return_to.criteria();
        let selected_regions = criteria.clone().regions;
        let types = criteria.clone().types;
        let sort_key = criteria.clone().sort_key;
        let sort_direction = criteria.clone().sort_order;
        let filter_mode = criteria.clone().filter_mode;

        (
            Self {
                return_to: Some(return_to),
                criteria,

                selected_regions,
                selected_types: types,
                sort_key,
                sort_direction,
                filter_mode,

                selection: build_selection_grid(),
            },
            Task::none(),
        )
    }

    fn reset_from_criteria(&mut self) {
        self.selected_regions = self.criteria.clone().regions;
        self.selected_types = self.criteria.clone().types;
        self.sort_key = self.criteria.clone().sort_key;
        self.sort_direction = self.criteria.clone().sort_order;
        self.filter_mode = self.criteria.clone().filter_mode;
    }

    fn set_criteria(&mut self) {
        self.criteria.regions = self.selected_regions.clone();
        self.criteria.types = self.selected_types.clone();
        self.criteria.sort_key = self.sort_key;
        self.criteria.sort_order = self.sort_direction;
        self.criteria.filter_mode = self.filter_mode;
    }

    pub fn range_bounds(&self, originator: RangeOriginator) -> (f32, f32) {
        if originator == RangeOriginator::Height {
            (self.criteria.height_lower, self.criteria.height_upper)
        } else {
            (self.criteria.weight_lower, self.criteria.weight_upper)
        }
    }

    pub fn set_range_bounds(&mut self, originator: RangeOriginator, min: f32, max: f32) {
        if originator == RangeOriginator::Height {
            self.criteria.height_lower = min;
            self.criteria.height_upper = max;
        } else {
            self.criteria.weight_lower = min;
            self.criteria.weight_upper = max;
        }
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::Apply => match self.return_to.take() {
                Some(mut browser) => {
                    self.set_criteria();

                    let task = browser.apply_filter(self.criteria.clone());
                    Action::Return(browser, task)
                }
                None => Action::None,
            },
            Message::RegionToggled(region) => {
                if let Some(index) = self.selected_regions.iter().position(|r| r == &region) {
                    self.selected_regions.remove(index);
                } else {
                    self.selected_regions.push(region);
                }
                self.set_criteria();
                Action::None
            }
            Message::TypeToggled(pokemon_type) => {
                if self.selected_types.contains(&pokemon_type) {
                    self.selected_types.retain(|type_| pokemon_type != *type_);
                } else {
                    self.selected_types.push(pokemon_type);
                }
                self.set_criteria();
                Action::None
            }
            Message::SortDirectionToggled => {
                self.sort_direction = self.sort_direction.toggled();
                self.set_criteria();
                Action::None
            }
            Message::SortKeyToggled => {
                self.sort_key = self.sort_key.toggled();
                self.set_criteria();
                Action::None
            }
            Message::HeightRowClicked => Action::OpenSlider(RangeOriginator::Height),
            Message::WeightRowClicked => Action::OpenSlider(RangeOriginator::Weight),
            Message::FilterModeToggled => {
                self.filter_mode = self.filter_mode.toggled();
                Action::None
            }
            Message::ClearAllFilters => {
                self.criteria = FilterCriteria::default();
                self.reset_from_criteria();
                Action::None
            }
            Message::SelectAllToggle => {
                if self.criteria.is_all_selected() {
                    self.criteria.regions = Vec::new();
                    self.criteria.types = Vec::new();
                } else {
                    self.criteria.regions = Vec::from(Region::ALL);
                    self.criteria.types = Vec::from(PokemonType::ALL);
                }
                self.reset_from_criteria();
                Action::None
            }
            Message::OkPressed => Action::Run(Task::done(Message::Apply)),
            Message::IOInput(input) => match input {
                IOAction::Left => {
                    self.selection.navigate(SelectionDirection::Left);
                    Action::None
                }
                IOAction::Right => {
                    self.selection.navigate(SelectionDirection::Right);
                    Action::None
                }
                IOAction::ScrollUp => {
                    self.selection.navigate(SelectionDirection::Up);
                    Action::None
                }
                IOAction::ScrollDown => {
                    self.selection.navigate(SelectionDirection::Down);
                    Action::None
                }
                IOAction::Select => match self.focused_message() {
                    Some(message) => self.update(message),
                    None => Action::None,
                },
            },
        }
    }

    fn focused_message(&self) -> Option<Message> {
        let SelectionPosition { section, row, col } = self.selection.position?;

        match section {
            s if s == section::REGION => {
                let index = (row * REGION_GRID_COLS + col).min(Region::ALL.len() - 1);
                Some(Message::RegionToggled(Region::ALL[index]))
            }
            s if s == section::SORT_ORDER => match col {
                0 => Some(Message::SortDirectionToggled),
                _ => Some(Message::SortKeyToggled),
            },
            s if s == section::HEIGHT => Some(Message::HeightRowClicked),
            s if s == section::WEIGHT => Some(Message::WeightRowClicked),
            s if s == section::TYPE => {
                let index = (row * TYPE_GRID_COLS + col).min(PokemonType::ALL.len() - 1);
                Some(Message::TypeToggled(PokemonType::ALL[index]))
            }
            s if s == section::ACTIONS => match col {
                0 => Some(Message::SelectAllToggle),
                1 => Some(Message::FilterModeToggled),
                2 => Some(Message::ClearAllFilters),
                _ => Some(Message::OkPressed),
            },
            _ => None,
        }
    }

    pub fn top_view<'a>(&'a self, common: &'a CommonAssets) -> Element<'a, Message> {
        holo_header_backdrop(common, "National Pokédex", "Filter Mode")
    }

    pub fn bottom_view<'a>(&'a self, common: &'a CommonAssets) -> Element<'a, Message> {
        let focus = self.selection.position();

        let hmin = self.criteria.height_lower;
        let hmax = self.criteria.height_upper;
        let wmin = self.criteria.weight_lower;
        let wmax = self.criteria.weight_upper;

        let controls_column = column![
            sort_order_row(self.sort_direction, self.sort_key, focus),
            height_row(
                focus,
                "Height".to_string(),
                hmin,
                hmax,
                Message::HeightRowClicked
            ),
            weight_row(focus, wmin, wmax),
        ]
        .spacing(6)
        .width(Length::Fill);

        let top_row = row![region_card(&self.selected_regions, focus), controls_column]
            .spacing(20)
            .width(Length::Fill);

        stack![
            // scanlines
            container(
                Canvas::new(&common.scanlines)
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
                type_card(&self.selected_types, focus),
                action_row(self.filter_mode, self.criteria.is_all_selected(), focus),
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
fn is_focused_at(focus: Option<SelectionPosition>, section: usize, row: usize, col: usize) -> bool {
    focus.is_some_and(|f| f.section == section && f.row == row && f.col == col)
}

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

fn section_card<'a, Message: Clone + 'a>(
    content: Element<'a, Message>,
    focused: bool,
) -> Element<'a, Message> {
    container(content)
        .padding(12)
        .width(Length::Fill)
        .style(move |_theme: &Theme| {
            let (border_color, border_width) = if focused {
                (colors::PRIMARY_FOCUS, 3.0)
            } else {
                (colors::CARD_BORDER, 2.0)
            };

            container::Style {
                background: Some(Background::Color(colors::CARD_BG)),
                border: Border {
                    color: border_color,
                    width: border_width,
                    radius: 16.0.into(),
                },
                shadow: default_shadow(),
                ..Default::default()
            }
        })
        .into()
}

// ---------------------------------------------------------------------
// Region card
// ---------------------------------------------------------------------
fn region_card<'a>(
    selected: &'a Vec<Region>,
    focus: Option<SelectionPosition>,
) -> Element<'a, Message> {
    let grid_rows: Vec<Element<'a, Message>> = Region::ALL
        .chunks(REGION_GRID_COLS)
        .enumerate()
        .map(|(row_idx, chunk)| {
            let bubbles: Vec<Element<'a, Message>> = chunk
                .iter()
                .enumerate()
                .map(|(col_idx, region)| {
                    let is_ragged_row = chunk.len() < REGION_GRID_COLS;
                    let focused = focus.is_some_and(|f| {
                        f.section == section::REGION
                            && f.row == row_idx
                            && (is_ragged_row || f.col == col_idx)
                    });
                    region_bubble(*region, selected.contains(region), focused)
                })
                .collect();
            row(bubbles)
                .spacing(12)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        })
        .collect();

    let grid = column(grid_rows)
        .spacing(8)
        .width(Length::Fill)
        .height(Length::Fill);

    let content = column![section_title("Region"), grid]
        .spacing(8)
        .width(Length::Fill)
        .height(Length::Fill);

    section_card(content.into(), false)
    // card(content.into(), 175.0, 16.0, false)
}

fn region_bubble<'a>(region: Region, selected: bool, focused: bool) -> Element<'a, Message> {
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

            let (border_color, border_width) = if focused {
                (colors::PRIMARY_FOCUS, 3.0)
            } else {
                (colors::CARD_BORDER, 2.0)
            };

            button::Style {
                background: Some(Background::Color(background)),
                text_color,
                border: Border {
                    color: border_color,
                    width: border_width,
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
fn sort_order_row<'a>(
    direction: SortDirection,
    key: SortKey,
    focus: Option<SelectionPosition>,
) -> Element<'a, Message> {
    let control = container(
        row![
            sort_direction_button(direction, focus),
            sort_key_button(key, focus)
        ]
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

    section_card(content.into(), false)
}

fn sort_direction_button<'a>(
    direction: SortDirection,
    focus: Option<SelectionPosition>,
) -> Element<'a, Message> {
    let focused = is_focused_at(focus, section::SORT_ORDER, 0, 0);
    button(text(direction.glyph()).align_y(Alignment::Center).size(14))
        .padding(6)
        .style(move |theme, status| control_button_style(theme, status, focused))
        .on_press(Message::SortDirectionToggled)
        .into()
}

fn sort_key_button<'a>(key: SortKey, focus: Option<SelectionPosition>) -> Element<'a, Message> {
    let focused = is_focused_at(focus, section::SORT_ORDER, 0, 1);
    button(
        row![
            text(key.label()).size(16).align_y(Alignment::Center),
            text("⌄").align_y(Alignment::Center).size(13)
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding([6, 10])
    .style(move |theme, status| control_button_style(theme, status, focused))
    .on_press(Message::SortKeyToggled)
    .into()
}

fn control_button_style(_theme: &Theme, status: button::Status, focused: bool) -> button::Style {
    let background = match status {
        button::Status::Hovered => Some(Background::Color(colors::CONTROL_HOVER_BG)),
        _ => None,
    };

    let border = if focused {
        Border {
            color: colors::SECONDARY_FOCUS,
            width: 3.0,
            radius: 8.0.into(),
        }
    } else {
        Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 8.0.into(),
        }
    };

    button::Style {
        background,
        text_color: colors::TEXT_DARK,
        border: border,
        shadow: Shadow::default(),
        ..Default::default()
    }
}

pub fn height_row<'a, Message: Clone + 'a>(
    focus: Option<SelectionPosition>,
    title: String,
    hmin: f32,
    hmax: f32,
    message: Message,
) -> Element<'a, Message> {
    let check_focus = focus.filter(|f| f.section == section::HEIGHT);
    let range = RangeOriginator::Height;
    let min = &range.format(hmin);
    let max = &range.format(hmax);
    let content = row![
        text(title).size(16).color(colors::TEXT_DARK),
        Space::new().width(Length::Fill),
        range_display(min, max, None, check_focus),
    ]
    .align_y(Alignment::Center)
    .height(Length::Fixed(30.0))
    .width(Length::Fill);

    mouse_area(section_card(content.into(), false))
        .on_press(message)
        .into()
}

fn weight_row<'a>(focus: Option<SelectionPosition>, wmin: f32, wmax: f32) -> Element<'a, Message> {
    let check_focus = focus.filter(|f| f.section == section::WEIGHT);
    let range = RangeOriginator::Weight;
    let min = &range.format(wmin);
    let max = &range.format(wmax);
    let content = row![
        text("Weight").size(16).color(colors::TEXT_DARK),
        Space::new().width(Length::Fill),
        range_display(min, max, Some("kgs"), check_focus),
    ]
    .align_y(Alignment::Center)
    .height(Length::Fixed(30.0))
    .width(Length::Fill);

    mouse_area(section_card(content.into(), false))
        .on_press(Message::WeightRowClicked)
        .into()
}

fn range_display<'a, Message: Clone + 'a>(
    min_value: &str,
    max_value: &str,
    unit: Option<&str>,
    focus: Option<SelectionPosition>,
) -> Element<'a, Message> {
    let value_box = |value: String, focused: bool| {
        let (border_color, border_width) = if focused {
            (colors::SECONDARY_FOCUS, 3.0)
        } else {
            (colors::CARD_BORDER, 2.0)
        };
        container(
            text(value)
                .align_y(Center)
                .font(iced::Font {
                    weight: iced::font::Weight::Light,
                    family: CONDENSED.family,
                    ..Default::default()
                })
                .size(18)
                .color(colors::TEXT_DARK),
        )
        .padding([6, 12])
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(Color::WHITE)),
            border: Border {
                color: border_color,
                width: border_width,
                radius: 10.0.into(),
            },
            ..Default::default()
        })
    };

    let min_box = if focus.is_some_and(|focus| focus.col == 0) {
        value_box(min_value.to_string(), true)
    } else {
        value_box(min_value.to_string(), false)
    };

    let max_box = if focus.is_some_and(|focus| focus.col == 1) {
        value_box(max_value.to_string(), true)
    } else {
        value_box(max_value.to_string(), false)
    };

    let mut content = row![
        min_box,
        text("~").size(15).color(colors::TEXT_DARK),
        max_box,
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
fn type_card<'a>(
    selected: &'a Vec<PokemonType>,
    focus: Option<SelectionPosition>,
) -> Element<'a, Message> {
    let grid_rows: Vec<Element<'a, Message>> = PokemonType::ALL
        .chunks(6)
        .enumerate()
        .map(|(row_idx, chunk)| {
            let badges: Vec<Element<'a, Message>> = chunk
                .iter()
                .enumerate()
                .map(|(col_idx, pokemon_type)| {
                    let focused = is_focused_at(focus, section::TYPE, row_idx, col_idx);
                    type_badge(*pokemon_type, selected.contains(pokemon_type), focused)
                })
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
            .spacing(4)
            .padding(Padding {
                top: 4.0,
                ..Default::default()
            })
            .width(Length::Fill)
            .height(Length::Fill),
    ]
    .spacing(2)
    .height(Length::Fixed(140.0))
    .width(Length::Fill);

    section_card(content.into(), false)
}

fn color_to_hex(color: Color) -> String {
    // Convert 0.0..=1.0 floats to 0..=255 integers
    let r = (color.r * 255.0).round() as u8;
    let g = (color.g * 255.0).round() as u8;
    let b = (color.b * 255.0).round() as u8;

    // Returns a 6-digit hex string like "#b7410e"
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

fn type_badge<'a>(
    pokemon_type: PokemonType,
    selected: bool,
    focused: bool,
) -> Element<'a, Message> {
    let source = std::fs::read_to_string(pokemon_type.asset_path()).unwrap_or_default();

    let (border_color, border_width) = if focused {
        (color_to_hex(colors::PRIMARY_FOCUS), 3.0)
    } else {
        (color_to_hex(Color::BLACK), 1.0)
    };

    let colored_svg = source.replacen(
        r#"stroke="black"/>"#,
        &format!(r#"stroke="{border_color}" stroke-width="{border_width}"/>"#),
        1,
    );
    let icon = svg(svg::Handle::from_memory(colored_svg.into_bytes()));
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
fn action_row<'a>(
    filter_mode: FilterMode,
    all_selected: bool,
    focus: Option<SelectionPosition>,
) -> Element<'a, Message> {
    let selectall_focus = is_focused_at(focus, section::ACTIONS, 0, 0);
    let filtermode_focus = is_focused_at(focus, section::ACTIONS, 0, 1);
    let clear_focus = is_focused_at(focus, section::ACTIONS, 0, 2);
    let ok_focus = is_focused_at(focus, section::ACTIONS, 0, 3);
    row![
        secondary_button(
            "Select all".to_string(),
            Message::SelectAllToggle,
            Some(all_selected),
            selectall_focus
        ),
        secondary_button(
            format!("Filter Mode: {}", filter_mode.label()),
            Message::FilterModeToggled,
            None,
            filtermode_focus
        ),
        secondary_button(
            "Clear all filters".to_string(),
            Message::ClearAllFilters,
            None,
            clear_focus
        ),
        primary_button("OK".to_string(), Message::OkPressed, ok_focus),
    ]
    .width(Length::Fill)
    .spacing(15)
    .into()
}
