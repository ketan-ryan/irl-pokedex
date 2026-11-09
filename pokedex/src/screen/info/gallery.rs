use std::path::Path;

use crate::{
    elements::{
        common::{
            self, CONDENSED, CommonAssets, action_row, default_shadow, inner_shadow,
            secondary_button,
        },
        icon_button::{
            IconButtonColors, IconButtonInteraction, IconButtonSize, icon_button_disableable,
        },
        message_box::Metrics,
        modal::{modal, modal_button_style},
        selectable::{
            SectionShape, SelectionDirection, SelectionGrid, SelectionPosition, is_focused_at,
        },
    },
    enums::IOAction,
    io::{self, GalleryImage},
    screen::info::info::Info,
};
use iced::{
    Alignment, Border, Color, ContentFit, Element, Font, Length, Task, Theme,
    font::Weight,
    widget::{
        Canvas, Id, Image, Space, button, column, container, mouse_area, operation, responsive,
        row, scrollable,
        scrollable::{Direction, Scrollbar},
        space, stack, svg, text,
    },
};
use log::debug;

#[derive(Clone, Debug, PartialEq)]
enum GalleryState {
    Default,
    DeleteConfirmation,
    DeleteError,
}

mod section {
    pub const GALLERY: usize = 0;
    pub const BUTTONS: usize = 1;

    pub mod delete_conf {
        pub const DELETE_CONF: usize = 0;
    }
    pub mod delete_err {
        pub const DELETE_ERR: usize = 0;
    }
}

fn escape(
    _sections: &[SectionShape],
    section: usize,
    _row: usize,
    col: usize,
    direction: SelectionDirection,
) -> Option<SelectionPosition> {
    use SelectionDirection::*;

    let at = |section: usize, row: usize, col: usize| Some(SelectionPosition { section, row, col });

    match (section, direction) {
        // From gallery: Down leads to center button. Everything else nothing
        (s, Up) | (s, Left) | (s, Right) if s == section::GALLERY => None,
        (s, Down) if s == section::GALLERY => at(section::BUTTONS, 0, 0),

        // From button row: All up leads to gallery, left-right as normal, down is nothing
        (s, Up) if s == section::BUTTONS => at(section::GALLERY, 0, 0),
        (s, Left) if s == section::BUTTONS && col == 0 => None,
        (s, Right) if s == section::BUTTONS && col == 3 => None,
        _ => None,
    }
}

fn escape_del(
    _sections: &[SectionShape],
    section: usize,
    _row: usize,
    _col: usize,
    direction: SelectionDirection,
) -> Option<SelectionPosition> {
    use SelectionDirection::*;

    match (section, direction) {
        (s, Up) | (s, Down) if s == section::delete_conf::DELETE_CONF => None,
        _ => None,
    }
}

const GALLERY_COLUMNS: usize = 2;
const GALLERY_SPACING: f32 = 8.0;

fn build_selection_grid(
    gallery_rows: usize,
    gallery_cols: usize,
    state: GalleryState,
) -> SelectionGrid {
    match state {
        GalleryState::Default => SelectionGrid::new(
            vec![
                SectionShape {
                    rows: gallery_rows,
                    cols: gallery_cols,
                },
                SectionShape { rows: 1, cols: 4 }, // buttons
            ],
            section::GALLERY,
            escape,
        ),
        GalleryState::DeleteConfirmation => SelectionGrid::new(
            vec![
                SectionShape { rows: 1, cols: 2 },
                SectionShape { rows: 1, cols: 2 },
            ],
            section::delete_conf::DELETE_CONF,
            escape_del,
        ),
        GalleryState::DeleteError => SelectionGrid::new(
            vec![
                SectionShape { rows: 1, cols: 1 },
                SectionShape { rows: 1, cols: 1 },
            ],
            section::delete_err::DELETE_ERR,
            escape,
        ),
    }
}

fn grid_shape(n: usize) -> (usize, usize) {
    let cols = n.clamp(1, GALLERY_COLUMNS);
    (n.div_ceil(cols), cols)
}

#[derive(Debug)]
pub struct Gallery {
    return_to: Option<Box<Info>>,

    close_icon: svg::Handle,
    close_interaction: IconButtonInteraction,

    selected_image: GalleryImage,

    selection: SelectionGrid,

    state: GalleryState,
    delete_failed_reason: String,
}

#[derive(Clone, Debug)]
pub enum Message {
    IOInput(IOAction),

    Thumbnail,
    DeleteClicked,
    Delete,
    DeleteResult(Result<String, String>),
    Storage,

    CloseInteraction(IconButtonInteraction),

