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
//!     Input(keyboard::InputAction), // from your dpad capture elsewhere
//! }
//!
//! // whenever you open the keyboard:
//! self.show_keyboard = true;
//! self.keyboard.reset_focus();
//!
//! // in `update`, both of these return `(Task<Message>, Action)` and can
//! // be handled identically:
//! Message::Keyboard(msg) => {
//!     let (task, action) = self.keyboard.update(msg);
//!     // handle `action`, then `task.map(Message::Keyboard)`
//! }
//! Message::Input(input) => {
//!     let (task, action) = self.keyboard.handle_input(input);
//!     // handle `action`, then `task.map(Message::Keyboard)`
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
    r: 0.9,
    g: 0.9,
    b: 0.9,
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

// ---- dpad focus grid ----
//
// Row 0 is the text field (a single focus target).
// Rows 1-2 are the plain letter rows.
// Row 3 is `[shift, z, x, c, v, b, n, m, backspace]` — 9 slots, not 7;
//   the letters are sandwiched between shift (index 0) and backspace
//   (the last index).
// Row 4 is `[close, space, enter]`.
//
// `keys_panel_view` renders directly from `ROW_1` / `ROW_2` / `ROW_3_LETTERS`
// below (rather than separate hardcoded strings), so the rendered row and
// its navigable length can't drift apart.
const ROW_1: &'static str = "qwertyuiop";
const ROW_2: &'static str = "asdfghjkl";
const ROW_3_LETTERS: &'static str = "zxcvbnm";

const ROW_3_SHIFT_IDX: usize = 0;
const ROW_3_LEN: usize = ROW_3_LETTERS.len() + 2; // + shift + backspace
const ROW_3_BACKSPACE_IDX: usize = ROW_3_LEN - 1;

const ROW_4_CLOSE_IDX: usize = 0;
const ROW_4_SPACE_IDX: usize = 1;
const ROW_4_ENTER_IDX: usize = 2;
const ROW_4_LEN: usize = 3;

const ROW_LENS: [usize; 5] = [1, ROW_1.len(), ROW_2.len(), ROW_3_LEN, ROW_4_LEN];

#[derive(Debug, Clone)]
pub enum InputAction {
    Left,
    Right,
    Up,
    Down,
    Select,
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
    /// The user clicked somewhere in the keyboard that isn't a focusable
    /// element (a key or the text field) — clears dpad focus.
    LoseFocus,
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
    cursor_visible: bool,
    shift_state: ShiftState,
    last_shift_click: Option<Instant>,
    show_text_field: bool,
    scroll_id: widget::Id,
    font: Option<Font>,

    // Dpad focus grid. Row 0 is the text field, rows 1-4 are the keys
    // (see the ROW_* constants above). `focused_idx` is `None` until the
    // first input arrives, per the "nothing focused at first" behaviour.
    focused_idx: Option<usize>,
    current_row: usize,
}

impl Keyboard {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            cursor_visible: true,
            shift_state: ShiftState::Off,
            last_shift_click: None,
            show_text_field: true,
            scroll_id: widget::Id::unique(),
            font: None,
            focused_idx: None,
            current_row: 1,
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

    /// Clears dpad focus back to the initial state (nothing highlighted,
    /// `q` as the origin for the next input). Call this every time you
    /// show/open the keyboard so focus doesn't carry over from whatever
    /// it was left on last time.
    pub fn reset_focus(&mut self) {
        self.current_row = 1;
        self.focused_idx = None;
    }

    pub fn handle_input(&mut self, input: InputAction) -> (Task<Message>, Action) {
        // The text field owns its own cursor navigation (via the physical
        // arrow-key listener in `subscription`); there's nothing in the
        // key grid to move — or select — while it's focused.
        if self.current_row != 0 {
            self.cursor_visible = true;
        }

        let Some(current) = self.focused_idx else {
            self.focused_idx = Some(0);
            return (Task::none(), Action::None);
        };

        let row_len = ROW_LENS[self.current_row];

        match input {
            InputAction::Right => {
                if self.current_row != 0 {
                    self.focused_idx = Some((current + 1) % row_len);
                }
                (Task::none(), Action::None)
            }
            InputAction::Left => {
                // Adding `row_len` before subtracting means this can never
                // underflow, even when `current` is already 0 (`usize`
                // can't go negative).
                if self.current_row != 0 {
                    self.focused_idx = Some((current + row_len - 1) % row_len);
                }
                (Task::none(), Action::None)
            }
            InputAction::Down => {
                self.cursor_visible = true;
                let next_row = self.row_below(self.current_row);
                self.shift_row(current, next_row);
                (Task::none(), Action::None)
            }
            InputAction::Up => {
                self.cursor_visible = true;
                let next_row = self.row_above(self.current_row);
                self.shift_row(current, next_row);
                (Task::none(), Action::None)
            }
            InputAction::Select => match self.message_for_focus() {
                Some(msg) => self.update(msg),
                None => (Task::none(), Action::None),
            },
        }
    }

