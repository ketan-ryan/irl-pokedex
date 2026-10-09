use iced::alignment::{Horizontal, Vertical};
use iced::widget::canvas::Action as CanvasAction;
use iced::widget::canvas::{Cache, Event, Frame, Geometry, Path, Stroke, Text};
use iced::widget::{Canvas, Space, canvas, column, container, row, stack};
use iced::{Alignment, Color, Element, Length, Point, Rectangle, Renderer, Theme, color};
use iced::{Size, Task, mouse};

use crate::enums::IOAction;
use crate::screen::browse_pokedex::filter_predicate::RangeOriginator;
use crate::screen::common::{
    self, CommonAssets, holo_header_backdrop, primary_button, secondary_button,
};
use crate::screen::filter::filter::{Filter, height_row};

use std::format;

#[derive(Debug, Clone)]
pub enum Message {
    ThumbDragged { min: f32, max: f32 },
    Cancel,
    Confirm,
    IOInput(IOAction),
    None,
}

pub enum Action {
    None,
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
            Message::Cancel => match self.return_to.take() {
                Some(filter) => Action::ReturnToFilter(filter),
                None => Action::None,
            },
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
            Message::None => Action::None,
        }
    }

    pub fn top_view<'a>(&'a self, common: &'a CommonAssets) -> Element<'a, Message> {
        holo_header_backdrop(
            common,
            "Filter Mode",
            format!("Sort by {} range", self.originator.label().to_lowercase()),
        )
    }

    pub fn bottom_view<'a>(&'a self, common: &'a CommonAssets) -> Element<'a, Message> {
        let header = height_row(
            None,
            self.originator.label().to_string(),
            self.min,
            self.max,
            Message::None,
        );

        let slider = container(
            canvas(SliderProgram {
                abs_min: self.abs_min,
                abs_max: self.abs_max,
                min: self.min,
                max: self.max,
                min_text: self.originator.format(self.min),
                max_text: self.originator.format(self.max),
                cache: &self.cache,
            })
            .width(Length::Fixed(480.0))
            .height(Length::Fixed(SLIDER_HEIGHT)),
        )
        .center_x(Length::Fill);

        let footer = row![
            secondary_button("Cancel", Message::Cancel, None, false),
            Space::new().width(Length::FillPortion(3)),
            primary_button("OK", Message::Confirm, false),
        ]
        .width(Length::Fill)
        .spacing(16);

        container(stack![
            Canvas::new(&common.scanlines)
                .width(Length::Fill)
                .height(Length::Fill),
            column![
                row![header,].width(Length::Fixed(300.0)),
                Space::new().height(Length::FillPortion(1)),
                slider,
                Space::new().height(Length::FillPortion(2)),
                footer
            ]
            .spacing(20)
            .align_x(Alignment::Center)
            .padding(20)
            .width(Length::Fill),
        ])
        .style(|_| iced::widget::container::Style {
            background: Some(iced::Background::Color(Color::from_rgb8(140, 213, 229))),
            ..Default::default()
        })
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

const THUMB_WIDTH: f32 = 15.0;
const THUMB_HEIGHT: f32 = 50.0;

const THUMB_BORDER: f32 = 2.0;
// how far a thumb's center must stay from the canvas edge: half its width + half its stroke
const EDGE: f32 = THUMB_WIDTH / 2.0 + THUMB_BORDER / 2.0;

const MIN_GAP: f32 = 0.01;

const MIN_THUMB_COLOR: iced::Color = color!(0x0ec2fe); // #0ec2fe
const MAX_THUMB_COLOR: iced::Color = color!(0x6480E0); // #6480E0

const LABEL_W: f32 = 86.0;
const LABEL_H: f32 = 60.0;
const CONNECTOR: f32 = 14.0;
// label above + connector + handle + connector + label below (+2 so the 2px strokes aren't clipped)
const SLIDER_HEIGHT: f32 = THUMB_HEIGHT + 2.0 * (LABEL_H + CONNECTOR) + 2.0;

struct SliderProgram<'a> {
    abs_min: f32,
    abs_max: f32,
    min: f32,
    max: f32,
    min_text: String,
    max_text: String,
    cache: &'a Cache,
}

