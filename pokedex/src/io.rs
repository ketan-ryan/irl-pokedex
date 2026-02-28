use include_assets::{NamedArchive, include_dir};
use serde::Deserialize;

use std::{collections::HashMap};
use std::env;
use std::io;
use std::path::PathBuf;

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

    // Construct a path to a configuration file or data file relative to the executable's directory
    // let config_path: PathBuf = exe_dir.join("Config").join("settings.ini");

    println!("Executable path: {}", exe_path.display());
    // println!("Config path: {}", config_path.display());

    Ok(exe_path)
}