    ImageSelected(GalleryImage),
    CloseModals,
}

pub enum Action {
    None,
    Run(Task<Message>),
    Return(Box<Info>, Task<crate::info::Message>),
}

impl Gallery {
    pub fn new(return_to: Box<Info>) -> (Self, Task<Message>) {
        let images = return_to.get_gallery_images();
        let selected_image = images[0].clone();
        let (rows, cols) = grid_shape(images.len());

        debug!("Gallery of size {}x{}", rows, cols);

        (
            Self {
                return_to: Some(return_to),

                close_icon: svg::Handle::from_memory(
                    include_bytes!("../../../assets/x.svg").as_slice(),
                ),
                close_interaction: IconButtonInteraction::default(),

                selected_image,

                selection: build_selection_grid(rows, cols, GalleryState::Default),

                state: GalleryState::Default,
                delete_failed_reason: "".into(),
            },
            Task::none(),
        )
    }

    fn focused_message(&self) -> Option<Message> {
        let SelectionPosition { section, row, col } = self.selection.position?;
        let state = self.state.clone();
        match (state, section) {
            (GalleryState::Default, section::GALLERY) => self
                .return_to
                .as_ref()?
                .get_gallery_images()
                .get(row * GALLERY_COLUMNS + col)
                .cloned()
                .map(Message::ImageSelected),
            (GalleryState::Default, section::BUTTONS) => match col {
                0 => Some(Message::Thumbnail),
                1 => Some(Message::DeleteClicked),
                2 => Some(Message::Storage),
                3 => Some(Message::CloseInteraction(IconButtonInteraction::Released)),
                _ => None,
            },
            (GalleryState::DeleteConfirmation, section::delete_conf::DELETE_CONF) => match col {
                0 => Some(Message::CloseModals),
                1 => Some(Message::Delete),
                _ => None,
            },
            (GalleryState::DeleteError, section::delete_err::DELETE_ERR) => match col {
                0 => Some(Message::CloseModals),
                _ => None,
            },
            _ => None,
        }
    }

