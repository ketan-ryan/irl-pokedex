use iced::{
    widget::{button, container, mouse_area, row, svg, text},
    Alignment, Border, Color, Element, Length, Shadow,
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
    fn resolve(&self, state: &IconButtonInteraction) -> (Color, Color, Color, Shadow) {
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
    let mut content_row = row![icon_widget].spacing(6).align_y(Alignment::Center);

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
