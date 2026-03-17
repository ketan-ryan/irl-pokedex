// modal.rs
use iced::{Color, Element, Length};
use iced::widget::{button::Button, column, container, row, text, Space};

use crate::elements::message_box::Panel;
use crate::elements::hex_button::HexButton;


pub struct ModalButton<Message> {
    pub label: String,
    pub on_press: Message,
}

pub fn modal<'a, Message: Clone + 'static>(
    base: Element<'a, Message>,
    title: &'a str,
    body: &'a str,
    buttons: Vec<Button<'a, Message>>,
    width: f32,
) -> Element<'a, Message> {
    let button_row = row(
        buttons.into_iter().map(|b| {
            Element::from(b)
        }).collect::<Vec<_>>()
    ).spacing(12);
    const PIXEL_FONT: iced::Font = iced::Font::with_name("Open Sans Light");

    let content = column![
        text(title).size(30).font(PIXEL_FONT),
        text(body).size(24).font(PIXEL_FONT),
        Space::new().height(8),
        button_row,
    ]
    .spacing(12)
    .padding(16);

    iced::widget::stack![
        base,
        // dark overlay
        // container(Space::new().width(Length::Fill).height(Length::Fill))
        //     .style(|_| container::Style {
        //         background: Some(iced::Background::Color(Color { a: 0.7, ..Color::BLACK })),
        //         ..Default::default()
        //     })
        //     .width(Length::Fill)
        //     .height(Length::Fill),
        // centered panel
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