// modal.rs
use iced::{Element, Length};
use iced::widget::{button::Button, column, container, row, text, Space};

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
        elements.push(b.into());
        if buttons.peek().is_some() {
            elements.push(Space::new().width(iced::Fill).into());
        }
    }
    let button_row = row(elements).width(iced::Fill);

    const PIXEL_FONT: iced::Font = iced::Font::with_name("Open Sans Light");

    let content = column![
        text(title).size(30).font(PIXEL_FONT),
        body,
        Space::new().height(8),
        button_row,
    ]
    .spacing(12)
    .padding(16);

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