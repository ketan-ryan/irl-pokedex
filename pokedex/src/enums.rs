use std::{
    cell::RefCell,
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
};

use iced::Color;
use ort::session::Session;
use serde::{Deserialize, Deserializer, Serialize};
use strum_macros::{Display, EnumString};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
pub enum Region {
    Kanto,
    Johto,
    Hoenn,
    Sinnoh,
    Unova,
    Kalos,
    Alola,
    Galar,
    Hisui,
    Paldea,
    Undiscovered,
}

impl Region {
    pub const ALL: [Region; 10] = [
        Region::Kanto,
        Region::Johto,
        Region::Hoenn,
        Region::Sinnoh,
        Region::Unova,
        Region::Kalos,
        Region::Alola,
        Region::Galar,
        Region::Hisui,
        Region::Paldea,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Region::Kanto => "Kanto",
            Region::Johto => "Johto",
            Region::Hoenn => "Hoenn",
            Region::Sinnoh => "Sinnoh",
            Region::Unova => "Unova",
            Region::Kalos => "Kalos",
            Region::Alola => "Alola",
            Region::Galar => "Galar",
            Region::Paldea => "Paldea",
            Region::Hisui => "Hisui",
            Region::Undiscovered => "Undiscovered",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Display, EnumString, Eq, Hash, PartialEq, Serialize)]
#[strum(serialize_all = "lowercase")]
pub enum PokemonType {
    Normal,
    Fire,
    Water,
    Grass,
    Electric,
    Ice,
    Fighting,
    Poison,
    Ground,
    Flying,
    Psychic,
    Bug,
    Rock,
    Ghost,
    Dragon,
    Dark,
    Steel,
    Fairy,
    Unknown,
}

impl PokemonType {
    pub const ALL: [PokemonType; 18] = [
        PokemonType::Normal,
        PokemonType::Fire,
        PokemonType::Water,
        PokemonType::Grass,
        PokemonType::Electric,
        PokemonType::Ice,
        PokemonType::Fighting,
        PokemonType::Poison,
        PokemonType::Ground,
        PokemonType::Flying,
        PokemonType::Psychic,
        PokemonType::Bug,
        PokemonType::Rock,
        PokemonType::Ghost,
        PokemonType::Dragon,
        PokemonType::Dark,
        PokemonType::Steel,
        PokemonType::Fairy,
    ];

    fn slug(&self) -> &'static str {
        match self {
            PokemonType::Electric => "electric",
            PokemonType::Fire => "fire",
            PokemonType::Water => "water",
            PokemonType::Grass => "grass",
            PokemonType::Flying => "flying",
            PokemonType::Ground => "ground",
            PokemonType::Bug => "bug",
            PokemonType::Fairy => "fairy",
            PokemonType::Dragon => "dragon",
            PokemonType::Ghost => "ghost",
            PokemonType::Dark => "dark",
            PokemonType::Psychic => "psychic",
            PokemonType::Steel => "steel",
            PokemonType::Ice => "ice",
            PokemonType::Fighting => "fighting",
            PokemonType::Poison => "poison",
            PokemonType::Rock => "rock",
            PokemonType::Normal => "normal",
            PokemonType::Unknown => "unknown",
        }
    }

    pub fn accent_color(&self) -> Color {
        match self {
            PokemonType::Bug => Color::from_str("#9DFF00").unwrap(),
            PokemonType::Dark => Color::from_str("#464646").unwrap(),
            PokemonType::Dragon => Color::from_str("#351AAC").unwrap(),
            PokemonType::Electric => Color::from_str("#FFEA00").unwrap(),
            PokemonType::Fairy => Color::from_str("#FF7CCF").unwrap(),
            PokemonType::Fighting => Color::from_str("#FFC400").unwrap(),
            PokemonType::Fire => Color::from_str("#FF0000").unwrap(),
            PokemonType::Flying => Color::from_str("#4CBAFF").unwrap(),
            PokemonType::Ghost => Color::from_str("#8C4AFF").unwrap(),
            PokemonType::Grass => Color::from_str("#12DE00").unwrap(),
            PokemonType::Ground => Color::from_str("#D07A00").unwrap(),
            PokemonType::Ice => Color::from_str("#00FFFF").unwrap(),
            PokemonType::Normal => Color::from_str("#DEDEDE").unwrap(),
            PokemonType::Poison => Color::from_str("#DE00AE").unwrap(),
            PokemonType::Psychic => Color::from_str("#C640FB").unwrap(),
            PokemonType::Rock => Color::from_str("#653600").unwrap(),
            PokemonType::Steel => Color::from_str("#BABABA").unwrap(),
            PokemonType::Unknown => Color::from_str("#00DEB9").unwrap(),
            PokemonType::Water => Color::from_str("#006FFF").unwrap(),
        }
    }

    /// Location of the badge graphic. Point this at wherever your type
    /// icon assets actually live -- this is a guessed convention.
    pub fn asset_path(&self) -> String {
        let path: String = format!("assets/types/svgs/{}.svg", self.slug());
        path
    }

    pub fn overlay_path(&self) -> String {
        "assets/types/svgs/overlay.svg".to_string()
    }
}

