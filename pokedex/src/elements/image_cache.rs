use std::collections::HashMap;
use std::sync::{Arc, RwLock as StdRwLock};

use crate::io;
use crate::screen::browse_pokedex::Message;
use anyhow::anyhow;
use iced::widget::image::Handle;
use log::trace;

#[derive(Debug, Clone)]
pub struct ImageCacheEntry {
    pub handle: Handle,
    pub center_of_mass: Option<f32>,
}

#[derive(Debug)]
pub struct ImageCache {
    // Sync cache for rendering (can be accessed without await)
    pub sync_cache: Arc<StdRwLock<HashMap<String, ImageCacheEntry>>>,
    // Ordered list of all pokemon names
    pub pokemon_order: Vec<String>,
    // Current visible range
    pub visible_start: usize,
    pub visible_end: usize,
    // Buffer size
    pub buffer_size: usize,
    // Track which images are currently being loaded
    pub loading: Arc<StdRwLock<std::collections::HashSet<String>>>,
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
        selected_option: Option<usize>,
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

        // start at the currently selected image and load images spiraling outward
        let selected = selected_option.unwrap_or(load_start + load_end / 2);
        let indices = (0..(load_end - load_start))
            .flat_map(|offset| {
                let above = selected + offset;
                let below = selected.checked_sub(offset);
                if offset == 0 {
                    [Some(above), None]
                } else {
                    [
                        Some(above).filter(|&i| i < load_end),
                        below.filter(|&i| i >= load_start),
                    ]
                }
            })
            .flatten()
            .collect::<Vec<_>>();

        for i in indices {
            let name = self.pokemon_order[i].clone();

            // Skip if already loaded or loading
            let should_load = {
                let cache = self.sync_cache.read().unwrap();
                let loading = self.loading.read().unwrap();
                !cache.contains_key(&name) && !loading.contains(&name)
            };

            if should_load {
                let load_task = self.load_image_async(sprite_folder.clone(), name.clone());

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
                match result {
                    Ok(bytes) => {
                        let handle = Handle::from_bytes(bytes.clone());
                        let offset = Some(crate::screen::register::find_image_com(&bytes));

                        Ok((pokemon_name, handle, offset))
                    }
                    Err(err) => Err((pokemon_name, err)),
                }
            },
            move |result| match result {
                Ok((name, handle, offset)) => {
                    let mut loading_set = loading.write().unwrap();
                    loading_set.remove(&name);
                    Message::ImageLoaded(name, handle, offset)
                }
                Err((name, _)) => {
                    let mut loading_set = loading.write().unwrap();
                    loading_set.remove(&name);
                    Message::ImageLoadFailed(name)
                }
            },
        )
    }

    pub fn compute_center_of_mass_async(
        &self,
        pokemon_name: String,
        handle: Handle,
    ) -> iced::Task<Message> {
        let loading = self.loading.clone();

        // Mark as loading for COM computation if not already
        {
            let mut loading_set = loading.write().unwrap();
            loading_set.insert(pokemon_name.clone());
        }

        iced::Task::perform(
            async move {
                let result = match handle {
                    Handle::Bytes(_, bytes) => {
                        let offset = crate::screen::register::find_image_com(bytes.as_ref());
                        Ok((pokemon_name, offset))
                    }
                    _ => Err((
                        pokemon_name,
                        anyhow!("unsupported image handle variant for COM"),
                    )),
                };
                result
            },
            move |result| {
                let mut loading_set = loading.write().unwrap();
                match result {
                    Ok((name, offset)) => {
                        loading_set.remove(&name);
                        Message::ImageCenterOfMass(name, offset)
                    }
                    Err((name, _)) => {
                        loading_set.remove(&name);
                        Message::ImageCenterOfMass(name, 0.5)
                    }
                }
            },
        )
    }

    /// Get handle for a specific pokemon (synchronous for rendering)
    pub fn get(&self, pokemon_name: &str) -> Option<Handle> {
        let cache = self.sync_cache.read().unwrap();
        cache.get(pokemon_name).map(|entry| entry.handle.clone())
    }

    /// Store a loaded image
    pub fn insert(&self, name: String, handle: Handle, center_of_mass: Option<f32>) {
        let mut cache = self.sync_cache.write().unwrap();
        cache.insert(
            name,
            ImageCacheEntry {
                handle,
                center_of_mass,
            },
        );
    }

    pub fn update_offset(&self, pokemon_name: &str, center_of_mass: f32) {
        let mut cache = self.sync_cache.write().unwrap();
        if let Some(entry) = cache.get_mut(pokemon_name) {
            entry.center_of_mass = Some(center_of_mass);
        }
    }

    /// Get the center-of-mass offset for a loaded image
    pub fn get_offset(&self, pokemon_name: &str) -> Option<f32> {
        let cache = self.sync_cache.read().unwrap();
        cache
            .get(pokemon_name)
            .and_then(|entry| entry.center_of_mass)
    }
}