impl SliderProgram<'_> {
    fn value_to_x(&self, value: f32, width: f32) -> f32 {
        let usable = width - 2.0 * EDGE;
        let t = (value - self.abs_min) / (self.abs_max - self.abs_min);
        EDGE + t * usable
    }

    fn x_to_value(&self, x: f32, width: f32) -> f32 {
        let usable = width - 2.0 * EDGE;
        let t = ((x - EDGE) / usable).clamp(0.0, 1.0);
        self.abs_min + t * (self.abs_max - self.abs_min)
    }

    fn min_gap(&self, width: f32) -> f32 {
        let usable = width - 2.0 * EDGE;
        // include the border so the two strokes touch rather than overlap
        let thumb_span = (THUMB_WIDTH + THUMB_BORDER) / usable * (self.abs_max - self.abs_min);
        thumb_span.max(MIN_GAP)
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
                let gap = self.min_gap(width);

                let (new_min, new_max) = match thumb {
                    Thumb::Min => (raw.min(self.max - gap), self.max),
                    Thumb::Max => (self.min, raw.max(self.min + gap)),
                };

                let new_min = new_min.clamp(self.abs_min, self.abs_max - gap);
                let new_max = new_max.clamp(self.abs_min + gap, self.abs_max);

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
        const TRACK_THICKNESS: f32 = 24.0;
        const TICK_COUNT: usize = 7;

        let geometry = self
            .cache
            .draw(renderer, bounds.size(), |frame: &mut Frame| {
                let width = frame.width();
                let mid_y = frame.height() / 2.0;
                let cap_inset = TRACK_THICKNESS / 2.0;

                // thumb-center bounds: where min/max sit at abs_min/abs_max
                let track_start = EDGE;
                let track_end = width - EDGE;

                // Empty bar — visible span (incl. round caps) == [track_start, track_end]
                frame.stroke(
                    &Path::line(
                        Point::new(track_start + cap_inset, mid_y),
                        Point::new(track_end - cap_inset, mid_y),
                    ),
                    Stroke::default()
                        .with_width(TRACK_THICKNESS)
                        .with_line_cap(canvas::LineCap::Round)
                        .with_color(iced::color!(0x003469)),
                );

                let track_top = mid_y - TRACK_THICKNESS / 2.0;
                let track_bottom = mid_y + TRACK_THICKNESS / 2.0;

                for i in 1..TICK_COUNT + 1 {
                    let t = i as f32 / (TICK_COUNT + 1) as f32;
                    let x = track_start + t * (track_end - track_start);

                    frame.stroke(
                        &Path::line(Point::new(x, track_top), Point::new(x, track_bottom)),
                        Stroke::default()
                            .with_width(1.0)
                            .with_color(iced::color!(0x2181E4)),
                    );
                }

                let min_x = self.value_to_x(self.min, width);
                let max_x = self.value_to_x(self.max, width);

                // Filled bar — visible span (incl. round caps) == [min_x, max_x]
                // guard against the two thumbs being closer together than the cap inset
                let fill_left = (min_x + cap_inset).min(max_x - cap_inset);
                let fill_right = (max_x - cap_inset).max(min_x + cap_inset);

                frame.stroke(
                    &Path::line(Point::new(fill_left, mid_y), Point::new(fill_right, mid_y)),
                    Stroke::default()
                        .with_width(TRACK_THICKNESS)
                        .with_color(iced::Color::from_rgba8(181, 218, 255, 0.75))
                        .with_line_cap(canvas::LineCap::Round),
                );

                let draw_thumb = |frame: &mut Frame, x: f32, mid_y: f32, color: iced::Color| {
                    let top_left = Point::new(x - THUMB_WIDTH / 2.0, mid_y - THUMB_HEIGHT / 2.0);
                    let size = iced::Size::new(THUMB_WIDTH, THUMB_HEIGHT);
                    let path = Path::rounded_rectangle(top_left, size, (THUMB_WIDTH / 2.0).into());

                    frame.fill(&path, iced::Color { a: 0.35, ..color });
                    frame.stroke(
                        &path,
                        Stroke::default().with_width(THUMB_BORDER).with_color(color),
                    );
                };

                draw_thumb(frame, min_x, mid_y, MIN_THUMB_COLOR);
                draw_thumb(frame, max_x, mid_y, MAX_THUMB_COLOR);

                let thumb_top = mid_y - THUMB_HEIGHT / 2.0;
                let thumb_bottom = mid_y + THUMB_HEIGHT / 2.0;

                let draw_label = |frame: &mut Frame,
                                  thumb_x: f32,
                                  top: f32,
                                  caption: &str,
                                  value: &str,
                                  color: iced::Color| {
                    // keep the box inside the canvas when the handle is near either end
                    let left = (thumb_x - LABEL_W / 2.0).clamp(1.0, width - LABEL_W - 1.0);
                    let path = Path::rounded_rectangle(
                        Point::new(left, top),
                        Size::new(LABEL_W, LABEL_H),
                        10.0.into(),
                    );

                    frame.fill(&path, iced::color!(0x003469));
                    frame.stroke(&path, Stroke::default().with_width(2.0).with_color(color));

                    let cx = left + LABEL_W / 2.0;
                    frame.fill_text(Text {
                        content: caption.to_string(),
                        position: Point::new(cx, top + 14.0),
                        color: color,
                        size: 11.0.into(),
                        align_x: Horizontal::Center.into(),
                        align_y: Vertical::Center,
                        ..Default::default()
                    });
                    frame.fill_text(Text {
                        content: value.to_string(),
                        position: Point::new(cx, top + 37.0),
                        color: iced::Color::WHITE,
                        size: 22.0.into(),
                        align_x: Horizontal::Center.into(),
                        align_y: Vertical::Center,
                        font: iced::Font {
                            weight: iced::font::Weight::Thin,
                            family: common::CONDENSED.family,
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                };

                // MIN: box above the handle, connector runs down to the handle's top edge
                let min_box_top = 1.0;
                frame.stroke(
                    &Path::line(
                        Point::new(min_x, min_box_top + LABEL_H),
                        Point::new(min_x, thumb_top),
                    ),
                    Stroke::default()
                        .with_width(1.0)
                        .with_color(MIN_THUMB_COLOR),
                );
                draw_label(
                    frame,
                    min_x,
                    min_box_top,
                    "MIN",
                    &self.min_text,
                    MIN_THUMB_COLOR,
                );

                // MAX: box below the handle, connector runs down from the handle's bottom edge
                let max_box_top = thumb_bottom + CONNECTOR;
                frame.stroke(
                    &Path::line(
                        Point::new(max_x, thumb_bottom),
                        Point::new(max_x, max_box_top),
                    ),
                    Stroke::default()
                        .with_width(1.0)
                        .with_color(MAX_THUMB_COLOR),
                );
                draw_label(
                    frame,
                    max_x,
                    max_box_top,
                    "MAX",
                    &self.max_text,
                    MAX_THUMB_COLOR,
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
