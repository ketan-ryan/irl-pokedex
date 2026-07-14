//! A reusable on-screen "holo" virtual keyboard widget for Iced 0.14.
//!
//! This owns its own text buffer, cursor position, shift/caps-lock state,
//! and blink/scroll behaviour, so the parent app only needs to store a
//! `Keyboard` and route its `Message` through. Wire it up like this:
//!
//! ```ignore
//! struct App {
//!     keyboard: keyboard::Keyboard,
//!     show_keyboard: bool,
//! }
//!
//! enum Message {
//!     Keyboard(keyboard::Message),
//!     // ...
//! }
//!
//! // in `update`:
//! Message::Keyboard(msg) => {
//!     let (task, action) = self.keyboard.update(msg);
//!     match action {
//!         keyboard::Action::Closed => self.show_keyboard = false,
//!         keyboard::Action::Submitted(text) => { /* use `text` */ }
//!         keyboard::Action::KeyPressed(_) | keyboard::Action::None => {}
//!     }
//!     task.map(Message::Keyboard)
//! }
//!
//! // in `view`, whenever `show_keyboard` is true:
//! self.keyboard.view().map(Message::Keyboard)
//!
//! // in `subscription`, whenever `show_keyboard` is true:
//! self.keyboard.subscription().map(Message::Keyboard)
//! ```

use iced::advanced::graphics::futures::event;
use iced::event::Status;
use iced::keyboard::Event::KeyPressed;
use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::{
    self, Column, Row, Space, button, container, mouse_area, operation, scrollable, text,
};
use iced::{
    Background, Border, Color, Element, Event, Font, Length, Padding, Shadow, Subscription, Task,
    Vector,
};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum IOAction {
    ScrollUp,
    ScrollDown,
    Left,
    Right,
}

// ---- layout constants (tuned to land at roughly 580x420 for the modal) ----
const MODAL_WIDTH: f32 = 580.0;
const KEY_W: f32 = 44.0;
const KEY_H: f32 = 44.0;
const GAP: f32 = 8.0;
const SPECIAL_W: f32 = 64.0; // shift / backspace
const CLOSE_W: f32 = 56.0;
const ENTER_W: f32 = 92.0;
const ROW1_WIDTH: f32 = KEY_W * 10.0 + GAP * 9.0; // 512
const ROW2_INDENT: f32 = (ROW1_WIDTH - (KEY_W * 9.0 + GAP * 8.0)) / 2.0; // ~26, centers row 2 under row 1
const ROW3_WIDTH: f32 = SPECIAL_W * 2.0 + KEY_W * 7.0 + GAP * 8.0; // 500
const ROW3_INDENT: f32 = (ROW1_WIDTH - ROW3_WIDTH) / 2.0; // ~6

const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(350);
const CURSOR_BLINK: Duration = Duration::from_millis(530);

