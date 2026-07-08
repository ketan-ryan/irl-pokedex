use std::collections::HashMap;
use std::sync::{Arc, RwLock as StdRwLock};

use crate::io;
use crate::screen::browse_pokedex::Message;
use iced::widget::image::Handle;
use log::trace;
use std::collections::VecDeque;

const MAX_CONCURRENT_LOADS: usize = 2;

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
    // Track which images are currently being loaded and which load cycle owns them
    pub loading: Arc<StdRwLock<HashMap<String, u64>>>,
    load_generation: Arc<StdRwLock<u64>>,
    pending_queue: Arc<StdRwLock<VecDeque<String>>>,
}

impl ImageCache {
    pub fn new(pokemon_names: Vec<String>, buffer_size: usize) -> Self {
        Self {
            sync_cache: Arc::new(StdRwLock::new(HashMap::new())),
            pokemon_order: pokemon_names,
            visible_start: 0,
            visible_end: 0,
            buffer_size,
            loading: Arc::new(StdRwLock::new(HashMap::new())),
            load_generation: Arc::new(StdRwLock::new(0)),
            pending_queue: Arc::new(StdRwLock::new(VecDeque::new())),
        }
    }

    pub(crate) fn current_generation(&self) -> u64 {
        *self.load_generation.read().unwrap()
    }

    fn begin_load_cycle(&self) -> u64 {
        let mut generation = self.load_generation.write().unwrap();
        *generation += 1;
        let next_generation = *generation;

        self.loading.write().unwrap().clear();
        self.pending_queue.write().unwrap().clear();

        next_generation
    }

    fn mark_loading(&self, pokemon_name: String, generation: u64) {
        let mut loading = self.loading.write().unwrap();
        loading.insert(pokemon_name, generation);
    }

    pub(crate) fn is_generation_current(&self, generation: u64) -> bool {
        self.current_generation() == generation
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

        let generation = self.begin_load_cycle();

        trace!("Starting at: {}, ending at: {}", start, end);

        let load_start = start.saturating_sub(self.buffer_size);
        let load_end = (end + self.buffer_size).min(self.pokemon_order.len());

        trace!("Load start: {}, load end: {}", load_start, load_end);

        // Cleanup old images
        {
            let mut cache = self.sync_cache.write().unwrap();
            let pokemon_order = &self.pokemon_order;
            cache.retain(|name, _| {
                pokemon_order
                    .iter()
                    .position(|n| n == name)
                    .is_some_and(|index| index >= load_start && index < load_end)
            });
        }

        trace!("Cleaned up old images");

        // start at the currently selected image and load images spiraling outward
        let selected = selected_option.unwrap_or(load_start + (load_end - load_start) / 2);
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
            .flatten();

        // Preserve spiral order, but keep visible-range names ahead of buffer-only names.
        let mut visible_names = Vec::new();
        let mut buffer_names = Vec::new();

        for i in indices {
            let name = self.pokemon_order[i].clone();
            let already_cached = self.sync_cache.read().unwrap().contains_key(&name);
            if already_cached {
                continue;
            }
            if i >= start && i < end {
                visible_names.push(name);
            } else {
                buffer_names.push(name);
            }
        }

        let queue: VecDeque<String> = visible_names.into_iter().chain(buffer_names).collect();
        *self.pending_queue.write().unwrap() = queue;

        let tasks: Vec<_> = (0..MAX_CONCURRENT_LOADS)
            .map_while(|_| self.dispatch_next_load(sprite_folder.clone(), generation))
            .collect();

        iced::Task::batch(tasks)
    }

    /// Pop the next name off the pending queue (if any) for the given generation
    /// and start loading it. Returns None if the queue is empty or stale.
    pub fn dispatch_next_load(
        &self,
        sprite_folder: String,
        generation: u64,
    ) -> Option<iced::Task<Message>> {
        if !self.is_generation_current(generation) {
            return None;
        }

        loop {
            let name = self.pending_queue.write().unwrap().pop_front()?;

            let already_have = {
                let cache = self.sync_cache.read().unwrap();
                let loading = self.loading.read().unwrap();
                cache.contains_key(&name) || loading.contains_key(&name)
            };

            if !already_have {
                return Some(self.load_image_async(sprite_folder, name, generation));
            }
            // else loop and try the next queued name
        }
    }

    fn load_image_async(
        &self,
        sprite_folder: String,
        pokemon_name: String,
        generation: u64,
    ) -> iced::Task<Message> {
        let loading = self.loading.clone();

        self.mark_loading(pokemon_name.clone(), generation);

        iced::Task::perform(
            async move {
                let name = pokemon_name.clone();
                let decoded = tokio::task::spawn_blocking(move || {
                    let bytes = io::load_png(sprite_folder, &name.to_lowercase())?;
                    let img = image::load_from_memory(&bytes)?.into_rgba8();
                    let (width, height) = img.dimensions();
                    let com = find_image_com_rgba(&img);
                    Ok::<_, anyhow::Error>((width, height, img.into_raw(), com))
                })
                .await;

                match decoded {
                    Ok(Ok((width, height, pixels, com))) => {
                        let handle = Handle::from_rgba(width, height, pixels);
                        Ok((pokemon_name, handle, com, generation))
                    }
                    _ => Err((pokemon_name, generation)),
                }
            },
            move |result| match result {
                Ok((name, handle, com, generation)) => {
                    loading.write().unwrap().remove(&name);
                    Message::ImageLoaded(name, handle, com, generation)
                }
                Err((name, generation)) => {
                    loading.write().unwrap().remove(&name);
                    Message::ImageLoadFailed(name, generation)
                }
            },
        )
    }

    pub fn compute_center_of_mass_async(
        &self,
        pokemon_name: String,
        handle: Handle,
        generation: u64,
    ) -> iced::Task<Message> {
        let loading = self.loading.clone();

        self.mark_loading(pokemon_name.clone(), generation);

        iced::Task::perform(
            async move {
                let bytes = match handle {
                    Handle::Bytes(_, bytes) => bytes,
                    _ => return Err((pokemon_name, generation)),
                };
                let name = pokemon_name.clone();
                match tokio::task::spawn_blocking(move || {
                    crate::screen::register::find_image_com(bytes.as_ref())
                })
                .await
                {
                    Ok(offset) => Ok((name, offset, generation)),
                    Err(_) => Err((pokemon_name, generation)),
                }
            },
            move |result| match result {
                Ok((name, offset, generation)) => {
                    loading.write().unwrap().remove(&name);
                    Message::ImageCenterOfMass(name, offset, generation)
                }
                Err((name, generation)) => {
                    loading.write().unwrap().remove(&name);
                    Message::ImageCenterOfMass(name, 0.5, generation)
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
pub fn find_image_com_rgba(img: &image::RgbaImage) -> f32 {
    let (width, height) = img.dimensions();
    if width == 0 || height == 0 {
        return 0.5;
    }
    let mut sum_x: u64 = 0;
    let mut count: u64 = 0;
    for (i, pixel) in img.as_raw().chunks_exact(4).enumerate() {
        if pixel[3] > 0 {
            sum_x += (i % width as usize) as u64;
            count += 1;
        }
    }
    if count == 0 {
        0.5
    } else {
        (sum_x as f32 / count as f32) / width as f32
    }
}
