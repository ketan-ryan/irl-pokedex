use std::sync::Arc;
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

use crate::{
    elements::{
        grid::Grid,
        icon_button::{IconButtonColors, IconButtonInteraction, icon_button},
        image_cache::ImageCache,
        registered_icon::{IconState, RegisteredIconWidget},
    },
    io::{PokedexConfig, PokemonInfo},
    screen::register,
};

#[derive(Clone, Debug)]
struct Selected {
    selected_pokemon: Option<String>,
    selected_idx: Option<usize>,
    previously_selected: Option<String>,
    selected_slot: usize,
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
    selected_com_offset: Option<f32>,
    scroll_animation: Option<Animation<f32>>,

    // used for animation
    current_scroll_offset: f32,
    target_scroll_offset: f32,
    size_animation: Animation<f32>,

    // scroll-based image loading with 200ms debounce
    last_scroll_time: Option<Instant>,
    pending_scroll_load: Option<(usize, usize)>,
    last_load_dispatch_time: Option<Instant>,

    search_icon: svg::Handle,
    filter_icon: svg::Handle,
    close_icon: svg::Handle,

    search_interaction: IconButtonInteraction,
    filter_interaction: IconButtonInteraction,
    close_interaction: IconButtonInteraction,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick(std::time::Instant),
    Scrolled(scrollable::Viewport),
    ImageLoaded(String, Handle, f32, u64),
    ImageCenterOfMass(String, f32, u64),
    ImageLoadFailed(String, u64),
    IOInput(IOAction),
    SelectPokemon(String, bool),
    AnimateScroll,
    SearchInteraction(IconButtonInteraction),
    FilterInteraction(IconButtonInteraction),
    CloseInteraction(IconButtonInteraction),
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

/// Computes the first visible row to show when a selection moves off-screen.
///
/// # Arguments
/// * `selected_index` - The index of the newly selected Pokémon.
/// * `selected_slot` - The current row position of the selection within the viewport.
/// * `items_per_page` - The number of visible rows in the list.
/// * `current_first_visible` - The first visible row before the new selection.
/// * `preserve_relative_slot` - Whether the selection should keep the same row position when scrolling.
///
/// # Returns
/// The first row index that should be visible after the selection change.
fn calculate_scroll_target_for_selection(
    selected_index: usize,
    selected_slot: usize,
    items_per_page: usize,
    current_first_visible: usize,
    preserve_relative_slot: bool,
) -> usize {
    if preserve_relative_slot {
        let last_visible = current_first_visible.saturating_add(items_per_page.saturating_sub(1));
        if selected_index < current_first_visible || selected_index > last_visible {
            selected_index.saturating_sub(selected_slot.min(items_per_page.saturating_sub(1)))
        } else {
            current_first_visible
        }
    } else if selected_index < current_first_visible {
        selected_index
    } else {
        let last_visible = current_first_visible.saturating_add(items_per_page.saturating_sub(1));
        if selected_index > last_visible {
            selected_index.saturating_sub(items_per_page.saturating_sub(1))
        } else {
            current_first_visible
        }
    }
}

impl PokedexBrowser {
    pub fn new(
        config: Arc<PokedexConfig>,
        pokemon_data: HashMap<String, PokemonInfo>,
        owned_pokemon: std::collections::HashSet<String>,
    ) -> (Self, Task<Message>) {
        let mut pokemon_names: Vec<String> = pokemon_data.keys().cloned().collect();

        // TODO: proper filtering
        pokemon_names.retain(|name| {
            pokemon_data.get(name).unwrap().base.is_none_or(|base| base) && !name.contains("mega ")
        });

        pokemon_names = pokemon_names
            .iter()
            .map(|name| {
                // Some pokemon have different names in the class map and pokedex json,
                // depending on how they were generated
                let mapped: Option<&String> = config.name_maps.get(name);
                if mapped.is_some() {
                    mapped.unwrap()
                } else {
                    name
                }
                .to_lowercase()
            })
            .collect();

        // Sort by pokemon number for better ordering
        pokemon_names.sort_by_key(|name| {
            pokemon_data
                .get(name)
                .and_then(|info| info.number.parse::<u32>().ok())
                .unwrap_or(9999)
        });

        let selected_idx = 5;
        let selected_pokemon = pokemon_names.get(selected_idx).cloned();

        let mut image_cache = ImageCache::new(pokemon_names, 25);

        let load_task = image_cache.update_visible_range(
            config.as_ref().sprites_location.clone(),
            0,
            20,
            Some(selected_idx),
        );

        let state = Self {
            config,
            grid: Grid::new(),
            last_tick: Instant::now(),
            pokemon_data,
            owned_pokemon,
            image_cache,

            // this offset is used for calculations
            scroll_offset: 0.0,
            items_per_page: 11,
            top_scroll_id: Id::unique(),
            bot_scroll_id: Id::unique(),

            pokeball_handle: Handle::from_bytes(
                include_bytes!("../../assets/background.png").as_slice(),
            ),
            info_svg: svg::Handle::from_memory(
                include_bytes!("../../assets/browse_screen/hint.svg").as_slice(),
            ),
            search_icon: svg::Handle::from_memory(
                include_bytes!("../../assets/browse_screen/search.svg").as_slice(),
            ),
            filter_icon: svg::Handle::from_memory(
                include_bytes!("../../assets/browse_screen/filter.svg").as_slice(),
            ),
            close_icon: svg::Handle::from_memory(
                include_bytes!("../../assets/browse_screen/x.svg").as_slice(),
            ),
            selected: Selected {
                selected_pokemon,
                selected_idx: Some(selected_idx),
                previously_selected: None,
                selected_slot: 5,
            },
            selected_com_offset: None,
            scroll_animation: None,
            // these offsets are used for animations
            current_scroll_offset: 0.0,
            target_scroll_offset: 0.0,

            size_animation: Animation::new(1.0)
                .duration(Duration::from_millis(200))
                .easing(iced::animation::Easing::EaseInOut),
            last_scroll_time: None,
            pending_scroll_load: None,
            last_load_dispatch_time: None,

            search_interaction: IconButtonInteraction::default(),
            filter_interaction: IconButtonInteraction::default(),
            close_interaction: IconButtonInteraction::default(),
        };

        (state, load_task)
    }

