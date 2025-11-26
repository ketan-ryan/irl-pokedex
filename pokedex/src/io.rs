use serde_json::Result;
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

pub fn load_dex_entries<P: AsRef<Path>>(path: P) -> Result<HashMap<String, PokemonInfo>> {
    let json = fs::read_to_string(path).unwrap();
    let pokedex: HashMap<String, PokemonInfo> = serde_json::from_str(&json)?;
    Ok(pokedex)
} 
