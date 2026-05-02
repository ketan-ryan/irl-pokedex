use std::sync::{Arc, RwLock as StdRwLock};
use std::{collections::HashMap, time::Instant};

use iced::advanced::graphics::core::widget;
use iced::event::{self, Status};
use iced::keyboard::{Event::KeyPressed, Key, key::Named};
use iced::{
    Border, Color, Element, Event, Length, Subscription, Task, Theme,
    widget::{Space, canvas, column, container, image, image::Handle, row, scrollable, text},
    window,
};

use log::{debug, error, trace};

use crate::screen::register;
use crate::{
    elements::grid::Grid,
    io::{self, PokedexConfig, PokemonInfo},
};
// enum State {

// }

#[derive(Debug)]
pub struct PokedexBrowser {
    // state: State,
    config: Arc<PokedexConfig>,
    grid: Grid,
    last_tick: Instant,
    pokemon_data: HashMap<String, PokemonInfo>,
    owned_pokemon: std::collections::HashSet<String>,
    image_cache: ImageCache,
    scroll_offset: f32,
    items_per_page: usize,
    scroll_id: widget::Id,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick(std::time::Instant),
    Scrolled(scrollable::Viewport),
    ImageLoaded(String, Handle),
    ImageLoadFailed(String),
    IOInput(IOAction),
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

        let mut image_cache = ImageCache::new(pokemon_names, 15);

        // Initial load command
        let load_task =
            image_cache.update_visible_range(config.as_ref().sprites_location.clone(), 0, 10);