impl Default for PokemonType {
    fn default() -> Self {
        PokemonType::Unknown
    }
}

/// Deserialize a slash-delimited Pokémon type string into a vector of enum values.
/// Ex: "steel/fairy" becomes [steel, fairy]
/// Args:
/// - deserializer: The serde deserializer for the incoming type string.
///
/// Returns: A list of parsed Pokémon types.
fn deserialize_types<'de, D>(deserializer: D) -> Result<Vec<PokemonType>, D::Error>
where
    D: Deserializer<'de>,
{
    // 1. Get the raw string from JSON (e.g., "steel/fairy")
    let s: String = Deserialize::deserialize(deserializer)?;

    // 2. Split, parse, and collect
    s.split('/')
        .map(|part| {
            // This now uses strum's generated FromStr
            part.trim()
                .to_lowercase()
                .parse::<PokemonType>()
                .map_err(|_| serde::de::Error::custom(format!("Unknown type: {}", part)))
        })
        .collect::<Result<Vec<PokemonType>, D::Error>>()
}

#[derive(Debug, Clone, PartialEq)]
pub struct Measurement {
    /// Original string as it appears in the JSON, e.g. "0.7 m (2′04″)"
    pub raw: String,
    /// Leading metric value, e.g. 0.7 — meters for height, kg for weight
    pub metric: f32,
}

impl<'de> Deserialize<'de> for Measurement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        let metric = raw
            .split_whitespace()
            .next()
            .and_then(|first| first.parse::<f32>().ok())
            .unwrap_or(0.0);
        Ok(Measurement { raw, metric })
    }
}

#[derive(Clone, Deserialize, Debug)]
pub struct PokemonInfo {
    pub number: String,
    #[serde(rename = "type", deserialize_with = "deserialize_types")]
    pub types: Vec<PokemonType>,
    pub species: String,
    pub height: Measurement,
    pub weight: Measurement,
    pub abilities: Vec<String>,
    pub dex_entries: HashMap<String, String>,
    pub region: Option<Region>,
    pub base: Option<bool>,
    pub display_name: Option<String>,
}

impl Default for PokemonInfo {
    fn default() -> Self {
        PokemonInfo {
            number: "0000".to_string(),
            types: vec![PokemonType::Unknown],
            species: "???".to_string(),
            height: Measurement {
                raw: "???".to_string(),
                metric: -999.99,
            },
            weight: Measurement {
                raw: "???".to_string(),
                metric: -999.99,
            },
            abilities: Vec::new(),
            dex_entries: HashMap::new(),
            region: Some(Region::Undiscovered),
            base: None,
            display_name: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Alphabetical,
    Numerical,
}

impl SortKey {
    pub fn label(&self) -> &'static str {
        match self {
            SortKey::Alphabetical => "Alphabetical",
            SortKey::Numerical => "Numerical",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            SortKey::Alphabetical => SortKey::Numerical,
            SortKey::Numerical => SortKey::Alphabetical,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    pub fn toggled(self) -> Self {
        match self {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            SortDirection::Descending => "▼",
            SortDirection::Ascending => "▲",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    All,
    Any,
}

impl FilterMode {
    pub fn label(&self) -> &'static str {
        match self {
            FilterMode::All => "All",
            FilterMode::Any => "Any",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            FilterMode::All => FilterMode::Any,
            FilterMode::Any => FilterMode::All,
        }
    }
}

#[derive(Debug)]
pub struct PokedexConfig {
    pub pokedex_json: HashMap<String, PokemonInfo>,
    pub sprites_location: String,
    pub session: Arc<Mutex<Session>>,
    pub classes: Vec<String>,
    pub confidence: f32,
    pub name_maps: HashMap<String, String>,
    pub local_dex: RefCell<Vec<String>>,
    pub saved_imgs_dir: String,
}