    fn scroll_to_focus(&self) -> Task<Message> {
        let Some(pos) = self.selection.position else {
            return Task::none();
        };
        if pos.section != section::GALLERY {
            return Task::none();
        }
        let count = self
            .return_to
            .as_ref()
            .map_or(0, |i| i.get_gallery_images().len());
        let rows = count.div_ceil(GALLERY_COLUMNS);
        if rows <= 1 {
            return Task::none(); // not scrollable
        }
        let y = pos.row as f32 / (rows - 1) as f32;
        debug!("Scrolling to {}", y);
        operation::snap_to(
            gallery_scroll_id(),
            scrollable::RelativeOffset { x: 0.0, y },
        )
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::IOInput(input) => match input {
                IOAction::Left => {
                    self.selection.navigate(SelectionDirection::Left);
                    Action::Run(self.scroll_to_focus())
                }
                IOAction::Right => {
                    self.selection.navigate(SelectionDirection::Right);
                    Action::Run(self.scroll_to_focus())
                }
                IOAction::ScrollUp => {
                    self.selection.navigate(SelectionDirection::Up);
                    Action::Run(self.scroll_to_focus())
                }
                IOAction::ScrollDown => {
                    self.selection.navigate(SelectionDirection::Down);
                    Action::Run(self.scroll_to_focus())
                }
                IOAction::Select => match self.focused_message() {
                    Some(message) => self.update(message),
                    None => Action::None,
                },
            },

            Message::Thumbnail => Action::None,
            Message::DeleteClicked => {
                if self.state == GalleryState::Default {
                    self.state = GalleryState::DeleteConfirmation;
                    self.selection = build_selection_grid(1, 2, GalleryState::DeleteConfirmation);
                } else {
                    self.state = GalleryState::Default;
                }
                Action::None
            }
            Message::Delete => {
                self.state = GalleryState::Default;
                Action::Run(
                    Task::future(io::delete_dex_img(self.selected_image.path.clone()))
                        .map(|res| Message::DeleteResult(res.map_err(|e| e.to_string()))),
                )
            }
            Message::DeleteResult(res) => {
                if let Err(e) = res {
                    self.delete_failed_reason = e;
                    self.state = GalleryState::DeleteError;
                    self.selection = build_selection_grid(1, 1, GalleryState::DeleteConfirmation);
                    return Action::None;
                }

                let info = self.return_to.as_mut().unwrap();
                let path = res.unwrap();
                let old_idx = info
                    .get_gallery_images()
                    .iter()
                    .position(|i| i.path == path.clone())
                    .unwrap_or(0);
                info.remove_gallery_image(Path::new(&path));

                let images = info.get_gallery_images();
                if images.is_empty() {
                    // nothing left to show; Info already reflects the removal
                    return Action::Return(self.return_to.take().unwrap(), Task::none());
                }

                self.selected_image = images[old_idx.min(images.len() - 1)].clone();

                let (rows, cols) = grid_shape(images.len());
                let pos = self.selection.position;
                self.selection = build_selection_grid(rows, cols, GalleryState::Default);
                self.selection.position = pos; // delete is triggered from the button row, so this stays valid
                Action::None
            }
            Message::Storage => Action::None,
            Message::CloseInteraction(i) => {
                if i == IconButtonInteraction::Released {
                    self.close_interaction = IconButtonInteraction::Hovered;
                    match self.return_to.take() {
                        Some(info) => Action::Return(info, Task::none()),
                        None => Action::None,
                    }
                } else {
                    self.close_interaction = i;
                    Action::None
                }
            }
            Message::ImageSelected(handle) => {
                self.selected_image = handle;
                Action::None
            }
            Message::CloseModals => {
                self.state = GalleryState::Default;
                let info = self.return_to.as_mut().unwrap();
                let (rows, cols) = grid_shape(info.get_gallery_images().len());
                self.selection = build_selection_grid(rows, cols, GalleryState::Default);
                self.delete_failed_reason.clear();

                Action::None
            }
        }
    }

    pub fn top_view<'a>(&'a self, common: &'a CommonAssets) -> Element<'a, Message> {
        let background = container(
            Canvas::new(&common.scanlines)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| iced::widget::container::Style {
            background: Some(iced::Background::Color(common::colors::BACKGROUND)),
            ..Default::default()
        });

        let mut font = Font::with_name("Open Sans SemiBold");
        font.weight = Weight::Semibold;

        let header = container(
            column![
                text!("Pokémon Info")
                    .size(24.0)
                    .color(common::colors::TEXT_DARK)
                    .font(font),
                text!("Gallery view")
                    .size(16.0)
                    .font(CONDENSED)
                    .color(common::colors::TEXT_LIGHT)
            ]
            .spacing(5.0)
            .padding(10.0),
        )
        .width(Length::Fill)
        .height(Length::Fixed(80.0))
        .style(|_| iced::widget::container::Style {
            background: Some(iced::Background::Color(common::colors::CARD_BG)),
            border: Border {
                radius: 10.0.into(),
                width: 1.0,
                color: common::colors::TEXT_DARK,
            },
            shadow: common::default_shadow(),
            ..Default::default()
        });

        let scale = 0.65;
        let selected = Image::new(self.selected_image.handle.clone())
            .width(Length::Fixed(self.selected_image.width as f32 * scale))
            .height(Length::Fixed(self.selected_image.height as f32 * scale))
            .border_radius(20.0)
            .content_fit(iced::ContentFit::Fill);

        stack![
            background,
            column![container(header).padding(20.0)],
            container(selected)
                .center_x(Length::Fill)
                .align_bottom(Length::Fill)
                .padding(iced::Padding::new(0.0).bottom(40.0))
        ]
        .into()
    }

    pub fn bottom_view<'a>(&'a self, common: &'a CommonAssets) -> Element<'a, Message> {
        let focus = self.selection.position();
        let selected_img = self.selected_image.clone();

        let gallery_images = self.return_to.as_ref().unwrap().get_gallery_images();
        let count_str: String = format!("[{}]", gallery_images.len());

        let gallery = container(
            column![
                row![
                    text!("Gallery")
                        .color(iced::Color::WHITE)
                        .size(24.0)
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..iced::Font::default()
                        }),
                    text!("{}", count_str)
                        .color(iced::Color::WHITE)
                        .size(24.0)
                        .font(common::CONDENSED)
                        .width(Length::Fill)
                        .align_x(Alignment::End)
                ]
                .width(Length::Fill)
                .padding(iced::Padding::new(20.0).top(10.0).bottom(5.0)),
                container(inner_shadow(
                    container(self::gallery_grid(
                        gallery_images,
                        focus,
                        selected_img,
                        self.state.clone()
                    ))
                    .clip(true)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(12.0)
                    .style(|_: &Theme| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgb8(25, 105, 188))),
                        border: iced::Border {
                            color: Color::TRANSPARENT,
                            width: 0.0,
                            radius: 20.0.into(),
                        },
                        ..Default::default()
                    }),
                    20.0,
                    8,
                    Color::from_rgba(0.0, 0.0, 0.0, 0.06)
                ))
                .padding(10.0)
            ]
            .width(Length::Fill),
        )
        .width(Length::Fixed(600.0))
        .height(Length::Fixed(360.0))
        .style(move |_: &Theme| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb8(33, 129, 228))),
            border: iced::Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: 20.0.into(),
            },
            ..Default::default()
        });

        // ====================== button row ======================
        let thumbnail_focus = is_focused_at(focus, section::BUTTONS, 0, 0);
        let delete_focus = is_focused_at(focus, section::BUTTONS, 0, 1);
        let storage_focus = is_focused_at(focus, section::BUTTONS, 0, 2);
        let close_focus = is_focused_at(focus, section::BUTTONS, 0, 3);

        let disabled = self.state != GalleryState::Default;

        let thumbnail_button = secondary_button(
            "Use as thumbnail".to_string(),
            Message::Thumbnail,
            None,
            thumbnail_focus,
            disabled,
        );

        let delete_button = secondary_button(
            "Delete".to_string(),
            Message::DeleteClicked,
            None,
            delete_focus,
            disabled,
        );

        let storage_button = secondary_button(
            "Manage storage".to_string(),
            Message::Storage,
            None,
            storage_focus,
            disabled,
        );

        let close = icon_button_disableable(
            self.close_icon.clone(),
            IconButtonSize::large(),
            None,
            &self.close_interaction,
            IconButtonColors {
                default_shadow: default_shadow(),
                ..Default::default()
            },
            Message::CloseInteraction,
            close_focus,
            disabled,
        );

        let elements = vec![thumbnail_button, delete_button, storage_button, close];

        let row = action_row(elements);

        let mut bottom_screen: Vec<Element<Message>> = vec![
            container(
                Canvas::new(&common.scanlines)
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(140, 213, 229))),
                ..Default::default()
            })
            .into(),
        ];
        bottom_screen.push(
            column![
                container(gallery,)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .width(Length::Fill)
                    .height(Length::Fill),
                row,
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        );

        if self.state == GalleryState::DeleteConfirmation {
            bottom_screen.push(
                mouse_area(
                    container(Space::new())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(|_| iced::widget::container::Style {
                            background: Some(iced::Background::Color(Color::from_rgba8(
                                0, 0, 0, 0.6,
                            ))),
                            ..Default::default()
                        }),
                )
                .on_release(Message::CloseModals)
                .into(),
            );

            let cancel_focus = is_focused_at(focus, section::delete_conf::DELETE_CONF, 0, 0);
            let confirm_focus = is_focused_at(focus, section::delete_conf::DELETE_CONF, 0, 1);

            let cancel_button = button("Cancel")
                .padding(10)
                .on_press(Message::DeleteClicked)
                .style(modal_button_style(
                    cancel_focus.then_some(common::colors::SECONDARY_FOCUS),
                ));

            let confirm_button = button("Confirm Deletion")
                .padding(10)
                .on_press(Message::Delete)
                .style(modal_button_style(
                    confirm_focus.then_some(common::colors::SECONDARY_FOCUS),
                ));

            bottom_screen.push(
                container(
                    modal(
                        Some("Delete selected image?".to_string()), 
                        text("Would you like to delete the currently selected image shown on the top screen?".to_string()).into(), 
                        vec![cancel_button, confirm_button],
                        400.0,
                        200.0,
                        Metrics::REFERENCE,
                        None)
                    ).center(Length::Fill)
                    .into()
            );
        } else if self.state == GalleryState::DeleteError {
            bottom_screen.push(
                mouse_area(
                    container(Space::new())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(|_| iced::widget::container::Style {
                            background: Some(iced::Background::Color(Color::from_rgba8(
                                0, 0, 0, 0.6,
                            ))),
                            ..Default::default()
                        }),
                )
                .on_release(Message::CloseModals)
                .into(),
            );
            let confirm_focus = is_focused_at(focus, section::delete_err::DELETE_ERR, 0, 0);
            let confirm_button = button("Ok")
                .padding(10)
                .on_press(Message::CloseModals)
                .style(modal_button_style(
                    confirm_focus.then_some(common::colors::SECONDARY_FOCUS),
                ));

            let display = format!(
                "Failed to delete selected image {}",
                self.delete_failed_reason
            );

            bottom_screen.push(
                container(modal(
                    Some("Deletion Failed".to_string()),
                    text(display).into(),
                    vec![confirm_button],
                    400.0,
                    200.0,
                    Metrics::REFERENCE,
                    None,
                ))
                .center(Length::Fill)
                .into(),
            );
        }

        iced::widget::Stack::with_children(bottom_screen).into()
    }
}

