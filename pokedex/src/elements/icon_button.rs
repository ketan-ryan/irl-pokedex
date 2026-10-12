use iced::{
    Alignment, Border, Color, Element, Length, Shadow,
    widget::{Space, button, container, mouse_area, row, svg, text},
};

#[derive(Debug, Clone, PartialEq)]
pub enum IconButtonInteraction {
    None,
    Hovered,
    Pressed,
    Released,
}

impl Default for IconButtonInteraction {
    fn default() -> Self {
        Self::None
    }
}

// ─── Color scheme ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct IconButtonColors {
    pub idle_bg: Color,
    pub idle_svg: Color,
    pub idle_text: Color,
    pub hover_bg: Color,
    pub hover_fg: Color,
    pub pressed_bg: Color,
    pub pressed_fg: Color,
    pub border_radius: f32,
    pub border_width: f32,
    pub default_shadow: Shadow,
    pub selected_shadow: Shadow,
}

impl Default for IconButtonColors {
    fn default() -> Self {
        let blue = Color::from_rgb8(33, 130, 228);
        let default_shadow = Shadow {
            blur_radius: 4.0,
            color: Color::BLACK,
            offset: iced::Vector { x: 0.2, y: 1.0 },
        };
        let selected_shadow = Shadow {
            blur_radius: 10.0,
            color: blue,
            offset: iced::Vector { x: 0.0, y: 0.0 },
        };
        Self {
            idle_bg: Color::WHITE,
            idle_svg: blue,
            idle_text: Color::BLACK,
            hover_bg: Color::from_rgb8(106, 168, 230),
            hover_fg: Color::WHITE,
            pressed_bg: blue,
            pressed_fg: Color::WHITE,
            border_radius: 16.0,
            border_width: 1.0,
            default_shadow,
            selected_shadow,
        }
    }
}

impl IconButtonColors {
    /// Resolve the colors and shadow that should be used for the given interaction state.
    ///
    /// Args:
    /// - state: The current button interaction state.
    ///
    /// Returns: A tuple containing the background, icon, text, and shadow styling to render.
    pub fn resolve(&self, state: &IconButtonInteraction) -> (Color, Color, Color, Shadow) {
        match state {
            IconButtonInteraction::Pressed => (
                self.pressed_bg,
                self.pressed_fg,
                self.pressed_fg,
                self.selected_shadow,
            ),
            IconButtonInteraction::Hovered => (
                self.hover_bg,
                self.hover_fg,
                self.hover_fg,
                self.selected_shadow,
            ),
            IconButtonInteraction::None => (
                self.idle_bg,
                self.idle_svg,
                self.idle_text,
                self.default_shadow,
            ),
            IconButtonInteraction::Released => (
                self.idle_bg,
                self.idle_svg,
                self.idle_text,
                self.default_shadow,
            ),
        }
    }
}

// ─── Widget ──────────────────────────────────────────────────────────────────

/// Build a rounded icon-and-label button with state-driven styling.
///
/// Usage:
/// ```
/// icon_button(
///     self.search_svg.clone(),
///     Some("Search"),
///     &self.search_interaction,
///     IconButtonColors::default(),
///     Message::SearchInteraction,
/// )
/// ```
///
/// Args:
/// - icon: The SVG handle to display inside the button.
/// - label: An optional label shown next to the icon.
/// - state: The current interaction state used to resolve styling.
/// - colors: The color palette and border settings for the button.
/// - on_interact: A callback that maps interaction states to application messages.
///
/// Returns: An iced element that renders the button widget.
pub fn icon_button<'a, Message>(
    icon: svg::Handle,
    label: Option<&'a str>,
    state: &IconButtonInteraction,
    colors: IconButtonColors,
    on_interact: impl Fn(IconButtonInteraction) -> Message + 'a,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let (bg, svg_col, fg, shadow) = colors.resolve(state);

    // SVG — tinted to fg color
    let icon_widget = svg(icon)
        .width(Length::Fixed(20.0))
        .height(Length::Fixed(20.0))
        .style(move |_, _| svg::Style {
            color: Some(svg_col),
        });

    // Row: icon + optional label
    let mut content_row: row::Row<'_, Message> =
        row![icon_widget].spacing(6).align_y(Alignment::Center);

    if let Some(label_str) = label {
        content_row = content_row.push(
            text(label_str)
                .color(fg)
                .size(15)
                .align_x(Alignment::Center),
        );
    }

    // Inner container owns the rounded shape + background
    let inner = container(content_row)
        .padding(iced::Padding {
            top: 4.0,
            bottom: 4.0,
            left: 10.0,
            right: 10.0,
        })
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: Some(fg),
            border: Border {
                radius: colors.border_radius.into(),
                width: colors.border_width,
                color: svg_col,
            },
            shadow: shadow,
            ..Default::default()
        });

    let btn = button(inner)
        .height(Length::Fixed(40.0))
        .padding(5)
        .style(|_, _| button::Style {
            background: None,
            ..Default::default()
        });

    mouse_area(btn)
        .on_enter(on_interact(IconButtonInteraction::Hovered))
        .on_exit(on_interact(IconButtonInteraction::None))
        .on_press(on_interact(IconButtonInteraction::Pressed))
        .on_release(on_interact(IconButtonInteraction::Released))
        .into()
}

