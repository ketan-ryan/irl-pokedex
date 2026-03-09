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
    let cfg_path = get_local_path()?.join("pokedex_settings.yaml");
    Config::builder()
        .add_source(config::File::from(cfg_path))
        .build()
        .map_err(|e| match e {
            config::ConfigError::NotFound(_) => PokedexError::ConfigNotFound,
            e => PokedexError::MalformedConfig(e.to_string())
        })?
        .try_deserialize::<HashMap<String, String>>()
        .map_err(|e| PokedexError::MalformedConfig(e.to_string()))
}

pub fn load_dex_entries(filename: &str) -> Result<HashMap<String, PokemonInfo>, PokedexError> {
    let dex_path = get_local_path()?.join(filename);
    let dex =  std::fs::read_to_string(dex_path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => PokedexError::PokedexNotFound(filename.to_string()),
        _ => PokedexError::MalformedPokedex(e.to_string())
    })?;

    serde_json::from_str(&dex)
        .map_err(|e| PokedexError::MalformedPokedex(e.to_string()))
}

pub fn get_local_path() -> Result<PathBuf, PokedexError> {
    // Get the path to the current executable
    let current_exe = env::current_exe();
    if current_exe.is_err() {
        return Err(PokedexError::FatalError("Failed to get current exe!".into()));
    }
    let exe_path: PathBuf = current_exe.unwrap();

    // Get the directory containing the executable
    let exe_parent = exe_path.parent();
    if exe_parent.is_none() {
        return Err(PokedexError::FatalError("Executable must be in a directory".into()))
    }
    let exe_dir: &std::path::Path = exe_parent.unwrap();

    Ok(exe_dir.into())
}

pub fn save_frame(frame: &VideoFrame) -> Result<PathBuf, image::ImageError> {
    // Save image to a temporary staging area while classification runs
    let path = get_local_path().map_err(|e| image::ImageError::IoError(io::Error::new(io::ErrorKind::Other, e.to_string())))?; 
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
    )?;

    Ok(out_path.to_path_buf())
}