fn gallery_scroll_id() -> Id {
    Id::new("gallery-scroll")
}

fn gallery_grid<'a>(
    images: Vec<GalleryImage>,
    focus: Option<SelectionPosition>,
    selected_img: GalleryImage,
    state: GalleryState,
) -> Element<'a, Message> {
    responsive(move |size| {
        let scrolls = images.len() > GALLERY_COLUMNS;
        let cols = images.len().clamp(1, GALLERY_COLUMNS);
        let cell_w = (size.width - GALLERY_SPACING * (cols - 1) as f32) / cols as f32;

        const BORDER_W: f32 = 4.0;

        let cell = |img: &GalleryImage, r: usize, c: usize| -> Element<'a, Message> {
            // reserve room for the ring so the total still fits the cell
            let avail_w = cell_w - 2.0 * BORDER_W;
            let mut scale = avail_w / img.width as f32;
            if !scrolls {
                scale = scale.min((size.height - 2.0 * BORDER_W) / img.height as f32);
            }
            let (w, h) = (img.width as f32 * scale, img.height as f32 * scale);
            let img_selected = selected_img.handle.id() == img.handle.id();
            let border_color =
                if is_focused_at(focus, section::GALLERY, r, c) && state == GalleryState::Default {
                    common::colors::SECONDARY_FOCUS
                } else if img_selected {
                    common::colors::PRIMARY_FOCUS
                } else {
                    Color::TRANSPARENT
                };
            container(
                container(
                    mouse_area(
                        Image::new(img.handle.clone())
                            .width(w)
                            .height(h)
                            .border_radius(20.0)
                            .content_fit(ContentFit::Fill),
                    )
                    .on_press(Message::ImageSelected(img.clone())),
                )
                .padding(BORDER_W) // keeps the image inside the ring
                .style(move |_| container::Style {
                    border: Border {
                        color: border_color,
                        width: BORDER_W,
                        radius: (20.0 + BORDER_W).into(), // concentric with the image's 20px radius
                    },
                    ..Default::default()
                }),
            )
            .width(cell_w)
            .center_x(Length::Fill)
            .into()
        };

        let rows = images
            .chunks(GALLERY_COLUMNS)
            .enumerate()
            .map(|(r, chunk)| {
                let cells = (0..cols).map(|i| match chunk.get(i) {
                    Some(img) => cell(img, r, i),
                    None => space::horizontal().width(cell_w).into(), // keeps a short last row aligned
                });

                row(cells)
                    .spacing(GALLERY_SPACING)
                    .align_y(iced::Alignment::Center)
                    .into()
            });

        let grid = column(rows).spacing(GALLERY_SPACING).width(Length::Fill);

        if scrolls {
            scrollable(grid)
                .width(Length::Fill)
                .height(Length::Fill)
                .direction(Direction::Vertical(
                    Scrollbar::new()
                        .width(4)
                        .scroller_width(4)
                        .margin(0)
                        .spacing(6),
                ))
                .style(|_theme, status| {
                    let scroller_color = match status {
                        scrollable::Status::Dragged { .. } => Color::from_rgba8(15, 0, 150, 1.0),
                        scrollable::Status::Hovered { .. } => Color::from_rgba8(15, 0, 150, 0.9),
                        _ => Color::from_rgba8(15, 0, 150, 0.6),
                    };
                    scrollable::Style {
                        container: Default::default(),
                        vertical_rail: scrollable::Rail {
                            background: Some(iced::Background::Color(Color::from_rgba8(
                                0, 0, 0, 0.1,
                            ))),
                            border: Border {
                                radius: 4.0.into(),
                                width: 0.0,
                                color: Color::TRANSPARENT,
                            },
                            scroller: scrollable::Scroller {
                                background: iced::Background::Color(scroller_color),
                                border: Border {
                                    radius: 4.0.into(),
                                    width: 0.0,
                                    color: Color::TRANSPARENT,
                                },
                            },
                        },
                        horizontal_rail: scrollable::Rail {
                            background: None,
                            border: Border::default(),
                            scroller: scrollable::Scroller {
                                background: iced::Background::Color(Color::TRANSPARENT),
                                border: Border::default(),
                            },
                        },
                        gap: None,
                        auto_scroll: scrollable::AutoScroll {
                            background: iced::Background::Color(Color::TRANSPARENT),
                            border: Border::default(),
                            shadow: Default::default(),
                            icon: Color::TRANSPARENT,
                        },
                    }
                })
                .id(gallery_scroll_id())
                .into()
        } else {
            container(grid).center(Length::Fill).into()
        }
    })
    .into()
}
