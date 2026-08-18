use std::collections::HashMap;
use std::sync::{Arc, RwLock as StdRwLock};

use crate::browse_pokedex::Message;
use crate::io;
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
    /// Create a new image cache with the provided Pokémon ordering and prefetch buffer size.
    ///
    /// Args:
    /// - pokemon_names: The ordered list of Pokémon names to track.
    /// - buffer_size: The number of surrounding entries to preload around the visible range.
    ///
    /// Returns: A new image cache instance.
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

    /// Return the current load-generation counter for the cache.
    ///
    /// Returns: The active generation identifier.
    pub(crate) fn current_generation(&self) -> u64 {
        *self.load_generation.read().unwrap()
    }

    /// Start a new load cycle, clearing pending work and incrementing the generation counter.
    ///
    /// Returns: The new generation identifier for the cycle.
    fn begin_load_cycle(&self) -> u64 {
        let mut generation = self.load_generation.write().unwrap();
        *generation += 1;
        let next_generation = *generation;

        self.loading.write().unwrap().clear();
        self.pending_queue.write().unwrap().clear();

        next_generation
    }

    /// Mark a Pokémon image as currently being loaded for the given generation.
    ///
    /// Args:
    /// - pokemon_name: The name of the Pokémon being loaded.
    /// - generation: The generation number that owns the load.
    ///
    fn mark_loading(&self, pokemon_name: String, generation: u64) {
        let mut loading = self.loading.write().unwrap();
        loading.insert(pokemon_name, generation);
    }

    /// Check whether the supplied generation is still the active load cycle.
    ///
    /// Args:
    /// - generation: The generation number to validate.
    ///
    /// Returns: True when the generation is still current.
    pub(crate) fn is_generation_current(&self, generation: u64) -> bool {
        self.current_generation() == generation
    }

    /// Update the visible range and queue image loads for the current viewport and buffer.
    ///
    /// Args:
    /// - sprite_folder: The folder containing the sprite images.
    /// - start: The first visible index.
    /// - end: The first index after the visible range.
    /// - selected_option: The currently selected Pokémon index, if any.
    ///
    /// Returns: A batch of loading tasks for the queued image loads.
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

    /// Pop the next queued Pokémon name for the active generation and start loading it.
    ///
    /// Args:
    /// - sprite_folder: The folder containing the sprite images.
    /// - generation: The generation number for the current load cycle.
    ///
    /// Returns: An optional loading task to execute for the next queued image.
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

    /// Load an image asynchronously from disk and emit a message when the task completes.
    ///
    /// Args:
    /// - sprite_folder: The folder containing the sprite images.
    /// - pokemon_name: The Pokémon whose image should be loaded.
    /// - generation: The generation number for the current load cycle.
    ///
    /// Returns: A task that resolves to an image-loading message.
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

    /// Return the cached image handle for a Pokémon, if it has already been loaded.
    ///
    /// Args:
    /// - pokemon_name: The Pokémon whose cached handle should be returned.
    ///
    /// Returns: The cached image handle, if present.
    pub fn get(&self, pokemon_name: &str) -> Option<Handle> {
        let cache = self.sync_cache.read().unwrap();
        cache.get(pokemon_name).map(|entry| entry.handle.clone())
    }

    /// Store a loaded image and its optional center-of-mass offset in the cache.
    ///
    /// Args:
    /// - name: The Pokémon name used as the cache key.
    /// - handle: The image handle to store.
    /// - center_of_mass: The optional center-of-mass value for the image.
    ///
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

    /// Return the cached center-of-mass offset for a loaded image, if available.
    ///
    /// Args:
    /// - pokemon_name: The Pokémon whose cached offset should be returned.
    ///
    /// Returns: The cached center-of-mass offset, if present.
    pub fn get_offset(&self, pokemon_name: &str) -> Option<f32> {
        let cache = self.sync_cache.read().unwrap();
        cache
            .get(pokemon_name)
            .and_then(|entry| entry.center_of_mass)
    }
}
/// Compute the horizontal center-of-mass of an RGBA image, normalized to the image width.
///
/// Args:
/// - img: The RGBA image to analyze.
///
/// Returns: A normalized horizontal center-of-mass value between 0 and 1.
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
