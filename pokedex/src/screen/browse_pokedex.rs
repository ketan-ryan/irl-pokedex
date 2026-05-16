use std::sync::{Arc, RwLock as StdRwLock};
use std::time::Duration;
use std::{collections::HashMap, time::Instant};

use iced::advanced::graphics::core::widget;
use iced::animation::Animation;
use iced::event::{self, Status};
use iced::keyboard::{Event::KeyPressed, Key, key::Named};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Id, Scrollable, mouse_area, operation, stack};
use iced::{
    Alignment, Border, Color, Element, Event, Length, Padding, Subscription, Task,
    widget::{Space, canvas, column, container, image, image::Handle, row, scrollable, svg, text},
    window,
};

use log::{debug, error, info, trace};

use crate::elements::registered_icon::{IconState, RegisteredIconWidget};
use crate::screen::register;
use crate::{
    elements::grid::Grid,
    io::{self, PokedexConfig, PokemonInfo},
};

#[derive(Clone, Debug)]
struct Selected {
    selected_pokemon: Option<String>,
    selected_idx: Option<usize>,
    previously_selected: Option<String>,
}

impl Selected {
    pub fn new() -> Self {
        Self {
            selected_pokemon: None,
            selected_idx: None,
            previously_selected: None,
        }
    }
}

#[derive(Debug)]
pub struct PokedexBrowser {
    config: Arc<PokedexConfig>,
    grid: Grid,
    last_tick: Instant,
    pokemon_data: HashMap<String, PokemonInfo>,
    owned_pokemon: std::collections::HashSet<String>,
    image_cache: ImageCache,
    pokeball_handle: Handle,
    info_svg: svg::Handle,

    // scroll params
    scroll_offset: f32,
    items_per_page: usize,
    top_scroll_id: widget::Id,
    bot_scroll_id: widget::Id,
    selected: Selected,
    scroll_animation: Option<Animation<f32>>,
    // used for animation
    current_scroll_offset: f32,
    target_scroll_offset: f32,
    size_animation: Animation<f32>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick(std::time::Instant),
    Scrolled(scrollable::Viewport),
    ImageLoaded(String, Handle),
    ImageLoadFailed(String),
    IOInput(IOAction),
    SelectPokemon(String),
    AnimateScroll,
}

pub enum Action {
    None,
    GoHome,
    Run(Task<Message>),
}

#[derive(Debug, Clone)]
pub enum IOAction {
    ScrollUp,
    ScrollDown,
    Left,
    Right,
}

const TOP_SCREEN_ITEMS: usize = 8;
const ROW_HEIGHT: f32 = 40.0;

