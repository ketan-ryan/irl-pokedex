use include_assets::{NamedArchive, include_dir};
use serde::Deserialize;

use std::{collections::HashMap};

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
