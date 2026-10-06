use iced::widget::canvas::Action as CanvasAction;
use iced::widget::canvas::{Cache, Event, Frame, Geometry, Path, Stroke};
use iced::widget::{button, canvas, column, container, row, text};
use iced::{Alignment, Element, Length, Point, Rectangle, Renderer, Theme};
use iced::{Task, mouse};

use crate::enums::IOAction;
use crate::screen::browse_pokedex::filter_predicate::RangeOriginator;
use crate::screen::common::{CommonAssets, holo_header_backdrop};
use crate::screen::filter::filter::Filter;

use std::format;

const MIN_GAP: f32 = 0.01;
const THUMB_RADIUS: f32 = 10.0;

#[derive(Debug, Clone)]
pub enum Message {
    ThumbDragged { min: f32, max: f32 },
    Cancel,
    Confirm,
    IOInput(IOAction),
}

pub enum Action {
    None,
    Run(Task<Message>),
    ReturnToFilter(Box<Filter>),
}

#[derive(Debug)]
pub struct RangeSliderScreen {
    return_to: Option<Box<Filter>>,
    abs_min: f32,
    abs_max: f32,
    min: f32,
    max: f32,
    cache: Cache,
    originator: RangeOriginator,
}

impl RangeSliderScreen {
    pub fn new(return_to: Box<Filter>, originator: RangeOriginator) -> (Self, Task<Message>) {
        let (abs_min, abs_max) = originator.abs_bounds();
        let (min, max) = return_to.range_bounds(originator);
        (
            Self {
                return_to: Some(return_to),
                abs_min,
                abs_max,
                min: min.clamp(abs_min, abs_max - MIN_GAP),
                max: max.clamp(abs_min + MIN_GAP, abs_max),
                cache: Cache::default(),
                originator,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::ThumbDragged { min, max } => {
                self.min = min;
                self.max = max;
                self.cache.clear();
                Action::None
            }
            Message::Cancel => Action::None,
            Message::Confirm => match self.return_to.take() {
                Some(mut filter) => {
                    filter.set_range_bounds(self.originator, self.min, self.max);
                    Action::ReturnToFilter(filter)
                }
                None => Action::None,
            },
            Message::IOInput(input) => match input {
                _ => Action::None,
            },
        }
    }

    pub fn top_view<'a>(&'a self, common: &'a CommonAssets) -> Element<'a, Message> {
        holo_header_backdrop(
            common,
            "Filter Mode",
            format!("Sort by {} range", self.originator.label().to_lowercase()),
        )
    }

    pub fn bottom_view(&self) -> Element<'_, Message> {
        let header = row![
            text(self.originator.label()),
            text(self.originator.format(self.abs_min)),
            text("~"),
            text(self.originator.format(self.abs_max)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let labels = row![
            column![text("MIN"), text(self.originator.format(self.min))].align_x(Alignment::Center),
            column![text("MAX"), text(self.originator.format(self.max))].align_x(Alignment::Center),
        ]
        .spacing(20);

        let slider = canvas(SliderProgram {
            abs_min: self.abs_min,
            abs_max: self.abs_max,
            min: self.min,
            max: self.max,
            cache: &self.cache,
        })
        .width(Length::Fill)
        .height(Length::Fixed(48.0));

        let footer = row![
            button("Cancel").on_press(Message::Cancel),
            button("OK").on_press(Message::Confirm),
        ]
        .spacing(16);

        container(
            column![header, labels, slider, footer]
                .spacing(16)
                .align_x(Alignment::Center),
        )
        .padding(20)
        .into()
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Thumb {
    Min,
    Max,
}

#[derive(Default)]
struct SliderState {
    dragging: Option<Thumb>,
}

struct SliderProgram<'a> {
    abs_min: f32,
    abs_max: f32,
    min: f32,
    max: f32,
    cache: &'a Cache,
}

impl SliderProgram<'_> {
    fn value_to_x(&self, value: f32, width: f32) -> f32 {
        (value - self.abs_min) / (self.abs_max - self.abs_min) * width
    }

    fn x_to_value(&self, x: f32, width: f32) -> f32 {
        let t = (x / width).clamp(0.0, 1.0);
        self.abs_min + t * (self.abs_max - self.abs_min)
    }
}

impl canvas::Program<Message> for SliderProgram<'_> {
    type State = SliderState;

    fn update(
        &self,
        state: &mut SliderState,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<CanvasAction<Message>> {
        let width = bounds.width;
        let min_x = self.value_to_x(self.min, width);
        let max_x = self.value_to_x(self.max, width);

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let pos = cursor.position_in(bounds)?;
                let thumb = if (pos.x - min_x).abs() <= (pos.x - max_x).abs() {
                    Thumb::Min
                } else {
                    Thumb::Max
                };
                state.dragging = Some(thumb);
                Some(CanvasAction::capture())
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.dragging.take().map(|_| CanvasAction::capture())
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let thumb = state.dragging?;
                let raw = self.x_to_value(position.x - bounds.x, width);

                let (new_min, new_max) = match thumb {
                    Thumb::Min => (raw.min(self.max - MIN_GAP), self.max),
                    Thumb::Max => (self.min, raw.max(self.min + MIN_GAP)),
                };

                let new_min = new_min.clamp(self.abs_min, self.abs_max - MIN_GAP);
                let new_max = new_max.clamp(self.abs_min + MIN_GAP, self.abs_max);

                Some(
                    CanvasAction::publish(Message::ThumbDragged {
                        min: new_min,
                        max: new_max,
                    })
                    .and_capture(),
                )
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &SliderState,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let geometry = self
            .cache
            .draw(renderer, bounds.size(), |frame: &mut Frame| {
                let width = frame.width();
                let mid_y = frame.height() / 2.0;

                frame.stroke(
                    &Path::line(Point::new(0.0, mid_y), Point::new(width, mid_y)),
                    Stroke::default().with_width(6.0),
                );

                let min_x = self.value_to_x(self.min, width);
                let max_x = self.value_to_x(self.max, width);

                frame.stroke(
                    &Path::line(Point::new(min_x, mid_y), Point::new(max_x, mid_y)),
                    Stroke::default()
                        .with_width(6.0)
                        .with_color(iced::Color::from_rgb(0.4, 0.85, 0.95)),
                );

                frame.fill(
                    &Path::circle(Point::new(min_x, mid_y), THUMB_RADIUS),
                    iced::Color::WHITE,
                );
                frame.fill(
                    &Path::circle(Point::new(max_x, mid_y), THUMB_RADIUS),
                    iced::Color::WHITE,
                );
            });

        return vec![geometry];
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: iced::advanced::mouse::Cursor,
    ) -> iced::advanced::mouse::Interaction {
        iced::advanced::mouse::Interaction::default()
    }
}