    /// The row a "down" press from `row` lands on, skipping the text-field
    /// row entirely when it isn't shown.
    fn row_below(&self, row: usize) -> usize {
        let next = if row == 4 { 0 } else { row + 1 };
        if next == 0 && !self.show_text_field {
            1
        } else {
            next
        }
    }

    /// The row an "up" press from `row` lands on, skipping the text-field
    /// row entirely when it isn't shown.
    fn row_above(&self, row: usize) -> usize {
        let prev = if row == 0 { 4 } else { row - 1 };
        if prev == 0 && !self.show_text_field {
            row
        } else {
            prev
        }
    }

    /// Moves focus to `target_row`, clamping the column so it can't land
    /// past that row's last slot (rows have different lengths).
    fn shift_row(&mut self, current_idx: usize, target_row: usize) {
        self.current_row = target_row;
        self.focused_idx = Some(current_idx.min(ROW_LENS[target_row].saturating_sub(1)));
    }

    /// Whether the key at `(row, idx)` in the dpad grid currently has focus.
    fn is_key_focused(&self, row: usize, idx: usize) -> bool {
        self.current_row == row && self.focused_idx == Some(idx)
    }

    /// Maps the currently dpad-focused element to the `Message` that
    /// pressing/clicking it would send, so `InputAction::Select` can go
    /// through the exact same handling as a mouse click. `None` when
    /// nothing is focused, or focus is on the text field (not "pressable").
    fn message_for_focus(&self) -> Option<Message> {
        let idx = self.focused_idx?;
        let cased = |c: char| {
            if self.shift_state.is_upper() {
                c.to_ascii_uppercase()
            } else {
                c
            }
        };

        match self.current_row {
            1 => ROW_1.chars().nth(idx).map(|c| Message::Char(cased(c))),
            2 => ROW_2.chars().nth(idx).map(|c| Message::Char(cased(c))),
            3 => match idx {
                ROW_3_SHIFT_IDX => Some(Message::Shift),
                ROW_3_BACKSPACE_IDX => Some(Message::Backspace),
                i => ROW_3_LETTERS
                    .chars()
                    .nth(i - 1)
                    .map(|c| Message::Char(cased(c))),
            },
            4 => match idx {
                ROW_4_CLOSE_IDX => Some(Message::Close),
                ROW_4_SPACE_IDX => Some(Message::Char(' ')),
                ROW_4_ENTER_IDX => Some(Message::Enter),
                _ => None,
            },
            _ => None, // row 0: the text field isn't itself "pressable"
        }
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
                self.cursor_visible = true;
                (self.snap_scroll(), Action::KeyPressed(' '))
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
                // The text field is row 0 in the same focus grid as every
                // other key, so "focusing" it just means moving there.
                self.current_row = 0;
                self.focused_idx = Some(0);
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
            Message::LoseFocus => {
                self.reset_focus();
                (Task::none(), Action::None)
            }
        }
    }

    /// Only ticks the blink timer and captures Left/Right arrow keys while
    /// the text field is dpad-focused, so this widget doesn't steal
    /// arrow-key input used elsewhere in the app when it's not.
    pub fn subscription(&self) -> Subscription<Message> {
        if self.current_row != 0 {
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

        let modal = container(body)
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
            });

        // Catches clicks that land on the modal's own background (padding,
        // panel gaps) rather than on a key or the text field. Those more
        // specific widgets capture their own clicks first, so this only
        // fires for genuinely "outside a focusable element" clicks.
        mouse_area(modal).on_press(Message::LoseFocus).into()
    }

    fn insert_char(&mut self, ch: char) {
        let mut chars: Vec<char> = self.value.chars().collect();
        chars.insert(self.cursor, ch);
        self.value = chars.into_iter().collect();
        self.cursor += 1;
        self.cursor_visible = true;
    }

    /// Keeps the cursor roughly in view by snapping scroll position
    /// proportionally to how far through the text the cursor sits.
    /// This is an approximation (Iced doesn't expose glyph metrics at this
    /// level) but keeps long entries usable.
    fn snap_scroll(&self) -> Task<Message> {
        let len = self.value.chars().count().max(1) as f32;
        let ratio = (self.cursor as f32 / len).clamp(0.0, 1.0);
        operation::snap_to(
            self.scroll_id.clone(),
            scrollable::RelativeOffset { x: ratio, y: 0.0 },
        )
    }

    fn text_field_view(&self) -> Element<'_, Message> {
        let field_focused = self.current_row == 0;

        let chars: Vec<char> = self.value.chars().collect();
        let before: String = chars[..self.cursor].iter().collect();
        let after: String = chars[self.cursor..].iter().collect();

        let cursor_color = if self.cursor_visible {
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

        let scroll = scrollable(content)
            .id(self.scroll_id.clone())
            .direction(scrollable::Direction::Horizontal(Default::default()))
            .width(Length::Fill);

        let (border_color, border_width) = if field_focused {
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
        let row1 = self.letter_row(1, ROW_1, 0);

        let row2 = Row::new()
            .push(
                Space::new()
                    .width(Length::Fixed(ROW2_INDENT))
                    .height(Length::Shrink),
            )
            .push(self.letter_row(2, ROW_2, 0));

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
                    // Row 3's letters sit between shift (index 0) and
                    // backspace (the last index), so they're offset by 1.
                    .push(self.letter_row(3, ROW_3_LETTERS, 1))
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

    /// Builds a row of letter keys from `letters`, wiring each one's focus
    /// state up to `(row, index_offset + position in the string)`.
    fn letter_row(&self, row: usize, letters: &str, index_offset: usize) -> Row<'_, Message> {
        Row::with_children(
            letters
                .chars()
                .enumerate()
                .map(|(i, c)| self.letter_key(c, self.is_key_focused(row, i + index_offset)))
                .collect::<Vec<_>>(),
        )
        .spacing(GAP)
    }

    fn letter_key(&self, base: char, focused: bool) -> Element<'_, Message> {
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
        .style(move |theme, status| key_style(theme, status, focused))
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
        let (border_color, border_width) = if self.is_key_focused(3, ROW_3_SHIFT_IDX) {
            (FIELD_FOCUS_BORDER, 2.0)
        } else {
            (Color::TRANSPARENT, 0.0)
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
                button::Status::Hovered => Color::from_rgb8(106, 168, 230),
                _ => bg,
            })),
            text_color: fg,
            border: Border {
                color: border_color,
                width: border_width,
                radius: 12.0.into(),
            },
            ..Default::default()
        })
        .into()
    }

    fn backspace_key(&self) -> Element<'_, Message> {
        let focused = self.is_key_focused(3, ROW_3_BACKSPACE_IDX);

        button(centered(text("\u{232b}").size(20).font(self.key_font())))
            .on_press(Message::Backspace)
            .width(Length::Fixed(SPECIAL_W))
            .height(Length::Fixed(KEY_H))
            .style(move |theme, status| key_style(theme, status, focused))
            .into()
    }

    fn close_key(&self) -> Element<'_, Message> {
        let (border_color, border_width) = if self.is_key_focused(4, ROW_4_CLOSE_IDX) {
            (FIELD_FOCUS_BORDER, 2.0)
        } else {
            (Color::TRANSPARENT, 0.0)
        };
        button(centered(text("\u{2715}").size(16).font(self.key_font())))
            .on_press(Message::Close)
            .width(Length::Fixed(CLOSE_W))
            .height(Length::Fixed(KEY_H))
            .style(move |_theme, status| button::Style {
                background: Some(Background::Color(match status {
                    button::Status::Pressed | button::Status::Hovered => darken(CLOSE_BG),
                    _ => CLOSE_BG,
                })),
                text_color: Color::from_rgb8(0x44, 0x44, 0x44),
                border: Border {
                    color: border_color,
                    width: border_width,
                    radius: 10.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn space_key(&self) -> Element<'_, Message> {
        let focused = self.is_key_focused(4, ROW_4_SPACE_IDX);
        button(centered(text("Space").size(16).font(self.key_font())))
            .on_press(Message::Char(' '))
            .width(Length::Fill)
            .height(Length::Fixed(KEY_H))
            .style(move |theme, status| key_style(theme, status, focused))
            .into()
    }

    fn enter_key(&self) -> Element<'_, Message> {
        let focused = self.is_key_focused(4, ROW_4_ENTER_IDX);
        button(centered(text("Enter").size(16).font(self.key_font())))
            .on_press(Message::Enter)
            .width(Length::Fixed(ENTER_W))
            .height(Length::Fixed(KEY_H))
            .style(move |theme, status| key_style(theme, status, focused))
            .into()
    }
}

impl Default for Keyboard {
    fn default() -> Self {
        Self::new()
    }
}

fn key_style(_theme: &iced::Theme, status: button::Status, focused: bool) -> button::Style {
    let bg = match status {
        button::Status::Pressed => KEY_ACTIVE_BG,
        button::Status::Hovered => Color::from_rgb8(106, 168, 230),
        _ => KEY_BG,
    };
    let fg = if status == button::Status::Pressed || status == button::Status::Hovered {
        Color::WHITE
    } else {
        KEY_TEXT
    };

    let (border_color, border_width) = if focused {
        (FIELD_FOCUS_BORDER, 2.0)
    } else {
        (Color::TRANSPARENT, 0.0)
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: fg,
        border: Border {
            color: border_color,
            width: border_width,
            radius: 12.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.32),
            offset: Vector::new(1.0, 1.0),
            blur_radius: 4.0,
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
