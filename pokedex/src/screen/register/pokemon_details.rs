use image::imageops::FilterType;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::time::Instant;

use noise::{self, Exponent, NoiseFn, Perlin};
use rand::prelude::*;

use crate::io::PokemonInfo;

// Animation speed for updating noise
const SPEED: f64 = 0.01;

#[derive(Debug, Clone)]
pub struct PokemonDetailsState {
    time: f64,
    generating: bool,
    pub noise_image: Option<iced::widget::image::Handle>,
    last_completed: Instant,
    pub palette: Vec<Rgb>,
    random: u32,
    current_pokemon: Option<PokemonInfo>,
    current_dex: Option<String>,
}

impl PokemonDetailsState {
    pub fn new() -> Self {
        let mut rng = rand::rng();
        let random = rng.random::<u32>();

        Self {
            time: 0.0,
            generating: false,
            noise_image: None,
            last_completed: Instant::now(),
            palette: Vec::new(),
            random,
            current_pokemon: None,
            current_dex: None,
        }
    }

    pub fn tick(&mut self) -> Option<iced::widget::image::Handle> {
        if self.generating {
            return None;
        }

        // might take more than one frame to generate the noise
        let now = Instant::now();
        let dt = now.duration_since(self.last_completed).as_secs_f64();
        self.time += dt * SPEED;

        self.generating = true;
        let t = self.time;
        let (w, h) = (640 / 4, 480 / 4);
        Some(build_noise_image(w, h, t, &self.palette, self.random))
    }

    pub fn update_noise_handle(&mut self, handle: Option<iced::widget::image::Handle>) {
        self.noise_image = handle;
    }

    pub fn set_palette(&mut self, palette: Vec<Rgb>) {
        self.palette = palette;
    }

    pub fn quantize(bytes: &[u8]) -> Vec<Rgb> {
        let loaded = load_img(bytes);
        return median_cut(loaded, MAX_ITERATIONS);
    }

    pub fn set_current_pokemon(&mut self, pokemon: Option<PokemonInfo>) -> Option<String> {
        self.current_pokemon = pokemon;
        let random_index = rand::random_range(
            0..self
                .current_pokemon
                .as_ref()
                .map_or(1, |p| p.dex_entries.len()),
        );
        self.current_dex = self.current_pokemon.as_ref().and_then(|p| {
            p.dex_entries
                .iter()
                .nth(random_index)
                .map(|(_, v)| v.clone())
        });
        self.current_dex.clone()
    }

    pub fn current_pokedex(&self) -> Option<&String> {
        self.current_dex.as_ref()
    }

    pub fn current_pokemon(&self) -> Option<&PokemonInfo> {
        self.current_pokemon.as_ref()
    }
}

/// Generates a noise texture and colors it using a color palette
/// # Arguments
/// * `width` the width of the noise image to generate
/// * `height` the height of the noise image to generate
/// * `time` time scaled by speed, used to scroll the noise texture
/// * `seed` a randomly generated seed for the perlin noise
///
/// # Returns
/// An [`iced::widget::image::Handle`] containing the noise texture at the current time
fn build_noise_image(
    width: u32,
    height: u32,
    time: f64,
    palette: &[Rgb],
    seed: u32,
) -> iced::widget::image::Handle {
    let perlin = Perlin::new(seed);
    let exponent = Exponent::new(perlin).set_exponent(3.0);

    // how zoomed in it is
    let scale = 0.009_f64;
    let sorted_palette = sort_palette_by_hue(palette);

    // Sampling resolution — lower = faster but blockier
    let step: u32 = 1;

    let rows: Vec<u32> = (0..height).step_by(step as usize).collect();

    let pixels: Vec<u8> = rows
        .into_par_iter()
        .flat_map(|py| {
            let mut row = Vec::with_capacity((width * 4) as usize);
            for px in (0..width).step_by(step as usize) {
                let v = exponent.get([px as f64 * scale, py as f64 * scale, time]);
                let v = (v + 1.0) / 2.0; // remap to 0..1
                let color = clamp_brightness(sample_palette(v, &sorted_palette), 200.0);
                // repeat the pixel across the step block
                for _ in 0..step {
                    row.extend_from_slice(&[color[0] as u8, color[1] as u8, color[2] as u8, 255]);
                }
            }
            // repeat the row downward to fill the step block height
            row.repeat(step as usize)
        })
        .collect();

    iced::widget::image::Handle::from_rgba(width, height, pixels)
}

// Results in 2^MAX_ITERATIONS buckets
const MAX_ITERATIONS: u32 = 4;
// Width to scale image to
const MAX_WIDTH: u32 = 64;
pub type Rgb = [f64; 3];

/// Clamps an RGB color to a maximum brightness
/// # Arguments
/// * `color` the 3-channel color to clamp
/// * `max_brightness` the maximum allowed brightness
///
/// # Returns
/// The color if its brightness < the threshold, or the scaled brightness
fn clamp_brightness(color: Rgb, max_brightness: f64) -> Rgb {
    let brightness = (color[0] + color[1] + color[2]) / 3.0;
    if brightness > max_brightness {
        let scale = max_brightness / brightness;
        [color[0] * scale, color[1] * scale, color[2] * scale]
    } else {
        color
    }
}