fn combined_interaction(
    a: &IconButtonInteraction,
    b: &IconButtonInteraction,
) -> IconButtonInteraction {
    use IconButtonInteraction::*;
    match (a, b) {
        (Pressed, _) | (_, Pressed) => Pressed,
        (Hovered, _) | (_, Hovered) => Hovered,
        (Released, _) | (_, Released) => Released,
        _ => None,
    }
}

fn outer_half_radius(is_left: bool, r: f32) -> iced::border::Radius {
    if is_left {
        iced::border::Radius {
            top_left: r,
            bottom_left: r,
            top_right: 0.0,
            bottom_right: 0.0,
        }
    } else {
        iced::border::Radius {
            top_left: 0.0,
            bottom_left: 0.0,
            top_right: r,
            bottom_right: r,
        }
    }
}

pub fn split_icon_button<'a, Message>(
    left_icon: svg::Handle,
    left_label: Option<&'a str>,
    left_state: &IconButtonInteraction,
    left_colors: IconButtonColors,
    on_left_interact: impl Fn(IconButtonInteraction) -> Message + 'a,

    right_icon: svg::Handle,
    right_label: Option<&'a str>,
    right_state: &IconButtonInteraction,
    right_colors: IconButtonColors,
    on_right_interact: impl Fn(IconButtonInteraction) -> Message + 'a,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let (left_bg, left_svg_col, left_fg, _) = left_colors.resolve(left_state);
    let (right_bg, right_svg_col, right_fg, _) = right_colors.resolve(right_state);

    // Shared shadow/border react to EITHER half being interacted with —
    // resolved once, off the combined state, same as the single-button case.
    let shared_state = combined_interaction(left_state, right_state);
    let (_, _, _, shared_shadow) = left_colors.resolve(&shared_state);
    let shared_border_color = left_colors.resolve(&shared_state).1;

    // Left half
    let left_icon_widget = svg(left_icon)
        .width(Length::Fixed(20.0))
        .height(Length::Fixed(20.0))
        .style(move |_, _| svg::Style {
            color: Some(left_svg_col),
        });

    let mut left_row: row::Row<'_, Message> =
        row![left_icon_widget].spacing(6).align_y(Alignment::Center);
    if let Some(s) = left_label {
        left_row = left_row.push(text(s).color(left_fg).size(15));
    }

    let left_half = mouse_area(
        container(left_row)
            .padding(iced::Padding {
                top: 4.0,
                bottom: 4.0,
                left: 10.0,
                right: 10.0,
            })
            .height(Length::Fill)
            .align_y(Alignment::Center)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(left_bg)),
                text_color: Some(left_fg),
                border: Border {
                    radius: outer_half_radius(true, 16.0),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                ..Default::default()
            }),
    )
    // ... same on_enter/on_exit/on_press/on_release
    .on_enter(on_left_interact(IconButtonInteraction::Hovered))
    .on_exit(on_left_interact(IconButtonInteraction::None))
    .on_press(on_left_interact(IconButtonInteraction::Pressed))
    .on_release(on_left_interact(IconButtonInteraction::Released));

    // Right half
    let right_icon_widget = svg(right_icon)
        .width(Length::Fixed(20.0))
        .height(Length::Fixed(20.0))
        .style(move |_, _| svg::Style {
            color: Some(right_svg_col),
        });

    let mut right_row: row::Row<'_, Message> = row![right_icon_widget]
        .spacing(6)
        .align_y(Alignment::Center);
    if let Some(s) = right_label {
        right_row = right_row.push(text(s).color(right_fg).size(15));
    }

    let right_half = mouse_area(
        container(right_row)
            .padding(iced::Padding {
                top: 4.0,
                bottom: 4.0,
                left: 10.0,
                right: 10.0,
            })
            .height(Length::Fill)
            .align_y(Alignment::Center)
            .style(move |_| container::Style {
                border: Border {
                    radius: outer_half_radius(false, 16.0),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                background: Some(iced::Background::Color(right_bg)),
                text_color: Some(right_fg),
                ..Default::default()
            }),
    )
    .on_enter(on_right_interact(IconButtonInteraction::Hovered))
    .on_exit(on_right_interact(IconButtonInteraction::None))
    .on_press(on_right_interact(IconButtonInteraction::Pressed))
    .on_release(on_right_interact(IconButtonInteraction::Released));

    let divider =
        container(Space::new().width(Length::Fixed(1.0)).height(Length::Fill)).style(|_| {
            container::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(0x1A, 0x3A, 0x4A))),
                ..Default::default()
            }
        });

    let inner_row = row![left_half, divider, right_half]
        .height(Length::Fixed(28.0))
        .align_y(Alignment::Center);

    container(inner_row)
        .width(Length::Shrink)
        .height(Length::Fixed(30.0)) // +2 to account for padding eating into content height
        .padding(1.0)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_| container::Style {
            border: Border {
                radius: 18.0.into(),
                width: 6.0,
                color: shared_border_color,
            },
            shadow: shared_shadow,
            ..Default::default()
        })
        .into()
}