        let state = Self {
            config,
            grid: Grid::new(),
            last_tick: Instant::now(),
            pokemon_data,
            owned_pokemon,
            image_cache,
            scroll_offset: 0.0,
            items_per_page: 10,
            scroll_id: "scrollable".into(),
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
                const ROW_HEIGHT: f32 = 45.0;

                // Get the actual scroll offset (viewport gives you the absolute position)
                let scroll_offset = viewport.absolute_offset().y;

                // Calculate which row is at the top of the visible area
                let start_index = (scroll_offset / ROW_HEIGHT).round() as usize;

                let visible_rows = 10;
                let end_index = start_index + visible_rows;

                self.scroll_offset = scroll_offset;
                Action::Run(self.image_cache.update_visible_range(
                    self.config.sprites_location.clone(),
                    start_index,
                    end_index,
                ))
            }
            Message::ImageLoaded(name, handle) => {
                debug!("Loaded {}", name);
                self.image_cache.insert(name, handle);
                Action::None
            }
            Message::ImageLoadFailed(name) => {
                error!("Failed to load image for: {}", name);
                Action::None
            }
            Message::IOInput(action) => {
                const SCROLL_AMOUNT: f32 = 45.0;
                match action {
                    IOAction::ScrollUp => {
                        debug!("Got scroll up event");
                        return Action::Run(iced::widget::operation::scroll_by(
                            self.scroll_id.clone(),
                            scrollable::AbsoluteOffset {
                                x: 0.0,
                                y: -SCROLL_AMOUNT,
                            },
                        ));
                    }
                    IOAction::ScrollDown => {
                        debug!("Got scroll down event");
                        return Action::Run(iced::widget::operation::scroll_by(
                            self.scroll_id.clone(),
                            scrollable::AbsoluteOffset {
                                x: 0.0,
                                y: SCROLL_AMOUNT,
                            },
                        ));
                    }
                };
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // tick screen for updates ~60fps
        // time::every(Duration::from_millis(16)).map(Message::Tick)

        Subscription::batch([
            window::frames().map(Message::Tick),
            // TODO: Will need custom subscription / event to handle rpi IO
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
                _ => None,
            }),
        ])
    }

    pub fn top_view(&self) -> Element<'_, Message> {
        const ROW_HEIGHT: f32 = 45.0;

        // Calculate visible range
        let start_index = (self.scroll_offset / ROW_HEIGHT).round() as usize;
        let end_index = start_index + self.items_per_page;

        // Apply buffer to rendering range (match the buffer_size from ImageCache)
        let buffer_size = self.image_cache.buffer_size;
        let render_start = start_index.saturating_sub(buffer_size);
        let render_end = (end_index + buffer_size).min(self.image_cache.pokemon_order.len());

        // Add spacer for items before visible range
        let top_spacer_height = render_start as f32 * ROW_HEIGHT;

        // Render only the visible + buffer items
        let visible_items: Vec<Element<Message>> = self
            .image_cache
            .pokemon_order
            .iter()
            .skip(render_start)
            .take(render_end - render_start)
            .map(|name| {
                let info = self.pokemon_data.get(name).unwrap();
                let is_owned = self.owned_pokemon.contains(name);

                self.render_pokemon_item(name, info, is_owned)
            })
            .collect();

        // Calculate bottom spacer for items after visible range
        let bottom_spacer_height =
            (self.image_cache.pokemon_order.len() - render_end) as f32 * ROW_HEIGHT;

        // Build the column with spacers
        let mut content = column!().spacing(5);

        // Top spacer to maintain scroll position
        if top_spacer_height > 0.0 {
            content = content.push(Space::new().width(Length::Fill).height(top_spacer_height));
        }

        // Add visible items
        for item in visible_items {
            content = content.push(item);
        }

        // Bottom spacer to maintain total height
        if bottom_spacer_height > 0.0 {
            content = content.push(
                Space::new()
                    .width(Length::Fill)
                    .height(bottom_spacer_height),
            );
        }

        container(
            scrollable(content.spacing(5))
                .id(self.scroll_id.clone())
                .on_scroll(Message::Scrolled)
                .height(Length::Fill),
        )
        .padding(iced::Padding {
            top: 20.0,
            bottom: 20.0,
            ..Default::default()
        })
        .into()
    }

    fn render_pokemon_item(
        &'_ self,
        name: &str,
        info: &PokemonInfo,
        is_owned: bool,
    ) -> Element<'_, Message> {
        const IMG_SIZE: f32 = 20.0;
        let mut item_row = row!().spacing(3).align_y(iced::Alignment::Center);

        debug!("Rendering {} {}", info.number, name);

        // Add image or placeholder
        if let Some(handle) = self.image_cache.get(name) {
            item_row = item_row.push(image(handle).width(IMG_SIZE).height(IMG_SIZE));
        } else if self.image_cache.is_loading(name) {
            // Show loading indicator
            item_row = item_row.push(
                container(text("Loading..."))
                    .width(IMG_SIZE)
                    .height(IMG_SIZE)
                    .center(Length::Fill),
            );
        } else {
            // Show placeholder
            item_row = item_row.push(Space::new().width(IMG_SIZE).height(IMG_SIZE));
        }

        // Pokemon info
        let info_column = column![
            text(format!(
                "{}\t{}",
                info.number,
                register::to_proper_case(name)
            ))
            .size(14),
        ]
        .spacing(2);

        item_row = item_row.push(info_column);

        // Owned indicator
        if is_owned {
            item_row = item_row.push(
                container(text("✓ Owned").size(16))
                    .padding(5)
                    .style(Self::selected_style),
            );
        }

        row![
            Space::new().width(Length::FillPortion(2)),
            container(item_row)
                .padding(10)
                .width(Length::FillPortion(1))
                .style(if is_owned {
                    Self::owned_style
                } else {
                    Self::normal_style
                })
        ]
        .into()
    }

    pub fn is_owned(&self, pokemon_name: &str) -> bool {
        self.owned_pokemon.contains(pokemon_name)
    }

    pub fn bottom_view(&self) -> Element<'_, Message> {
        container(
            canvas::Canvas::new(&self.grid)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .style(|_| iced::widget::container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(
                1.0,
                162.0 / 255.0,
                0.0,
            ))),
            text_color: Some(Color::BLACK),
            border: Default::default(),
            shadow: Default::default(),
            snap: Default::default(),
        })
        .into()
    }

    pub fn normal_style(theme: &Theme) -> container::Style {
        container::Style {
            background: Some(Color::from_rgb(0.2, 0.3, 0.5).into()), // DS-style Blue
            border: Border {
                radius: 5.0.into(),
                width: 2.0,
                color: Color::from_rgb(0.1, 0.3, 0.7),
            },
            text_color: Some(Color::WHITE),
            ..Default::default()
        }
    }

    pub fn selected_style(theme: &Theme) -> container::Style {
        container::Style {
            background: Some(Color::from_rgb(0.2, 0.5, 0.9).into()), // DS-style Blue
            border: Border {
                radius: 5.0.into(),
                width: 2.0,
                color: Color::from_rgb(0.1, 0.3, 0.7),
            },
            text_color: Some(Color::WHITE),
            ..Default::default()
        }
    }

    pub fn owned_style(theme: &Theme) -> container::Style {
        container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.9, 1.0, 0.9,
            ))),
            border: iced::Border {
                color: iced::Color::from_rgb(0.2, 0.8, 0.2),
                width: 2.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        }
    }
}
