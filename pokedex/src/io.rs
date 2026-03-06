use anyhow::Result;
use config::Config;
use serde::Deserialize;

use std::{collections::HashMap};
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{elements::gstreamer_stream::{VideoFrame}};
use crate::PokedexError;


#[derive(Deserialize, Debug)]
pub struct PokemonInfo {
    pub number: String,
    pub r#type: String,
    pub height: String,
    pub weight: String,
    pub abilities: Vec<String>,
    pub dex_entries: HashMap<String, String>
}

pub fn load_settings() -> Result<HashMap<String, String>, PokedexError> {
    Config::builder()
        .add_source(config::File::with_name("pokedex_settings"))
        .build()
        .map_err(|e| match e {
            config::ConfigError::NotFound(_) => PokedexError::ConfigNotFound,
            e => PokedexError::MalformedConfig(e.to_string())
        })?
        .try_deserialize::<HashMap<String, String>>()
        .map_err(|e| PokedexError::MalformedConfig(e.to_string()))
}

pub fn load_dex_entries(filename: &str) -> Result<HashMap<String, PokemonInfo>, PokedexError> {
    let dex =  std::fs::read_to_string(filename).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => PokedexError::PokedexNotFound(filename.to_string()),
        _ => PokedexError::MalformedPokedex(e.to_string())
    })?;

    serde_json::from_str(&dex)
        .map_err(|e| PokedexError::MalformedPokedex(e.to_string()))
}

pub fn get_local_path() -> io::Result<PathBuf> {
    // Get the path to the current executable
    let exe_path: PathBuf = env::current_exe()?;
    // Get the directory containing the executable
    let exe_dir: &std::path::Path = exe_path.parent().expect("Executable must be in a directory");
    println!("Executable path: {}", exe_path.display());

    Ok(exe_dir.into())
}

pub fn save_frame(frame: &VideoFrame) -> Result<(), image::ImageError> {
    // Save image to a temporary staging area while classification runs
    let path = get_local_path()?; 
    let staging_area = path.join("staging");

    // remove dir could error if dir isn't present - ignore
    let _ = fs::remove_dir_all(&staging_area);
    fs::create_dir_all(&staging_area)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time should go forward")
        .as_millis();

    let name = now.to_string() + ".png";

    let out_path = &staging_area.join(name);
    println!("{:?}", out_path);
    image::save_buffer_with_format(
        out_path, 
        &frame.data, 
        frame.width, 
        frame.height, 
        image::ColorType::Rgba8,
        image::ImageFormat::Png
    )
}