/// Given a palette of RGB colors, returns a palette sorted by hue
/// # Arguments
/// `palette` The list of RGB colors
///
/// # Returns
/// The original colors sorted by their hues
fn sort_palette_by_hue(palette: &[Rgb]) -> Vec<Rgb> {
    let mut sorted = palette.to_vec();
    sorted.sort_by(|a, b| hue_of(a).partial_cmp(&hue_of(b)).unwrap());
    sorted
}

/// Linearly interpolates between 2 RGB colors
/// # Arguments
/// * `a` The start color
/// * `b` The end color
/// * `t` The amount to interpolate by
///
/// # Returns
/// A color between the original colors at the specified point
fn lerp_color(a: &Rgb, b: &Rgb, t: f64) -> Rgb {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Given a value between 0 and 1, finds the closest color in the palette
/// # Arguments
/// * `value` A value between [0..1]
/// * `palette` The color palette to use to find the output color
///
/// # Returns
/// An output color at the appropriate spot in the palette
fn sample_palette(value: f64, palette: &[Rgb]) -> Rgb {
    let n = palette.len();
    if n == 0 {
        return [0.0, 0.0, 0.0];
    }
    if n == 1 {
        return palette[0];
    }

    // Scale value into palette index space
    let scaled = (value as f32) * (n - 1) as f32;
    let lo = (scaled.floor() as usize).min(n - 2);
    let hi = lo + 1;
    let t = scaled - lo as f32; // lerp factor

    lerp_color(&palette[lo], &palette[hi], t.into())
}

/// Calculates the hue of a color from its RGB values
/// # Arguments
/// `c` The input RGB color
///
/// # Returns
/// The color's hue as a float between [0..1]
fn hue_of(c: &Rgb) -> f64 {
    let r = c[0] / 255.0;
    let g = c[1] / 255.0;
    let b = c[2] / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    if delta == 0.0 {
        return delta;
    }

    let hue = if max == r {
        ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };

    hue / 6.0 // normalise to 0..1
}

/// Converts an image from a list of bytes to a list of RGB colors
///
/// Resizes the image to a max size of 64x64, as more detail is unecessary for our use
/// # Arguments
/// `bytes` The image as a bytearray
///
/// # Returns
/// The image as a list of RGB colors
fn load_img(bytes: &[u8]) -> Vec<Rgb> {
    let img = image::load_from_memory(bytes).expect("Failed to decode image");

    let img = if img.width() > MAX_WIDTH {
        let height_percent = MAX_WIDTH as f64 / img.width() as f64;
        let new_height = (img.height() as f64 * height_percent) as u32;
        img.resize(MAX_WIDTH, new_height, FilterType::Nearest)
    } else {
        img
    };

    // Convert to RGBA so the channel layout is always consistent
    let img = img.to_rgba8();

    let pixels = img
        .pixels()
        .filter(|p| p.0[3] > 0) // skip fully transparent pixels
        .map(|p| [p.0[0] as f64, p.0[1] as f64, p.0[2] as f64])
        .collect();

    pixels
}

/// Finds the color channel with the largest range in the image, used for splitting the image in the median cut algorithm
/// # Arguments
/// `arr` The image as a list of RGB colors
///
/// # Returns
/// The index of the channel with the largest range (0 for red, 1 for green, 2 for blue)
fn find_max_range(arr: &[Rgb]) -> usize {
    let mut max = [f64::NEG_INFINITY; 3];
    let mut min = [f64::INFINITY; 3];

    for pixel in arr {
        for c in 0..3 {
            max[c] = max[c].max(pixel[c]);
            min[c] = min[c].min(pixel[c]);
        }
    }

    let ranges = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];

    ranges
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Performs the median cut algorithm to quantize the image to a smaller palette
///
/// Implementation based on this article https://en.wikipedia.org/wiki/Median_cut
/// # Arguments
/// `arr` The image as a list of RGB colors
/// `depth` The number of times to split the image, resulting in 2^depth colors in the output palette
///
/// # Returns
/// A color palette generated from the input image
fn median_cut(mut arr: Vec<Rgb>, depth: u32) -> Vec<Rgb> {
    if depth == 0 {
        let len = arr.len() as f64;
        let sum = arr.iter().fold([0.0f64; 3], |mut acc, p| {
            acc[0] += p[0];
            acc[1] += p[1];
            acc[2] += p[2];
            acc
        });
        return vec![[sum[0] / len, sum[1] / len, sum[2] / len]];
    }

    let channel = find_max_range(&arr);
    arr.sort_by(|a, b| a[channel].partial_cmp(&b[channel]).unwrap());

    let upper = arr.split_off(arr.len() / 2);

    let mut palette = median_cut(arr, depth - 1);
    palette.extend(median_cut(upper, depth - 1));
    palette
}
