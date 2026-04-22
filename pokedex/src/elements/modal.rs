// modal.rs
use iced::widget::{Space, button, button::Button, column, container, row, text};
use iced::{Border, Color, Element, Length, Theme, Vector};

use crate::elements::message_box::Panel;

pub const SCALE_WIDTH: f32 = 600.0;
const PIXEL_FONT: iced::Font = iced::Font::with_name("Open Sans Condensed Light");
const MIN_FONT_SIZE: f32 = 10.0;
const MAX_FONT_SIZE: f32 = 150.0;
const AVERAGE_CHAR_WIDTH_RATIO: f32 = 0.56; // heuristic for monospace-like fonts

/// Shrinks text to fit within max width and height, allowing for wrapping up to max_wrapped_lines.
pub fn shrink_text_to_fit(
    text: &str,
    requested_size: f32,
    max_width: f32,
    min_size: f32,
    max_wrapped_lines: usize,
    max_height: f32,
) -> f32 {
    if max_width <= 0.0 || max_height <= 0.0 {
        return min_size.max(MIN_FONT_SIZE);
    }

    let chars = text.chars().count() as f32;
    if chars == 0.0 {
        return min_size.max(MIN_FONT_SIZE);
    }

    let mut size = requested_size.max(min_size).clamp(min_size, MAX_FONT_SIZE);

    // More conservative estimation with padding for descenders
    let line_count = |font_size: f32| {
        let width = chars * font_size * AVERAGE_CHAR_WIDTH_RATIO;
        (width / max_width).ceil().max(1.0)
    };
    // Line height is typically 1.3-1.4, add extra padding for descenders
    let estimated_height = |font_size: f32, lines: f32| {
        let line_height = font_size * 1.4; // More conservative than 1.2
        lines * line_height + (font_size * 0.2) // Extra padding for descenders
    };

    // Shrink if too large
    while size > min_size
        && (line_count(size) > max_wrapped_lines as f32
            || estimated_height(size, line_count(size)) > max_height)
    {
        size -= 0.5; // Smaller increments
    }

    // Grow back up if there's room - but leave a safety margin
    while size < MAX_FONT_SIZE
        && line_count(size + 0.5) <= max_wrapped_lines as f32
        && estimated_height(size + 0.5, line_count(size + 0.5)) <= (max_height * 0.95)
    // 5% safety margin
    {
        size += 0.5;
    }

    size.clamp(min_size, MAX_FONT_SIZE)
}

pub fn modal<'a, Message: Clone + 'static>(
    title: Option<String>,
    body: Element<'a, Message>,
    buttons: Vec<Button<'a, Message>>,
    width: f32,
    height: f32,
    padding: Option<iced::Padding>,
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

    let scale_factor = width / SCALE_WIDTH;

    // only add title if it is Some
    let mut content_col = column![];
    let mut top_padding = 4.0;
    if let Some(t) = title {
        content_col = content_col.push(
            text(t)
                .font(PIXEL_FONT)
                .size(24.0 * scale_factor)
                .color(Color::BLACK),
        );
    } else {
        top_padding = 14.0;
    }
    let body_padding = padding.unwrap_or(iced::Padding {
        top: top_padding * scale_factor,
        bottom: 6.0 * scale_factor,
        left: 60.0 * (scale_factor / 1.25),
        right: 60.0 * (scale_factor / 1.25),
    });

    let content = content_col
        .push(container(body).center(Length::Fill))
        .push(Space::new().height(4))
        .push(button_row)
        .spacing(4)
        .padding(body_padding)
        .height(iced::Fill);

    container(Panel::new(content).width(width).height(height))
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .align_x(iced::Center)
        .align_y(iced::Center)
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