    /// Returns the first and last visible list indices for a given scroll offset.
    ///
    /// # Arguments
    /// * `scroll_offset` - The vertical scroll position of the bottom list in pixels.
    ///
    /// # Returns
    /// A tuple containing the first and last visible row indices.
    fn visible_range(&self, scroll_offset: f32) -> (usize, usize) {
        let first_visible = (scroll_offset / ROW_HEIGHT).floor() as usize;
        let last_visible = first_visible.saturating_add(self.items_per_page.saturating_sub(1));
        (first_visible, last_visible)
    }

    /// Updates the selected slot and scroll target for a newly chosen Pokémon index.
    ///
    /// # Arguments
    /// * `index` - The Pokémon index to select.
    /// * `preserve_slot` - Whether to keep the current relative slot position when scrolling.
    fn update_selection_for_index(&mut self, index: usize, preserve_slot: bool) {
        let current_slot = self.selected.selected_slot;
        let (first_visible, last_visible) = self.visible_range(self.scroll_offset);

        let target_first = if preserve_slot {
            calculate_scroll_target_for_selection(
                index,
                current_slot,
                self.items_per_page,
                first_visible,
                true,
            )
        } else if index < first_visible {
            index
        } else if index > last_visible {
            index.saturating_sub(self.items_per_page.saturating_sub(1))
        } else {
            first_visible
        };

        if index < first_visible || index > last_visible {
            self.selected.selected_slot = if preserve_slot {
                current_slot
            } else if index < first_visible {
                0
            } else {
                self.items_per_page.saturating_sub(1)
            };
            self.start_scroll_animation_to(target_first as f32 * ROW_HEIGHT);
        } else if preserve_slot {
            self.selected.selected_slot = current_slot;
        } else {
            self.selected.selected_slot = index.saturating_sub(first_visible);
        }
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::Tick(now) => {
                let dt = now - self.last_tick;
                self.last_tick = now;

                self.grid.tick(dt);

                // Check if we have a pending scroll load and 200ms has elapsed
                if let Some((start_index, end_index)) = self.pending_scroll_load {
                    let settled = self
                        .last_scroll_time
                        .is_some_and(|t| now.duration_since(t) >= Duration::from_millis(200));

                    let throttle_elapsed = self
                        .last_load_dispatch_time
                        .is_none_or(|t| now.duration_since(t) >= Duration::from_millis(150));

                    if settled || throttle_elapsed {
                        self.pending_scroll_load = None;
                        self.last_load_dispatch_time = Some(now);

                        let load_task = self.image_cache.update_visible_range(
                            self.config.sprites_location.clone(),
                            start_index,
                            end_index,
                            self.selected.selected_idx,
                        );
                        return Action::Run(load_task);
                    }
                }

                Action::None
            }
            Message::Scrolled(viewport) => {
                // Bottom scrollable's absolute position represents how far we've scrolled
                let bot_scroll_pos = viewport.absolute_offset().y;
                let rh = ROW_HEIGHT + 0.0;

                // Calculate the snapped position
                let snapped_bot_scroll_pos = (bot_scroll_pos / rh).round() * rh;

                // Check if we need to snap (only if off by more than threshold)
                let needs_snap = (bot_scroll_pos - snapped_bot_scroll_pos).abs() > 0.1;

                // Use snapped position for calculations
                let effective_scroll_pos = if needs_snap {
                    snapped_bot_scroll_pos
                } else {
                    bot_scroll_pos
                };

                // Top screen shows items that have scrolled past the top of bottom screen
                // When bot_scroll_pos = 0, top shows nothing (scroll position irrelevant)
                // When bot_scroll_pos = 450 (10 items), top should show items 0-9
                // The top scrollable should be at position 0 when it starts showing content
                let top_scroll_pos = (effective_scroll_pos - rh * TOP_SCREEN_ITEMS as f32).max(0.0);

                self.scroll_offset = effective_scroll_pos;
                let (first_visible, _) = self.visible_range(effective_scroll_pos);
                let target_index = (first_visible + self.selected.selected_slot)
                    .min(self.image_cache.pokemon_order.len().saturating_sub(1));

                if let Some(name) = self.image_cache.pokemon_order.get(target_index).cloned() {
                    if self.selected.selected_pokemon.as_ref() != Some(&name) {
                        self.selected.previously_selected = self.selected.selected_pokemon.clone();
                        self.selected.selected_pokemon = Some(name.clone());
                        self.selected_com_offset = self.image_cache.get_offset(&name);
                        if let Some(index) = self
                            .image_cache
                            .pokemon_order
                            .iter()
                            .position(|n| n == &name)
                        {
                            self.selected.selected_idx = Some(index);
                            self.selected.selected_slot =
                                target_index.saturating_sub(first_visible);
                        }
                    }
                }

                // Build commands: sync top scrollable + optionally snap bottom scrollable
                let sync_top = operation::scroll_to(
                    self.top_scroll_id.clone(),
                    scrollable::AbsoluteOffset {
                        x: 0.0,
                        y: top_scroll_pos,
                    },
                );

                let mut tasks = vec![sync_top];

                if needs_snap {
                    let snap_bottom = operation::scroll_to(
                        self.bot_scroll_id.clone(),
                        scrollable::AbsoluteOffset {
                            x: 0.0,
                            y: snapped_bot_scroll_pos,
                        },
                    );

                    tasks.push(snap_bottom);
                }

                let start_index = (effective_scroll_pos / rh).floor() as usize;
                let end_index = start_index + (self.items_per_page * 2);

                self.last_scroll_time = Some(Instant::now());
                self.pending_scroll_load =
                    Some((start_index.saturating_sub(TOP_SCREEN_ITEMS), end_index));

                Action::Run(iced::Task::batch(tasks))
            }
            Message::ImageLoaded(name, handle, com, generation) => {
                if !self.image_cache.is_generation_current(generation) {
                    return Action::None;
                }

                self.image_cache.insert(name.clone(), handle, Some(com));

                if self.selected.selected_pokemon.as_deref() == Some(name.as_str()) {
                    self.selected_com_offset = Some(com);
                }

                match self
                    .image_cache
                    .dispatch_next_load(self.config.as_ref().sprites_location.clone(), generation)
                {
                    Some(next) => Action::Run(next),
                    None => Action::None,
                }
            }
            Message::ImageLoadFailed(_name, generation) => {
                if !self.image_cache.is_generation_current(generation) {
                    return Action::None;
                }
                match self
                    .image_cache
                    .dispatch_next_load(self.config.as_ref().sprites_location.clone(), generation)
                {
                    Some(next) => Action::Run(next),
                    None => Action::None,
                }
            }
            Message::ImageCenterOfMass(name, offset, generation) => {
                if !self.image_cache.is_generation_current(generation) {
                    return Action::None;
                }
                self.image_cache.update_offset(&name, offset);
                if self.selected.selected_pokemon.as_deref() == Some(name.as_str()) {
                    self.selected_com_offset = Some(offset);
                }
                Action::None
            }
            Message::IOInput(action) => {
                let current_index = self.selected.selected_idx.unwrap_or(0);

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
                    let should_check_selected = !matches!(action, IOAction::Left | IOAction::Right);
                    let new_name = self.image_cache.pokemon_order[new_index].clone();
                    return Action::Run(Task::done(Message::SelectPokemon(
                        new_name,
                        should_check_selected,
                    )));
                }

                Action::None
            }
            Message::SelectPokemon(name, should_check_selected) => {
                self.selected.previously_selected = self.selected.selected_pokemon.clone();
                self.selected.selected_pokemon = Some(name.clone());
                self.selected_com_offset = self.image_cache.get_offset(&name); // None until ImageLoaded arrives

                if let Some(index) = self
                    .image_cache
                    .pokemon_order
                    .iter()
                    .position(|n| n == &name)
                {
                    self.selected.selected_idx = Some(index);
                    self.update_selection_for_index(index, !should_check_selected);

                    self.last_scroll_time = Some(Instant::now());
                    self.pending_scroll_load = Some(self.load_range_for_index(index));
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
            Message::SearchInteraction(i) => {
                if i == IconButtonInteraction::Released {
                    self.search_interaction = IconButtonInteraction::Hovered;
                    // your action logic here, or fire a Command
                    println!("Search clicked!");
                } else {
                    self.search_interaction = i;
                }
                Action::None
            }
            Message::FilterInteraction(i) => {
                if i == IconButtonInteraction::Released {
                    self.filter_interaction = IconButtonInteraction::Hovered;
                    // your action logic here, or fire a Command
                    println!("Filter clicked!");
                } else {
                    self.filter_interaction = i;
                }
                Action::None
            }
            Message::CloseInteraction(i) => {
                if i == IconButtonInteraction::Released {
                    self.close_interaction = IconButtonInteraction::Hovered;
                    return Action::GoHome;
                } else {
                    self.close_interaction = i;
                }
                Action::None
            }
        }
    }

