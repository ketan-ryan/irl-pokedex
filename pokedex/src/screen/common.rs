// screen/common.rs

use iced::font::Weight;
use iced::widget::{Canvas, Space, button, column, container, image, stack, svg, text};
use iced::{Alignment, Background, Border, Color, Element, Font, Length, Padding, Shadow, Theme};
use std::str::FromStr;

use crate::elements::scanlines::Scanlines;

pub const CONDENSED: Font = iced::Font::with_name("Open Sans Condensed");

pub mod colors {
    use iced::{Color, color};

    // #B5DAFF
    pub const CARD_BG: Color = color!(0xB5DAFF);
    // #2181E4
    pub const CARD_BORDER: Color = color!(0x2181E4);

    // #D0ECFF
    pub const BUBBLE_BG: Color = color!(0xD0ECFF);
    // #9CCBEE
    pub const BUBBLE_HOVER_BG: Color = Color::from_rgb(0.612, 0.796, 0.933);
    // #1b3f73
    pub const BUBBLE_SELECTED_BG: Color = Color::from_rgb(0.106, 0.247, 0.451);

    // #003469
    pub const TEXT_DARK: Color = color!(0x003469);

    // #ddecf8
    pub const CONTROL_HOVER_BG: Color = Color::from_rgb(0.867, 0.925, 0.973);

    // #4fa3e0
    pub const PRIMARY_BG: Color = Color::from_rgb(0.310, 0.639, 0.878);
    // #3d8fcb
    pub const PRIMARY_HOVER_BG: Color = Color::from_rgb(0.239, 0.561, 0.796);

    /// Cursor / keyboard-focus ring, distinct from selection state.
    pub const PRIMARY_FOCUS: Color = Color::WHITE;
    // #30fff7
    pub const SECONDARY_FOCUS: Color = Color::from_rgb(0.188, 1.0, 0.969);
}

pub struct CommonAssets {
    pub pokeball_handle: image::Handle,
    pub filter_modal: svg::Handle,
    pub scanlines: Scanlines,
}

impl CommonAssets {
    pub fn new(pokeball_handle: image::Handle, filter_modal: svg::Handle) -> Self {
        Self {
            pokeball_handle,
            filter_modal,
            scanlines: Scanlines::new(),
        }
    }
}

pub fn holo_header_backdrop<'a, Message: 'a>(
    assets: &'a CommonAssets,
    header_text: impl Into<String>,
    sub_text: impl Into<String>,
) -> Element<'a, Message> {
    let mut font = Font::with_name("Open Sans SemiBold");
    font.weight = Weight::Semibold;

    container(stack![
        column![
            Space::new().height(Length::Fixed(50.0)),
            image(assets.pokeball_handle.clone())
                .opacity(0.2)
                .scale(0.95)
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center),
        Canvas::new(&assets.scanlines)
            .width(Length::Fill)
            .height(Length::Fill),
        container(svg(assets.filter_modal.clone()).opacity(1.0)).padding(Padding {
            top: 15.0,
            ..Default::default()
        }),
        column![
            text(header_text.into())
                .font(font)
                .size(22.0)
                .color(Color::from_str("#003469").unwrap()),
            text(sub_text.into())
                .font(iced::Font {
                    weight: iced::font::Weight::Thin,
                    family: CONDENSED.family,
                    ..Default::default()
                })
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

pub fn secondary_button<'a, Message: Clone + 'a>(
    label: impl Into<String>,
    message: Message,
    selected: Option<bool>,
    focused: bool,
) -> Element<'a, Message> {
    let is_selected = selected.unwrap_or(false);
    let text_color = if is_selected {
        Color::WHITE
    } else {
        colors::TEXT_DARK
    };

    button(
        text(label.into())
            .align_y(Alignment::Center)
            .size(18)
            .color(text_color),
    )
    .height(Length::Fixed(50.0))
    .padding([12, 22])
    .style(move |_theme: &Theme, status: button::Status| {
        let background = match (status, is_selected) {
            (button::Status::Hovered, true) => colors::PRIMARY_HOVER_BG,
            (button::Status::Hovered, false) => colors::CONTROL_HOVER_BG,
            (_, true) => colors::PRIMARY_BG,
            (_, false) => Color::WHITE,
        };

        let (border_color, border_width) = if focused {
            (colors::SECONDARY_FOCUS, 3.0)
        } else {
            (colors::CARD_BORDER, 2.0)
        };

        button::Style {
            background: Some(Background::Color(background)),
            text_color: Color::WHITE,
            border: Border {
                color: border_color,
                width: border_width,
                radius: 20.0.into(),
            },
            shadow: default_shadow(),
            ..Default::default()
        }
    })
    .on_press(message)
    .into()
}

pub fn primary_button<'a, Message: Clone + 'a>(
    label: impl Into<String>,
    message: Message,
    focused: bool,
) -> Element<'a, Message> {
    let (border_color, border_width) = if focused {
        (colors::PRIMARY_FOCUS, 3.0)
    } else {
        (Color::TRANSPARENT, 0.0)
    };

    button(
        text(label.into())
            .align_y(Alignment::Center)
            .size(18)
            .color(Color::WHITE),
    )
    .height(Length::Fixed(50.0))
    .padding([12, 30])
    .style(move |_theme: &Theme, status: button::Status| {
        let background = match status {
            button::Status::Hovered => colors::PRIMARY_HOVER_BG,
            _ => colors::PRIMARY_BG,
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: Color::WHITE,
            border: Border {
                color: border_color,
                width: border_width,
                radius: 20.0.into(),
            },
            shadow: default_shadow(),
            ..Default::default()
        }
    })
    .on_press(message)
    .into()
}

pub fn default_shadow() -> Shadow {
    Shadow {
        blur_radius: 4.0,
        color: Color::BLACK,
        offset: iced::Vector { x: 1.0, y: 3.0 },
    }
}