impl PokedexBrowser {
    pub fn new(
        config: Arc<PokedexConfig>,
        pokemon_data: HashMap<String, PokemonInfo>,
        owned_pokemon: std::collections::HashSet<String>,
    ) -> (Self, Task<Message>) {
        let mut pokemon_names: Vec<String> = pokemon_data.keys().cloned().collect();

        pokemon_names.retain(|name| config.classes.contains(name));

        // Sort by pokemon number for better ordering
        pokemon_names.sort_by_key(|name| {
            pokemon_data
                .get(name)
                .and_then(|info| info.number.parse::<u32>().ok())
                .unwrap_or(9999)
        });

        let selected_idx = 5;
        let selected_pokemon = pokemon_names.get(selected_idx).cloned();

        let mut image_cache = ImageCache::new(pokemon_names, 15);

        let load_task =
            image_cache.update_visible_range(config.as_ref().sprites_location.clone(), 0, 20);

        let state = Self {
            config,
            grid: Grid::new(),
            last_tick: Instant::now(),
            pokemon_data,
            owned_pokemon,
            image_cache,
            scroll_offset: 0.0,
            items_per_page: 10,
            top_scroll_id: Id::unique(),
            bot_scroll_id: Id::unique(),
            pokeball_handle: Handle::from_bytes(
                include_bytes!("../../assets/background.png").as_slice(),
            ),
            info_svg: svg::Handle::from_memory(
                include_bytes!("../../assets/browse_screen/hint.svg").as_slice(),
            ),
            selected: Selected {
                selected_pokemon,
                selected_idx: Some(selected_idx),
                previously_selected: None,
            },
            scroll_animation: None,
            current_scroll_offset: 0.0,
            target_scroll_offset: 0.0,
            size_animation: Animation::new(1.0)
                .duration(Duration::from_millis(200))
                .easing(iced::animation::Easing::EaseInOut),
        };

        (state, load_task)
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::Tick(now) => {
                let dt = now - self.last_tick;
                self.last_tick = now;

                self.grid.tick(dt);

                Action::None
            }
            Message::Scrolled(viewport) => {
                // Bottom scrollable's absolute position represents how far we've scrolled
                let bot_scroll_pos = viewport.absolute_offset().y;

                // Top screen shows items that have scrolled past the top of bottom screen
                // When bot_scroll_pos = 0, top shows nothing (scroll position irrelevant)
                // When bot_scroll_pos = 450 (10 items), top should show items 0-9
                // The top scrollable should be at position 0 when it starts showing content
                let top_scroll_pos =
                    (bot_scroll_pos - ROW_HEIGHT * TOP_SCREEN_ITEMS as f32).max(0.0);

                // Store for rendering calculations
                self.scroll_offset = bot_scroll_pos;

                // Synchronize top scrollable
                let sync_task = operation::scroll_to(
                    self.top_scroll_id.clone(),
                    scrollable::AbsoluteOffset {
                        x: 0.0,
                        y: top_scroll_pos,
                    },
                );

                // Update visible range for image loading
                // Start from the top of bottom screen
                let start_index = (bot_scroll_pos / ROW_HEIGHT).floor() as usize;
                let end_index = start_index + (self.items_per_page * 2);
                let load_task = self.image_cache.update_visible_range(
                    self.config.sprites_location.clone(),
                    start_index.saturating_sub(TOP_SCREEN_ITEMS),
                    end_index,
                );

                Action::Run(Task::batch(vec![sync_task, load_task]))
            }
            Message::ImageLoaded(name, handle) => {
                // debug!("Loaded {}", name);
                self.image_cache.insert(name, handle);
                Action::None
            }
            Message::ImageLoadFailed(name) => {
                error!("Failed to load image for: {}", name);
                Action::None
            }
            Message::IOInput(action) => {
                let current_index = self
                    .selected
                    .selected_pokemon
                    .as_ref()
                    .and_then(|name| {
                        self.image_cache
                            .pokemon_order
                            .iter()
                            .position(|n| n == name)
                    })
                    .unwrap_or(0);

                let new_index = match action {
                    IOAction::ScrollUp => current_index.saturating_sub(1),
                    IOAction::ScrollDown => {
                        (current_index + 1).min(self.image_cache.pokemon_order.len() - 1)
                    }
                    IOAction::Left => current_index.saturating_sub(10),
                    IOAction::Right => {
                        (current_index + 10).min(self.image_cache.pokemon_order.len() - 1)
                    }
                };

                if new_index != current_index {
                    let new_name = self.image_cache.pokemon_order[new_index].clone();
                    debug!("Scrolled to select new pokemon {}", new_name);
                    return Action::Run(Task::done(Message::SelectPokemon(new_name)));
                    // self.selected.selected_pokemon = Some(new_name);
                    // self.start_scroll_animation(new_index)
                }

                Action::None
            }
            Message::SelectPokemon(name) => {
                self.selected.previously_selected = self.selected.selected_pokemon.clone();
                self.selected.selected_pokemon = Some(name.clone());

                debug!("Selected pokemon {}", name);

                if let Some(index) = self
                    .image_cache
                    .pokemon_order
                    .iter()
                    .position(|n| n == &name)
                {
                    self.start_scroll_animation(index);
                }

                Action::None
            }
            Message::AnimateScroll => {
                if let Some(animation) = &mut self.scroll_animation {
                    if animation.interpolate_with(|v| v, Instant::now())
                        >= self.target_scroll_offset
                    {
                        self.scroll_animation = None;
                        self.current_scroll_offset = self.target_scroll_offset;

                        return Action::Run(iced::widget::operation::scroll_to(
                            self.bot_scroll_id.clone(),
                            scrollable::AbsoluteOffset {
                                x: 0.0,
                                y: self.current_scroll_offset,
                            },
                        ));
                    } else {
                        // Get the animated value
                        self.current_scroll_offset =
                            animation.interpolate_with(|v| v, Instant::now());
                    }

                    Action::Run(iced::widget::operation::scroll_to(
                        self.bot_scroll_id.clone(),
                        scrollable::AbsoluteOffset {
                            x: 0.0,
                            y: self.current_scroll_offset,
                        },
                    ))
                } else {
                    Action::None
                }
            }
        }
    }

    fn start_scroll_animation(&mut self, index: usize) {
        info!("Start scroll called");
        self.target_scroll_offset = index.saturating_sub(5) as f32 * ROW_HEIGHT;
        self.scroll_animation = Some(
            Animation::new(self.current_scroll_offset)
                .duration(Duration::from_millis(100))
                .easing(iced::animation::Easing::EaseOutCubic),
        );
        self.scroll_animation
            .as_mut()
            .unwrap()
            .go_mut(self.target_scroll_offset, Instant::now());
        self.size_animation = Animation::new(0.0)
            .duration(Duration::from_millis(100))
            .easing(iced::animation::Easing::EaseInOut);
        self.size_animation.go_mut(1.0, Instant::now());
    }

    fn scroll_to_center_item(&self, index: usize) -> Task<Message> {
        let scroll_offset = index.saturating_sub(5) as f32 * ROW_HEIGHT;

        iced::widget::operation::scroll_to(
            self.bot_scroll_id.clone(),
            scrollable::AbsoluteOffset {
                x: 0.0,
                y: scroll_offset,
            },
        )
    }

    fn scroll(&self, amount: f32) -> Task<Message> {
        // Only scroll the bottom scrollable - the Scrolled message will sync the top
        iced::widget::operation::scroll_by(
            self.bot_scroll_id.clone(),
            scrollable::AbsoluteOffset { x: 0.0, y: amount },
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // tick screen for updates ~60fps
        // time::every(Duration::from_millis(16)).map(Message::Tick)

        let mut subscriptions = Vec::new();
        subscriptions.push(window::frames().map(Message::Tick));

        // TODO: Will need custom subscription / event to handle rpi IO
        subscriptions.push(event::listen_with(|event, status, _| {
            match (event, status) {
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
            }
        }));

        if self.scroll_animation.is_some() {
            subscriptions
                .push(iced::time::every(Duration::from_millis(16)).map(|_| Message::AnimateScroll));
        }

        Subscription::batch(subscriptions)
    }

    pub fn top_view(&self) -> Element<'_, Message> {
        let semibold = iced::Font::with_name("Open Sans Semibold");
        let condensed = iced::Font::with_name("Open Sans Condensed");

        // scroll_offset now represents the bottom screen's scroll position
        // Top screen shows items that have scrolled past the top of bottom screen
        let bot_start_index = (self.scroll_offset / ROW_HEIGHT).floor() as usize;

        // Top screen shows items before bot_start_index
        // But only up to TOP_SCREEN_ITEMS worth
        let items_to_show = bot_start_index.min(TOP_SCREEN_ITEMS);
        let top_start_index = bot_start_index.saturating_sub(items_to_show);

        // Build the items for top screen
        let items: Vec<Element<Message>> = self
            .image_cache
            .pokemon_order
            .iter()
            .enumerate()
            .skip(top_start_index)
            .take(items_to_show)
            .map(|(idx, name)| {
                // TOP_SCREEN_ITEMS - 1 is bottom of screen, 0 is top
                // values don't matter as long as it's consistent regardless of
                // if the top screen is full or not yet
                let screen_pos = if items_to_show < TOP_SCREEN_ITEMS {
                    idx - top_start_index + (TOP_SCREEN_ITEMS - items_to_show)
                } else {
                    idx - top_start_index
                };

                // lerp opacity between 0.2 and 0.8
                let opacity =
                    0.2 + (screen_pos as f32 / (TOP_SCREEN_ITEMS - 1) as f32) * (0.8 - 0.2);

                let info = self.pokemon_data.get(name).unwrap();
                let is_owned = self.owned_pokemon.contains(name);
                self.render_pokemon_item(name, info, is_owned, opacity, false, false, true)
            })
            .collect();

        let num_items = items.len();
        let mut content = column(items).spacing(5);

        // If we have fewer than TOP_SCREEN_ITEMS, add spacer at top to push content to bottom
        // items, row height, padding
        let mut buffer = (TOP_SCREEN_ITEMS - num_items) as f32 * 45.0 + 10.0;
        if num_items >= TOP_SCREEN_ITEMS {
            buffer = 10.0;
        }
        content = column![
            Space::new()
                .width(Length::Fill)
                .height(Length::Fixed(buffer)),
            content
        ];

        let mut elements: Vec<Element<Message>> = vec![];
        // scanlines
        elements.push(
            container(
                canvas::Canvas::new(&self.grid)
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
        );

        // background pokeball
        elements.push(
            column![
                // push it down a bit for visual rather than true centering
                Space::new().height(Length::Fixed(50.0)),
                image(self.pokeball_handle.clone()).opacity(0.2).scale(0.95)
            ]
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .into(),
        );

        // bottom darker blue bit
        elements.push(
            column![
                Space::new()
                    .width(Length::Fill)
                    .height(Length::FillPortion(9)),
                container(Space::new())
                    .width(Length::Fill)
                    .height(Length::Fixed(40.0))
                    .style(|_| iced::widget::container::Style {
                        background: Some(iced::Background::Color(Color::from_rgba8(
                            179, 206, 255, 0.6,
                        ))),
                        ..Default::default()
                    })
            ]
            .into(),
        );

        let body = stack![
            container(column![
                // header
                container(
                    column![
                        row![text("National Pokédex").font(semibold).size(24.0)]
                            .height(Length::FillPortion(2)),
                        row![
                            column![
                                row![
                                    text("Registered").font(condensed).size(16.0),
                                    text("0541").font(condensed).size(16.0)
                                ]
                                .spacing(10.0)
                            ],
                            column![
                                row![
                                    text("Total").font(condensed).size(16.0),
                                    text("1160").font(condensed).size(16.0)
                                ]
                                .spacing(10.0)
                            ]
                        ]
                        .spacing(200.0)
                        .width(Length::Fill)
                        .height(Length::FillPortion(1))
                    ]
                    .spacing(10.0)
                )
                .width(Length::Fill)
                .height(Length::FillPortion(2))
                .padding(Padding {
                    left: 20.0,
                    right: 20.0,
                    bottom: 20.0,
                    top: 10.0
                })
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgba8(
                        255, 255, 255, 0.7
                    ))),
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 1.0,
                        radius: 24.0.into()
                    },
                    text_color: Some(Color::from_rgb8(24, 103, 184)),
                    ..Default::default()
                }),
                // the list that gets pushed up
                Scrollable::new(content)
                    .direction(Direction::Vertical(Scrollbar::hidden()))
                    .id(self.top_scroll_id.clone())
                    .width(Length::Fill)
                    .height(Length::FillPortion(9))
            ])
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(Color::TRANSPARENT)),
                ..Default::default()
            })
            .padding(iced::Padding {
                top: 10.0,
                bottom: 20.0,
                right: 10.0,
                left: 10.0,
                ..Default::default()
            })
        ]
        .into();
        elements.push(body);
        iced::widget::Stack::with_children(elements).into()
    }

    pub fn bottom_view(&self) -> Element<'_, Message> {
        let item_buffer = 10.0;
        let items: Vec<Element<Message>> = self
            .image_cache
            .pokemon_order
            .iter()
            .enumerate()
            .filter_map(|(index, name)| {
                let info = self.pokemon_data.get(name).unwrap();
                let is_owned = self.owned_pokemon.contains(name);
                let selected = self.selected.selected_pokemon.as_ref() == Some(name);
                let was_selected = self.selected.previously_selected.as_ref() == Some(name);
                Some(self.render_pokemon_item(
                    name,
                    info,
                    is_owned,
                    0.9,
                    selected,
                    was_selected,
                    false,
                ))
            })
            .collect();

        let mut elements: Vec<Element<Message>> = vec![];

        // scanlines
        elements.push(
            container(
                canvas::Canvas::new(&self.grid)
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
        );

        // top darker blue bit
        elements.push(
            stack!(
                column![
                    container(Space::new())
                        .width(Length::Fill)
                        .height(Length::FillPortion(1))
                        .style(|_| iced::widget::container::Style {
                            background: Some(iced::Background::Color(Color::from_rgba8(
                                179, 206, 255, 0.6,
                            ))),
                            ..Default::default()
                        }),
                    container(row![])
                        .style(|_| iced::widget::container::Style {
                            border: Border {
                                radius: 12.0.into(),
                                width: 1.0,
                                color: Color::WHITE,
                            },
                            background: Some(iced::Background::Color(Color::from_rgba8(
                                33, 129, 228, 0.9
                            ))),
                            ..Default::default()
                        })
                        .width(Length::Fill)
                        .height(Length::FillPortion(4)),
                    Space::new()
                        .width(Length::Fill)
                        .height(Length::FillPortion(6))
                ],
                row![
                    container(svg(self.info_svg.clone()))
                        .align_left(Length::Fixed(256.0))
                        .padding(Padding {
                            left: 10.0,
                            top: 10.0,
                            ..Default::default()
                        })
                ]
                .width(Length::Fill)
                .height(Length::Fill)
            )
            .into(),
        );

        // list
        let content = column(items).spacing(5);
        let body = stack![
            container(
                scrollable(content)
                    .id(self.bot_scroll_id.clone())
                    .on_scroll(Message::Scrolled)
                    .height(Length::Fill),
            )
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(Color::TRANSPARENT)),
                ..Default::default()
            })
            .padding(iced::Padding {
                top: 10.0,
                bottom: 20.0,
                right: 10.0,
                ..Default::default()
            })
        ]
        .into();
        elements.push(body);
        iced::widget::Stack::with_children(elements).into()
    }

    fn render_pokemon_item(
        &'_ self,
        name: &str,
        info: &PokemonInfo,
        is_owned: bool,
        opacity: f32,
        selected: bool,
        was_selected: bool,
        is_top_screen: bool,
    ) -> Element<'_, Message> {
        const IMG_SIZE: f32 = 20.0;
        let mut item_row = row!().spacing(1.5).align_y(iced::Alignment::Center);

        // Add image or placeholder
        if let Some(handle) = self.image_cache.get(name) {
            item_row = item_row.push(
                image(handle)
                    .width(IMG_SIZE)
                    .height(IMG_SIZE)
                    .opacity(opacity),
            );
        } else {
            // Show placeholder
            item_row = item_row.push(Space::new().width(IMG_SIZE).height(IMG_SIZE));
        }

        // Owned indicator
        let state = if is_owned {
            IconState::Registered
        } else {
            IconState::Unregistered
        };
        let shape = RegisteredIconWidget::new(state, opacity);

        item_row = item_row.push(container(
            canvas::Canvas::new(shape)
                .width(Length::Fixed(20.0))
                .height(Length::Fixed(20.0)),
        ));

        // Pokemon info
        let info_column = column![
            text(format!(
                "{}\t{}",
                info.number,
                register::to_proper_case(name)
            ))
            .size(14)
            .color(Color::from_rgba(0.0, 0.0, 0.0, opacity)),
        ]
        .spacing(2);

        item_row = item_row.push(info_column);
        let size_now = self.size_animation.interpolate_with(|v| v, Instant::now());

        let color = if selected {
            Color::from_rgba(0.2, 0.8, 0.2, size_now)
        } else if was_selected {
            Color::from_rgba(0.2, 0.8, 0.2, 1.0 - size_now)
        } else {
            Color::from_rgba(1.0, 1.0, 1.0, opacity)
        };

        let name_ = name.to_string();

        let size = if selected {
            35.0 + (10.0 * size_now)
        } else if was_selected {
            45.0 - (10.0 * size_now)
        } else {
            35.0
        };

        let area = mouse_area(
            container(item_row)
                .padding(10)
                .width(Length::FillPortion(5))
                .height(Length::Fixed(size))
                .style(move |_| container::Style {
                    background: Some(color.into()),
                    border: Border {
                        radius: 12.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                    text_color: Some(Color::from_rgba(0.0, 0.0, 0.0, opacity)),
                    ..Default::default()
                }),
        );

        // only bottom screen should be clickables
        let area = if !is_top_screen {
            area.on_press(Message::SelectPokemon(name_))
        } else {
            area
        };

        row![Space::new().width(Length::FillPortion(4)), area].into()
    }
}

