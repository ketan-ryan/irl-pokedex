use include_assets::{NamedArchive, include_dir};
use serde::Deserialize;

use std::{collections::HashMap};
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{elements::gstreamer_stream::{VideoFrame}};


#[derive(Deserialize)]
pub struct PokemonInfo {
    pub number: String,
    pub r#type: String,
    pub height: String,
    pub weight: String,
    pub abilities: Vec<String>,
    pub dex_entries: HashMap<String, String>
}

pub fn load_dex_entries(filename: &str) -> HashMap<String, PokemonInfo> {
    let archive = NamedArchive::load(include_dir!("assets"));
    let pokedex_asset = archive.get(filename).unwrap();
    let json = match str::from_utf8(pokedex_asset) {
        Ok(s) => s,
        Err(e) => panic!("Invalid utf-8 seq {}", e),
    }.to_string();

    // If the JSON is unparsable, that probably indicates some upstream error with
    // its generation, and we should panic so it can be fixed.
    serde_json::from_str(&json).expect("Couldn't parse pokedex JSON")
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

    // we want this to be the only image present
    fs::remove_dir_all(&staging_area)?;
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