    /// Computes a surrounding image-loading window for a given Pokémon index.
    ///
    /// # Arguments
    /// * `index` - The Pokémon index around which images should be loaded.
    ///
    /// # Returns
    /// A start/end range of indices to prefetch.
    fn load_range_for_index(&self, index: usize) -> (usize, usize) {
        let start = index.saturating_sub(TOP_SCREEN_ITEMS + self.items_per_page);
        let end = (index + self.items_per_page * 2).min(self.image_cache.pokemon_order.len());
        (start, end)
    }

    /// Starts an animated scroll to a specific vertical target.
    ///
    /// # Arguments
    /// * `target_y` - The target vertical offset in pixels.
    fn start_scroll_animation_to(&mut self, target_y: f32) {
        self.target_scroll_offset = target_y;
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

    pub fn subscription(&self) -> Subscription<Message> {
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
        let items: Vec<Element<Message>> = self
            .image_cache
            .pokemon_order
            .iter()
            .filter_map(|name| {
                let info = self.pokemon_data.get(name).unwrap();
                let is_owned = self.owned_pokemon.contains(name);
                let selected = self.selected.selected_pokemon.as_ref() == Some(name);
                let was_selected = self.selected.previously_selected.as_ref() == Some(name);

                Some(self.render_pokemon_item(
                    &name,
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

        let info_size = 256.0;
        let container_element = container(svg(self.info_svg.clone()))
            .width(info_size)
            .height(info_size)
            .padding(Padding {
                left: 10.0,
                top: 10.0,
                ..Default::default()
            });

        let image_element: Element<'_, Message> =
            if let Some(name) = self.selected.selected_pokemon.as_ref() {
                if let Some(handle) = self.image_cache.get(name) {
                    let offset = self.selected_com_offset.unwrap_or(0.5);
                    let x_offset = (0.5 - offset) * 256.0;

                    let image_padding = if x_offset > 0.0 {
                        Padding {
                            left: x_offset,
                            ..Default::default()
                        }
                    } else {
                        Padding {
                            right: -x_offset,
                            ..Default::default()
                        }
                    };

                    stack![
                        // invisible image. just used to allow resizing the actual image
                        // without resizing the container, since the first entry in the
                        // stack determines the bounds
                        image(handle.clone())
                            .width(Length::Fixed(info_size))
                            .height(Length::Fixed(info_size))
                            .opacity(0.0),
                        container(
                            image(handle.clone())
                                .content_fit(iced::ContentFit::Cover)
                                .width(Length::Fixed(220.0))
                                .height(Length::Fixed(220.0))
                        )
                        .padding(image_padding)
                        .width(Length::Fixed(640.0))
                        .height(Length::Fixed(480.0))
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center),
                        container_element
                    ]
                    .into()
                } else {
                    container_element.into()
                }
            } else {
                container_element.into()
            };

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
                row![image_element].width(Length::Fill).height(Length::Fill)
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
                    .height(Length::Fill)
                    .style(|_theme, status| {
                        let scroller_color = match status {
                            scrollable::Status::Dragged { .. } => {
                                Color::from_rgba8(24, 103, 184, 1.0)
                            }
                            scrollable::Status::Hovered { .. } => {
                                Color::from_rgba8(24, 103, 184, 0.9)
                            }
                            _ => Color::from_rgba8(24, 103, 184, 0.6),
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
            )
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(Color::TRANSPARENT)),
                ..Default::default()
            })
            .padding(iced::Padding {
                top: 10.0,
                bottom: 20.0,
                ..Default::default()
            })
        ]
        .into();
        elements.push(body);

        let screen = iced::widget::Stack::with_children(elements);

        let search = icon_button(
            self.search_icon.clone(),
            Some("Search"),
            &self.search_interaction,
            IconButtonColors::default(),
            Message::SearchInteraction,
        );

        let filter = icon_button(
            self.filter_icon.clone(),
            Some("Filter"),
            &self.filter_interaction,
            IconButtonColors::default(),
            Message::FilterInteraction,
        );

        let close = icon_button(
            self.close_icon.clone(),
            None,
            &self.close_interaction,
            IconButtonColors::default(),
            Message::CloseInteraction,
        );

        screen
            .push(
                container(column![
                    Space::new().height(Length::FillPortion(8)),
                    row![
                        // Search button
                        search,
                        filter,
                        close,
                        Space::new().width(Length::FillPortion(13))
                    ]
                    .align_y(Alignment::End)
                    .height(Length::FillPortion(2))
                ])
                .style(|_| container::Style {
                    background: None,
                    ..Default::default()
                }),
            )
            .into()
    }

    /// Renders a single Pokémon row for either the top or bottom list view.
    ///
    /// # Arguments
    /// * `name` - The Pokémon name to render.
    /// * `info` - The metadata used to display the Pokémon label.
    /// * `is_owned` - Whether the Pokémon is registered by the user.
    /// * `opacity` - The visual opacity for the row.
    /// * `selected` - Whether the row is currently selected.
    /// * `was_selected` - Whether the row was recently selected during the transition.
    /// * `is_top_screen` - Whether the row belongs to the top-screen list.
    ///
    /// # Returns
    /// A rendered UI element for the Pokémon row.
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

        let dname = info.display_name.clone().unwrap_or(name.to_string());
        // Pokemon info
        let info_column = column![
            text(format!(
                "{}\t{}",
                info.number,
                register::to_proper_case(&dname)
            ))
            .size(14)
            .color(Color::from_rgba(0.0, 0.0, 0.0, opacity)),
        ]
        .spacing(2);

        item_row = item_row.push(info_column);
        let size_now = self.size_animation.interpolate_with(|v| v, Instant::now());

        let selected_color = Color::from_rgba(0.2, 0.8, 0.2, 1.0);
        let default_color = Color::from_rgba(1.0, 1.0, 1.0, opacity);
        let mixed = lerp_color(selected_color, default_color, size_now);

        let color = if selected {
            selected_color
        } else if was_selected {
            mixed
        } else {
            default_color
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
                .width(Length::FillPortion(10))
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
            area.on_press(Message::SelectPokemon(name_, true))
        } else {
            area
        };

        row![
            Space::new().width(Length::FillPortion(8)),
            area,
            Space::new().width(Length::FillPortion(1))
        ]
        .into()
    }
}

/// Interpolates between two colors using a normalized value.
///
/// # Arguments
/// * `start` - The starting color.
/// * `end` - The ending color.
/// * `t` - The interpolation factor clamped to the range $[0, 1]$.
///
/// # Returns
/// The blended color.
pub fn lerp_color(start: Color, end: Color, t: f32) -> Color {
    // Clamp t to ensure it stays within the expected [0.0, 1.0] range
    let t = t.clamp(0.0, 1.0);

    Color {
        r: start.r + (end.r - start.r) * t,
        g: start.g + (end.g - start.g) * t,
        b: start.b + (end.b - start.b) * t,
        a: start.a + (end.a - start.a) * t,
    }
}