#[derive(Debug)]
pub struct ImageCache {
    // Sync cache for rendering (can be accessed without await)
    sync_cache: Arc<StdRwLock<HashMap<String, Handle>>>,
    // Ordered list of all pokemon names
    pokemon_order: Vec<String>,
    // Current visible range
    visible_start: usize,
    visible_end: usize,
    // Buffer size
    buffer_size: usize,
    // Track which images are currently being loaded
    loading: Arc<StdRwLock<std::collections::HashSet<String>>>,
}

impl ImageCache {
    pub fn new(pokemon_names: Vec<String>, buffer_size: usize) -> Self {
        Self {
            sync_cache: Arc::new(StdRwLock::new(HashMap::new())),
            pokemon_order: pokemon_names,
            visible_start: 0,
            visible_end: 0,
            buffer_size,
            loading: Arc::new(StdRwLock::new(std::collections::HashSet::new())),
        }
    }

    /// Update the visible range and return commands to load new images
    pub fn update_visible_range(
        &mut self,
        sprite_folder: String,
        start: usize,
        end: usize,
    ) -> iced::Task<Message> {
        self.visible_start = start;
        self.visible_end = end;

        trace!("Starting at: {}, ending at: {}", start, end);

        let load_start = start.saturating_sub(self.buffer_size);
        let load_end = (end + self.buffer_size).min(self.pokemon_order.len());

        trace!("Load start: {}, load end: {}", load_start, load_end);

        // Cleanup old images
        {
            let mut cache = self.sync_cache.write().unwrap();
            let pokemon_order = &self.pokemon_order;
            cache.retain(|name, _| {
                if let Some(index) = pokemon_order.iter().position(|n| n == name) {
                    // trace!("retaining {}", name);
                    index >= load_start && index < load_end
                } else {
                    false
                }
            });
        }

        trace!("Cleaned up old images");

        // Separate visible and buffer images
        let mut visible_commands = Vec::new();
        let mut buffer_commands = Vec::new();

        for i in load_start..load_end {
            let name = self.pokemon_order[i].clone();

            // Skip if already loaded or loading
            let should_load = {
                let cache = self.sync_cache.read().unwrap();
                let loading = self.loading.read().unwrap();
                !cache.contains_key(&name) && !loading.contains(&name)
            };

            if should_load {
                let load_task = self.load_image_async(sprite_folder.clone(), name);

                // Prioritize visible range
                if i >= start && i < end {
                    visible_commands.push(load_task);
                } else {
                    buffer_commands.push(load_task);
                }
            }
        }

        // Load visible images first, then buffer images
        iced::Task::batch(
            visible_commands
                .into_iter()
                .chain(buffer_commands.into_iter()),
        )
    }

