// screen/common.rs

use iced::font::Weight;
use iced::widget::{Canvas, Space, column, container, image, stack, svg, text};
use iced::{Alignment, Color, Element, Font, Length, Padding};
use std::str::FromStr;

use crate::elements::scanlines::Scanlines;

pub const CONDENSED: Font = iced::Font::with_name("Open Sans Condensed");

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
