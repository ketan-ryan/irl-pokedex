// modal.rs
use iced::{Color, Border, Element, Length, Theme, Vector};
use iced::widget::{button, button::Button, column, container, row, text, Space};

use crate::elements::message_box::Panel;

pub fn modal<'a, Message: Clone + 'static>(
    title: &'a str,
    body: Element<'a, Message>,
    buttons: Vec<Button<'a, Message>>,
    width: f32,
) -> Element<'a, Message> {
    let mut elements: Vec<Element<Message>> = vec![];
    let mut buttons = buttons.into_iter().peekable();
    while let Some(b) = buttons.next() {
        elements.push(b.style(custom_button_style).into());
        if buttons.peek().is_some() {
            elements.push(Space::new().width(iced::Fill).into());
        }
    }
    let button_row = row(elements).width(iced::Fill);

    const PIXEL_FONT: iced::Font = iced::Font::with_name("Open Sans Light");

    let content = column![
        text(title).size(30).font(PIXEL_FONT),
        body,
        Space::new().height(4),
        button_row,
    ]
    .spacing(4)
    .padding(iced::Padding { top: 4.0, bottom: 6.0, left: 70.0, right: 70.0 });

    iced::widget::stack![
        container(
            Panel::new(content).width(width)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::Center)
        .align_y(iced::Center),
    ]
    .into()
}

fn custom_button_style(theme: &Theme, status: button::Status) -> button::Style {
    // Define style based on state (e.g., pressed, hovered)
    match status {
        button::Status::Active | button::Status::Pressed => button::Style {
            background: Some(Color::from_rgba(0.2, 0.2, 0.2, 0.6).into()),
            border: Border {
                color: Color::WHITE,
                width: 1.0,
                radius: 5.0.into(),
            },
            text_color: Color::WHITE,
            ..Default::default()
        },
        button::Status::Hovered => button::Style {
            background: Some(Color::from_rgba8(0, 102, 255, 0.9).into()),
            shadow: iced::Shadow {
                color: Color::from_rgba8(0, 112, 255, 1.0),
                offset: Vector::new(0.0, 0.0),
                blur_radius: 16.0,
            },
            ..custom_button_style(theme, button::Status::Active) // Reuse active
        },
        _ => button::Style {
            background: Some(Color::from_rgba(0.05, 0.05, 0.05, 0.6).into()),
            border: Border {
                color: Color::BLACK,
                width: 1.0,
                radius: 5.0.into(),
            },
            text_color: Color::WHITE,
            ..Default::default()
        },
    }
}
