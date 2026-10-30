use iced::{
    Alignment, Border, Color, ContentFit, Element, Font, Length, Task, Theme,
    font::Weight,
    widget::{
        Canvas, Image, column, container, mouse_area, responsive, row, scrollable,
        scrollable::{Direction, Scrollbar},
        space, stack, svg, text,
    },
};

use crate::{
    elements::{
        common::{
            self, CONDENSED, CommonAssets, action_row, default_shadow, inner_shadow,
            secondary_button,
        },
        icon_button::{IconButtonColors, IconButtonInteraction, IconButtonSize, icon_button},
    },
    enums::IOAction,
    io::GalleryImage,
    screen::info::info::Info,
};

#[derive(Debug)]
pub struct Gallery {
    return_to: Option<Box<Info>>,

    close_icon: svg::Handle,
    close_interaction: IconButtonInteraction,

    selected_image: GalleryImage,
}

#[derive(Clone, Debug)]
pub enum Message {
    IOInput(IOAction),

    Thumbnail,
    Delete,
    Storage,

    CloseInteraction(IconButtonInteraction),

    ImageSelected(GalleryImage),
}

pub enum Action {
    None,
    Run(Task<Message>),
    Return(Box<Info>),
}

impl Gallery {
    pub fn new(return_to: Box<Info>) -> (Self, Task<Message>) {
        let selected_image = return_to.get_gallery_images()[0].clone();
        (
            Self {
                return_to: Some(return_to),

                close_icon: svg::Handle::from_memory(
                    include_bytes!("../../../assets/x.svg").as_slice(),
                ),
                close_interaction: IconButtonInteraction::default(),

                selected_image,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::IOInput(input) => match input {
                _ => Action::None,
            },
            Message::Thumbnail => Action::None,
            Message::Delete => Action::None,
            Message::Storage => Action::None,
            Message::CloseInteraction(i) => {
                if i == IconButtonInteraction::Released {
                    self.close_interaction = IconButtonInteraction::Hovered;
                    match self.return_to.take() {
                        Some(info) => Action::Return(info),
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
                    container(self::gallery_grid(gallery_images))
                        .clip(true)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .padding(12.0)
                        .style(|_: &Theme| container::Style {
                            background: Some(iced::Background::Color(Color::from_rgb8(
                                25, 105, 188
                            ))),
                            border: iced::Border {
                                color: iced::Color::TRANSPARENT,
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
        .style(|_: &Theme| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb8(33, 129, 228))),
            border: iced::Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: 20.0.into(),
            },
            ..Default::default()
        });

        // ====================== button row ======================
        let thumbnail_button = secondary_button(
            "Use as thumbnail".to_string(),
            Message::Thumbnail,
            None,
            false,
        );

        let delete_button = secondary_button("Delete".to_string(), Message::Delete, None, false);

        let storage_button =
            secondary_button("Manage storage".to_string(), Message::Storage, None, false);

        let close = icon_button(
            self.close_icon.clone(),
            IconButtonSize::large(),
            None,
            &self.close_interaction,
            IconButtonColors {
                default_shadow: default_shadow(),
                ..Default::default()
            },
            Message::CloseInteraction,
        );

        let elements = vec![thumbnail_button, delete_button, storage_button, close];

        let row = action_row(elements);

        stack![
            container(
                Canvas::new(&common.scanlines)
                    .width(Length::Fill)
                    .height(Length::Fill)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(140, 213, 229))),
                ..Default::default()
            }),
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
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

const GALLERY_COLUMNS: usize = 2;
const GALLERY_SPACING: f32 = 8.0;

fn gallery_grid<'a>(images: Vec<GalleryImage>) -> Element<'a, Message> {
    responsive(move |size| {
        let scrolls = images.len() > GALLERY_COLUMNS;
        let cols = images.len().clamp(1, GALLERY_COLUMNS);
        let cell_w = (size.width - GALLERY_SPACING * (cols - 1) as f32) / cols as f32;

        let cell = |img: &GalleryImage| -> Element<'a, Message> {
            let mut scale = cell_w / img.width as f32;
            if !scrolls {
                // few images: also constrain by height so they fit without scrolling
                scale = scale.min(size.height / img.height as f32);
            }
            let (w, h) = (img.width as f32 * scale, img.height as f32 * scale);

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
            .width(cell_w)
            .center_x(Length::Fill)
            .into()
        };

        let rows = images.chunks(GALLERY_COLUMNS).map(|chunk| {
            let cells = (0..cols).map(|i| match chunk.get(i) {
                Some(img) => cell(img),
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
                .into()
        } else {
            container(grid).center(Length::Fill).into()
        }
    })
    .into()
}
