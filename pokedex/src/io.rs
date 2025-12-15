use serde::Deserialize;

use std::{collections::HashMap, fs, path::Path};

#[derive(Deserialize)]
pub struct PokemonInfo {
    pub number: String,
    pub r#type: String,
    pub height: String,
    pub weight: String,
    pub abilities: Vec<String>,
    pub dex_entries: HashMap<String, String>
}

pub fn load_dex_entries<P: AsRef<Path>>(path: P) -> HashMap<String, PokemonInfo> {
    let json = fs::read_to_string(path).expect("Couldn't open the pokedex JSON");

    // If the JSON is unparsable, that probably indicates some upstream error with
    // its generation, and we should panic so it can be fixed.
    serde_json::from_str(&json).expect("Couldn't parse pokedex JSON")
} 