// ---- palette (matches the white/sky-blue "holo" reference) ----
const BODY_BG: Color = Color::WHITE;
const PANEL_BG: Color = Color::from_rgba8(140, 215, 255, 1.0);
const KEY_BG: Color = Color::WHITE;
const KEY_TEXT: Color = Color {
    r: 0.2,
    g: 0.2,
    b: 0.2,
    a: 1.0,
};
const KEY_ACTIVE_BG: Color = Color {
    r: 0.102,
    g: 0.451,
    b: 0.820,
    a: 1.0,
}; // the "a"-key dark blue
const KEY_SHIFT_ON_BG: Color = Color {
    r: 0.35,
    g: 0.62,
    b: 0.90,
    a: 1.0,
}; // one-shot shift (lighter)
const CLOSE_BG: Color = Color {
    r: 0.85,
    g: 0.85,
    b: 0.85,
    a: 1.0,
};
const FIELD_BG: Color = Color {
    r: 0.945,
    g: 0.945,
    b: 0.945,
    a: 1.0,
};
const FIELD_FOCUS_BORDER: Color = Color {
    r: 0.16,
    g: 0.54,
    b: 0.88,
    a: 1.0,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShiftState {
    Off,
    Shift,
    CapsLock,
}

impl ShiftState {
    fn is_upper(self) -> bool {
        matches!(self, ShiftState::Shift | ShiftState::CapsLock)
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Char(char),
    Backspace,
    Shift,
    Enter,
    Close,
    FocusField,
    BlinkCursor,
    CursorLeft,
    CursorRight,
    IOInput(IOAction),
}

/// What happened as a result of an `update` call, for the parent to react to.
#[derive(Debug, Clone)]
pub enum Action {
    None,
    /// A letter or space was entered; carries the character actually typed
    /// (already case-adjusted for shift/caps-lock).
    KeyPressed(char),
    /// Enter was pressed; carries the full text entered so far.
    Submitted(String),
    /// The close (X) button was pressed.
    Closed,
}

#[derive(Debug)]
pub struct Keyboard {
    value: String,
    cursor: usize, // char index into `value`
    focused: bool,
    cursor_visible: bool,
    shift_state: ShiftState,
    last_shift_click: Option<Instant>,
    show_text_field: bool,
    scroll_id: widget::Id,
    font: Option<Font>,
}

impl Keyboard {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            focused: false,
            cursor_visible: true,
            shift_state: ShiftState::Off,
            last_shift_click: None,
            show_text_field: true,
            scroll_id: widget::Id::unique(),
            font: None,
        }
    }

    /// Some uses of this keyboard drive a text field that lives *elsewhere*
    /// in the app, and don't need the keyboard's own field displayed.
    /// Key presses still fire `Action::KeyPressed` either way.
    pub fn with_text_field(mut self, show: bool) -> Self {
        self.show_text_field = show;
        self
    }

    /// Overrides the font used for every label this widget draws (keys,
    /// space/enter, and the typed text). Falls back to the app's default
    /// font if never called. If the font you pass is missing a glyph
    /// (e.g. the shift/backspace/close symbols below), iced's text shaping
    /// falls back to another available font for just that glyph, so it's
    /// safe to set this even for a font that only covers latin letters.
    pub fn with_font(mut self, font: iced::Font) -> Self {
        self.font = Some(font);
        self
    }

    fn key_font(&self) -> iced::Font {
        self.font.unwrap_or_default()
    }

    pub fn text(&self) -> &str {
        &self.value
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Action) {
        match message {
            Message::Char(ch) => {
                self.insert_char(ch);
                // one-shot shift is consumed by the next character typed
                if self.shift_state == ShiftState::Shift {
                    self.shift_state = ShiftState::Off;
                }
                (self.snap_scroll(), Action::KeyPressed(ch))
            }
            Message::Backspace => {
                if self.cursor > 0 {
                    let mut chars: Vec<char> = self.value.chars().collect();
                    chars.remove(self.cursor - 1);
                    self.value = chars.into_iter().collect();
                    self.cursor -= 1;
                }
                self.focused = true;
                self.cursor_visible = true;
                (self.snap_scroll(), Action::None)
            }
            Message::Shift => {
                let now = Instant::now();
                self.shift_state = match self.shift_state {
                    ShiftState::Off => ShiftState::Shift,
                    ShiftState::Shift => {
                        let double_clicked = self
                            .last_shift_click
                            .map(|t| now.duration_since(t) < DOUBLE_CLICK_WINDOW)
                            .unwrap_or(false);
                        if double_clicked {
                            ShiftState::CapsLock
                        } else {
                            ShiftState::Off
                        }
                    }
                    ShiftState::CapsLock => ShiftState::Off,
                };
                self.last_shift_click = Some(now);
                (Task::none(), Action::None)
            }
            Message::Enter => (Task::none(), Action::Submitted(self.value.clone())),
            Message::Close => (Task::none(), Action::Closed),
            Message::FocusField => {
                self.focused = true;
                self.cursor_visible = true;
                (Task::none(), Action::None)
            }
            Message::BlinkCursor => {
                self.cursor_visible = !self.cursor_visible;
                (Task::none(), Action::None)
            }
            Message::CursorLeft => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                self.cursor_visible = true;
                (self.snap_scroll(), Action::None)
            }
            Message::CursorRight => {
                let len = self.value.chars().count();
                if self.cursor < len {
                    self.cursor += 1;
                }
                self.cursor_visible = true;
                (self.snap_scroll(), Action::None)
            }
            Message::IOInput(input) => match input {
                IOAction::Left => (Task::done(Message::CursorLeft), Action::None),
                IOAction::Right => (Task::done(Message::CursorRight), Action::None),
                _ => (Task::none(), Action::None),
            },
        }
    }

    /// Only ticks the blink timer and captures Left/Right arrow keys while
    /// the field is focused, so this widget doesn't steal arrow-key input
    /// used elsewhere in the app (e.g. list navigation) when it's not.
    pub fn subscription(&self) -> Subscription<Message> {
        // TODO change this
        if !self.focused {
            return Subscription::none();
        }
        Subscription::batch([
            iced::time::every(CURSOR_BLINK).map(|_| Message::BlinkCursor),
            event::listen_with(|event, status, _| match (event, status) {
                (
                    Event::Keyboard(KeyPressed {
                        key: Key::Named(Named::ArrowUp),
                        ..
                    }),
                    Status::Ignored,
                ) => Some(Message::IOInput(IOAction::ScrollUp)),
                (
                    Event::Keyboard(KeyPressed {
                        key: Key::Named(Named::ArrowDown),
                        ..
                    }),
                    Status::Ignored,
                ) => Some(Message::IOInput(IOAction::ScrollDown)),
                (
                    Event::Keyboard(KeyPressed {
                        key: Key::Named(Named::ArrowLeft),
                        ..
                    }),
                    Status::Ignored,
                ) => Some(Message::IOInput(IOAction::Left)),
                (
                    Event::Keyboard(KeyPressed {
                        key: Key::Named(Named::ArrowRight),
                        ..
                    }),
                    Status::Ignored,
                ) => Some(Message::IOInput(IOAction::Right)),
                _ => None,
            }),
        ])
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut body = Column::new()
            .spacing(14)
            .width(Length::Fixed(MODAL_WIDTH - 32.0));

        if self.show_text_field {
            body = body.push(self.text_field_view());
        }

        body = body.push(self.keys_panel_view());

        container(body)
            .padding(16)
            .width(Length::Fixed(MODAL_WIDTH))
            .style(|_theme| container::Style {
                background: Some(Background::Color(BODY_BG)),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 28.0.into(),
                },
                shadow: Shadow {
                    color: Color::from_rgba8(0, 0, 0, 0.30),
                    offset: Vector::new(0.0, 4.0),
                    blur_radius: 20.0,
                },
                ..Default::default()
            })
            .into()
    }

    fn insert_char(&mut self, ch: char) {
        let mut chars: Vec<char> = self.value.chars().collect();
        chars.insert(self.cursor, ch);
        self.value = chars.into_iter().collect();
        self.cursor += 1;
        self.focused = true;
        self.cursor_visible = true;
    }

    /// Keeps the cursor roughly in view by snapping scroll position
    /// proportionally to how far through the text the cursor sits.
    /// This is an approximation (Iced doesn't expose glyph metrics at this
    /// level) but keeps long entries usable. Refine with precise text
    /// measurement later if pixel-perfect scrolling matters.
    fn snap_scroll(&self) -> Task<Message> {
        let len = self.value.chars().count().max(1) as f32;
        let ratio = (self.cursor as f32 / len).clamp(0.0, 1.0);
        operation::snap_to(
            self.scroll_id.clone(),
            scrollable::RelativeOffset { x: ratio, y: 0.0 },
        )
    }

    fn text_field_view(&self) -> Element<'_, Message> {
        let chars: Vec<char> = self.value.chars().collect();
        let before: String = chars[..self.cursor].iter().collect();
        let after: String = chars[self.cursor..].iter().collect();

        let cursor_color = if self.focused && self.cursor_visible {
            FIELD_FOCUS_BORDER
        } else {
            Color::TRANSPARENT
        };

        let cursor_bar = container(
            Space::new()
                .width(Length::Fixed(2.0))
                .height(Length::Fixed(22.0)),
        )
        .style(move |_theme| container::Style {
            background: Some(Background::Color(cursor_color)),
            ..Default::default()
        });

        let content = Row::new()
            .align_y(iced::Alignment::Center)
            .push(
                text(before)
                    .size(18)
                    .font(self.key_font())
                    .color(Color::from_rgb8(0x33, 0x33, 0x33)),
            )
            .push(cursor_bar)
            .push(
                text(after)
                    .size(18)
                    .font(self.key_font())
                    .color(Color::from_rgb8(0x33, 0x33, 0x33)),
            );

        // NOTE: scrollbar chrome left at its default appearance below.
        // If you want it fully invisible, tune `scrollable::Scrollbar`
        // (width / scroller_width to 0) — check your installed iced 0.14
        // docs for the exact builder name, it has shifted slightly across
        // point releases.
        let scroll = scrollable(content)
            .id(self.scroll_id.clone())
            .direction(scrollable::Direction::Horizontal(Default::default()))
            .width(Length::Fill);

        let (border_color, border_width) = if self.focused {
            (FIELD_FOCUS_BORDER, 2.0)
        } else {
            (Color::TRANSPARENT, 0.0)
        };

        mouse_area(
            container(scroll)
                .padding(Padding::from([10.0, 16.0]))
                .width(Length::Fill)
                .height(Length::Fixed(52.0))
                .style(move |_theme| container::Style {
                    background: Some(Background::Color(FIELD_BG)),
                    border: Border {
                        color: border_color,
                        width: border_width,
                        radius: 14.0.into(),
                    },
                    shadow: Shadow {
                        color: Color::from_rgba8(0, 0, 0, 0.25),
                        offset: Vector::new(0.0, 2.0),
                        blur_radius: 5.0,
                    },
                    ..Default::default()
                }),
        )
        .on_press(Message::FocusField)
        .into()
    }

    fn keys_panel_view(&self) -> Element<'_, Message> {
        let row1 = Row::with_children(
            "qwertyuiop"
                .chars()
                .map(|c| self.letter_key(c))
                .collect::<Vec<_>>(),
        )
        .spacing(GAP);

        let row2 = Row::new()
            .push(
                Space::new()
                    .width(Length::Fixed(ROW2_INDENT))
                    .height(Length::Shrink),
            )
            .push(
                Row::with_children(
                    "asdfghjkl"
                        .chars()
                        .map(|c| self.letter_key(c))
                        .collect::<Vec<_>>(),
                )
                .spacing(GAP),
            );

        let row3 = Row::new()
            .push(
                Space::new()
                    .width(Length::Fixed(ROW3_INDENT))
                    .height(Length::Shrink),
            )
            .push(
                Row::new()
                    .spacing(GAP)
                    .push(self.shift_key())
                    .push(
                        Row::with_children(
                            "zxcvbnm"
                                .chars()
                                .map(|c| self.letter_key(c))
                                .collect::<Vec<_>>(),
                        )
                        .spacing(GAP),
                    )
                    .push(self.backspace_key()),
            );

        let row4 = Row::new()
            .spacing(GAP)
            .push(self.close_key())
            .push(self.space_key())
            .push(self.enter_key());

        let panel = Column::new()
            .spacing(GAP)
            .push(row1)
            .push(row2)
            .push(row3)
            .push(row4);

        container(panel)
            .padding(12)
            .width(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(Background::Color(PANEL_BG)),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 22.0.into(),
                },
                shadow: Shadow {
                    color: Color::from_rgba8(0, 0, 0, 0.15),
                    offset: Vector::new(0.0, 2.0),
                    blur_radius: 6.0,
                },
                ..Default::default()
            })
            .into()
    }

    fn letter_key(&self, base: char) -> Element<'_, Message> {
        let display = if self.shift_state.is_upper() {
            base.to_ascii_uppercase()
        } else {
            base
        };

        button(centered(
            text(display.to_string()).size(20).font(self.key_font()),
        ))
        .on_press(Message::Char(display))
        .width(Length::Fixed(KEY_W))
        .height(Length::Fixed(KEY_H))
        .style(key_style)
        .into()
    }

    fn shift_key(&self) -> Element<'_, Message> {
        let bg = match self.shift_state {
            ShiftState::Off => KEY_BG,
            ShiftState::Shift => KEY_SHIFT_ON_BG,
            ShiftState::CapsLock => KEY_ACTIVE_BG,
        };
        let fg = if self.shift_state == ShiftState::Off {
            KEY_TEXT
        } else {
            Color::WHITE
        };

        button(centered(
            text("\u{21e7}").size(20).color(fg).font(self.key_font()),
        ))
        .on_press(Message::Shift)
        .width(Length::Fixed(SPECIAL_W))
        .height(Length::Fixed(KEY_H))
        .style(move |_theme, status| button::Style {
            background: Some(Background::Color(match status {
                button::Status::Pressed => darken(bg),
                _ => bg,
            })),
            text_color: fg,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        })
        .into()
    }

    fn backspace_key(&self) -> Element<'_, Message> {
        button(centered(text("\u{232b}").size(20).font(self.key_font())))
            .on_press(Message::Backspace)
            .width(Length::Fixed(SPECIAL_W))
            .height(Length::Fixed(KEY_H))
            .style(key_style)
            .into()
    }

    fn close_key(&self) -> Element<'_, Message> {
        button(centered(text("\u{2715}").size(16).font(self.key_font())))
            .on_press(Message::Close)
            .width(Length::Fixed(CLOSE_W))
            .height(Length::Fixed(KEY_H))
            .style(|_theme, status| button::Style {
                background: Some(Background::Color(match status {
                    button::Status::Pressed | button::Status::Hovered => darken(CLOSE_BG),
                    _ => CLOSE_BG,
                })),
                text_color: Color::from_rgb8(0x44, 0x44, 0x44),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn space_key(&self) -> Element<'_, Message> {
        button(centered(text("Space").size(16).font(self.key_font())))
            .on_press(Message::Char(' '))
            .width(Length::Fill)
            .height(Length::Fixed(KEY_H))
            .style(key_style)
            .into()
    }

    fn enter_key(&self) -> Element<'_, Message> {
        button(centered(text("Enter").size(16).font(self.key_font())))
            .on_press(Message::Enter)
            .width(Length::Fixed(ENTER_W))
            .height(Length::Fixed(KEY_H))
            .style(key_style)
            .into()
    }
}

impl Default for Keyboard {
    fn default() -> Self {
        Self::new()
    }
}

fn key_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Pressed => KEY_ACTIVE_BG,
        button::Status::Hovered => Color::from_rgb8(0xEF, 0xF7, 0xFF),
        _ => KEY_BG,
    };
    let fg = if status == button::Status::Pressed {
        Color::WHITE
    } else {
        KEY_TEXT
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: fg,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 12.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.12),
            offset: Vector::new(0.0, 1.0),
            blur_radius: 2.0,
        },
        ..Default::default()
    }
}

fn darken(color: Color) -> Color {
    Color {
        r: color.r * 0.85,
        g: color.g * 0.85,
        b: color.b * 0.85,
        a: color.a,
    }
}

/// Centers arbitrary content within its available space. Used so key
/// labels sit dead-center in their buttons instead of at their natural
/// (shrink-to-fit) position.
fn centered<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into()
}