    /// Asynchronously load an image
    fn load_image_async(&self, sprite_folder: String, pokemon_name: String) -> iced::Task<Message> {
        let loading = self.loading.clone();

        // Mark as loading
        {
            let mut loading_set = loading.write().unwrap();
            loading_set.insert(pokemon_name.clone());
        }

        iced::Task::perform(
            async move {
                let result = io::load_png(sprite_folder, &pokemon_name.to_lowercase());
                (pokemon_name, result)
            },
            move |(name, bytes)| {
                // Remove from loading set
                let mut loading_set = loading.write().unwrap();
                loading_set.remove(&name);

                if bytes.is_ok() {
                    Message::ImageLoaded(name, Handle::from_bytes(bytes.unwrap()))
                } else {
                    Message::ImageLoadFailed(name)
                }
            },
        )
    }

    /// Get handle for a specific pokemon (synchronous for rendering)
    pub fn get(&self, pokemon_name: &str) -> Option<Handle> {
        let cache = self.sync_cache.read().unwrap();
        cache.get(pokemon_name).cloned()
    }

    /// Store a loaded image
    pub fn insert(&self, name: String, handle: Handle) {
        let mut cache = self.sync_cache.write().unwrap();
        cache.insert(name, handle);
    }

    /// Get current cache size
    pub fn cache_size(&self) -> usize {
        self.sync_cache.read().unwrap().len()
    }

    /// Check if an image is currently loading
    pub fn is_loading(&self, pokemon_name: &str) -> bool {
        self.loading.read().unwrap().contains(pokemon_name)
    }
}
