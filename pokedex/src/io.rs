use anyhow::{Result, anyhow};
use config::Config;
use log::{debug, error, trace, warn};
use ort::session::Session;
use serde::{Deserialize, Deserializer, Serialize};
use strum_macros::{Display, EnumString};
use tokio;
use tokio::io::AsyncReadExt;

use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::elements::gstreamer_stream::VideoFrame;
use crate::{PokedexError, ml};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Display, EnumString)]
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

impl Default for PokemonType {
    fn default() -> Self {
        PokemonType::Unknown
    }
}

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

#[derive(Clone, Deserialize, Debug)]
pub struct PokemonInfo {
    pub number: String,
    #[serde(rename = "type", deserialize_with = "deserialize_types")]
    pub types: Vec<PokemonType>,
    pub species: String,
    pub height: String,
    pub weight: String,
    pub abilities: Vec<String>,
    pub dex_entries: HashMap<String, String>,
}

impl Default for PokemonInfo {
    fn default() -> Self {
        PokemonInfo {
            number: "0000".to_string(),
            types: vec![PokemonType::Unknown],
            species: "???".to_string(),
            height: "???".to_string(),
            weight: "???".to_string(),
            abilities: Vec::new(),
            dex_entries: HashMap::new(),
        }
    }
}

const LOCAL_DEX_NAME: &str = "local_pokedex.json";
const SAVED_IMGS_DIR: &str = "saved_images";
const STAGING_DIR: &str = "staging";

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

pub fn get_type_images(types: Vec<PokemonType>) -> Vec<String> {
    types
        .iter()
        .map(|t| format!("assets/types/{}.png", t.to_string().to_lowercase()))
        .collect()
}

pub fn validate_config() -> Result<PokedexConfig, PokedexError> {
    let config = load_settings()?;

    let filename = config.get("pokedex_location");
    if filename.is_none() {
        let mcerr = "Could not find key pokedex_location in config. Pokedex cannot be loaded.";
        return Err(PokedexError::MalformedConfig(mcerr.to_string()));
    }

    let entries = load_dex_entries(filename.unwrap())?;
    let path = config.get("sprites_location");
    if path.is_none() {
        let mcerr = "Could not find key sprites_location in config. Assets cannot be loaded.";
        return Err(PokedexError::MalformedConfig(mcerr.to_string()));
    }

    let model_path = config.get("model_location");
    if model_path.is_none() {
        let mcerr =
            "Could not find key model_location in config. Classification model cannot be loaded.";
        return Err(PokedexError::MalformedConfig(mcerr.to_string()));
    }

    let binding = get_local_path()?.join(model_path.unwrap());
    let model =
        ml::init(binding.to_str().unwrap()).map_err(|e| PokedexError::ModelError(e.to_string()))?;

    let classes_path = config.get("classes_location");
    if classes_path.is_none() {
        let mcerr =
            "Could not find key classes_location in config. Pokemon classes cannot be mapped.";
        return Err(PokedexError::MalformedConfig(mcerr.to_string()));
    }
    let classes = load_classes(classes_path.unwrap())?;

    let conf = config.get("model_confidence");
    if conf.is_none() {
        let mcerr = "Could not find key model_confidence in config, model threshold unable to be determined.";
        return Err(PokedexError::MalformedConfig(mcerr.to_string()));
    }

    let confidence: f32 = match conf.unwrap().parse() {
        Ok(num) => num,
        Err(_) => {
            let mcerr =
                "Model confidence could not be parsed! Key model_confidence must be a valid float.";
            return Err(PokedexError::MalformedConfig(mcerr.to_string()));
        }
    };

    if confidence < 0.0 || confidence > 1.0 {
        let mcerr = "Invalid model confidence! Key model_confidence must be between 0.0 and 1.0, inclusive.";
        return Err(PokedexError::MalformedConfig(mcerr.to_string()));
    }

    // we can proceed without this, some pokemon just will have issues getting dex info
    // the map resolves discrepancies between image names and pokedex.json keys
    let mut name_maps: HashMap<String, String> = HashMap::new();
    let name_loc = config.get("name_maps");
    if name_loc.is_some() {
        name_maps = load_name_maps(name_loc.unwrap());
    } else {
        warn!("No name maps found, assuming perfect match between image names and pokedex keys");
    }

    let local_dex_path = config.get("local_dex_path");
    let dex_res = read_or_create_dex(local_dex_path);
    if dex_res.is_err() {
        return Err(PokedexError::MalformedPokedex(
            "Fatal error loading local pokedex".to_string(),
        ));
    }

    let saved_images_path = config.get("saved_images_path");
    let imgs_res = read_or_create_images_dir(saved_images_path);
    if imgs_res.is_err() {
        return Err(PokedexError::FatalError(
            "Fatal error creating images dir".to_string(),
        ));
    }

    Ok(PokedexConfig {
        pokedex_json: entries,
        sprites_location: path.unwrap().to_string(),
        session: model,
        classes: classes,
        confidence: confidence,
        name_maps,
        local_dex: dex_res.unwrap(),
        saved_imgs_dir: imgs_res.unwrap(),
    })
}

pub fn load_settings() -> Result<HashMap<String, String>, PokedexError> {
    let cfg_path = get_local_path()?.join("pokedex_settings.yaml");
    Config::builder()
        .add_source(config::File::from(cfg_path))
        .build()
        .map_err(|e| match e {
            config::ConfigError::NotFound(_) => PokedexError::ConfigNotFound,
            e => PokedexError::MalformedConfig(e.to_string()),
        })?
        .try_deserialize::<HashMap<String, String>>()
        .map_err(|e| PokedexError::MalformedConfig(e.to_string()))
}

pub fn load_dex_entries(filename: &str) -> Result<HashMap<String, PokemonInfo>, PokedexError> {
    let dex_path = get_local_path()?.join(filename);
    let dex = std::fs::read_to_string(dex_path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => PokedexError::PokedexNotFound(filename.to_string()),
        _ => PokedexError::MalformedPokedex(e.to_string()),
    })?;

    serde_json::from_str(&dex)
        .map_err(|e| PokedexError::MalformedPokedex(e.to_string()))
        .map(|raw_map: HashMap<String, PokemonInfo>| {
            // lowercase pokemon name for comparisons
            raw_map
                .into_iter()
                .map(|(k, v)| (k.to_lowercase(), v))
                .collect::<HashMap<String, PokemonInfo>>()
        })
}

pub fn load_classes(filename: &str) -> Result<Vec<String>, PokedexError> {
    let classes_path = get_local_path()?.join(filename);
    let classes = std::fs::read_to_string(classes_path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => PokedexError::ClassesNotFound(filename.to_string()),
        _ => PokedexError::MalformedClasses(e.to_string()),
    })?;

    serde_json::from_str(&classes).map_err(|e| PokedexError::MalformedClasses(e.to_string()))
}

pub fn load_name_maps(filename: &str) -> HashMap<String, String> {
    // If we couldn't get a local path, an error would have been thrown by the time this gets called
    let maps_path = get_local_path().unwrap().join(filename);
    let maps = std::fs::read_to_string(maps_path);
    if maps.is_ok() {
        debug!("Got name maps file");
        let res = serde_json::from_str(&maps.unwrap());
        if res.is_ok() {
            trace!("Successfully parsed maps json");
            return res.unwrap();
        } else {
            warn!("Failed to parse name maps json - proceeding without it.");
        }
    } else {
        warn!("Failed to find name maps json - proceeding without it.");
    }

    return HashMap::new();
}

pub fn load_png(sprite_folder: String, pokemon_name: &str) -> Result<Vec<u8>, anyhow::Error> {
    let folder = get_local_path()?.join(sprite_folder).join(pokemon_name);

    let img = {
        // first collect entries
        let mut entries: Vec<_> = std::fs::read_dir(folder)?.filter_map(|e| e.ok()).collect();
        // then reverse them
        entries.sort_by_key(|e| e.file_name());
        // the last photo in a directory, alphabetically, will be the clean
        // default pose for the pokemon.
        entries.into_iter().rev().find_map(|entry| {
            let path = entry.path();
            if path.extension()?.to_str()? == "png" {
                std::fs::read(&path).ok()
            } else {
                None
            }
        })
    };
    if img.is_none() {
        return Err(anyhow!("Failed to get image for pokemon {}", pokemon_name));
    }

    Ok(img.unwrap())
}

pub fn get_local_path() -> Result<PathBuf, PokedexError> {
    // Get the path to the current executable
    let current_exe = env::current_exe();
    if current_exe.is_err() {
        return Err(PokedexError::FatalError(
            "Failed to get current exe!".into(),
        ));
    }
    let exe_path: PathBuf = current_exe.unwrap();

    // Get the directory containing the executable
    let exe_parent = exe_path.parent();
    if exe_parent.is_none() {
        return Err(PokedexError::FatalError(
            "Executable must be in a directory".into(),
        ));
    }
    let exe_dir: &std::path::Path = exe_parent.unwrap();

    Ok(exe_dir.into())
}

pub fn save_frame(frame: &VideoFrame) -> Result<(), image::ImageError> {
    // Save image to a temporary staging area while classification runs
    let path = get_local_path().map_err(|e| {
        image::ImageError::IoError(io::Error::new(io::ErrorKind::Other, e.to_string()))
    })?;
    let staging_area = path.join(STAGING_DIR);

    // remove dir could error if dir isn't present - ignore
    let _ = fs::remove_dir_all(&staging_area);
    fs::create_dir_all(&staging_area)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time should go forward")
        .as_millis();

    let name = now.to_string() + ".png";

    let out_path = &staging_area.join(name);
    debug!("Saved frame to {:?}", out_path);
    image::save_buffer_with_format(
        out_path,
        &frame.data,
        frame.width,
        frame.height,
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )?;

    Ok(())
}

pub fn read_or_create_dex(path: Option<&String>) -> Result<RefCell<Vec<String>>, anyhow::Error> {
    let out_path = if path.is_none() {
        debug!(
            "No local pokedex path in config, falling back to {}",
            LOCAL_DEX_NAME
        );
        &LOCAL_DEX_NAME.to_owned()
    } else {
        trace!("Loading local pokedex from {}", &path.unwrap());
        path.unwrap()
    };

    let path = get_local_path()?.join(out_path);

    // If there's nothing there, we'll create it
    let local_dex = match std::fs::read_to_string(&path) {
        Ok(success) => {
            trace!("Reading pokedex from: {:?}", &path);
            success
        }
        Err(e) => {
            debug!(
                "Unable to find local dex {:?}, writing to file {:?}",
                e.to_string(),
                &path
            );
            std::fs::write(path, "[]")?;
            "[]".to_string()
        }
    };

    let map: Vec<String> = serde_json::from_str(&local_dex).map_err(|e| {
        warn!("Error parsing local pokedex: {}", e.to_string());
        PokedexError::MalformedPokedex(e.to_string())
    })?;

    return Ok(RefCell::new(map));
}

pub async fn update_dex(pokemon_list: Vec<String>) -> Result<(), anyhow::Error> {
    let config = load_settings()?;
    let local_dex_path = config.get("local_dex_path");

    let out_path = if local_dex_path.is_none() {
        debug!(
            "No local pokedex path in config, falling back to {}",
            LOCAL_DEX_NAME
        );
        &LOCAL_DEX_NAME.to_owned()
    } else {
        trace!("Loading local pokedex from {}", &local_dex_path.unwrap());
        local_dex_path.unwrap()
    };

    let path = get_local_path()?.join(out_path);
    let updated_json = match serde_json::to_string_pretty(&pokemon_list) {
        Ok(json) => {
            trace!("Parsed pokedex json to {}", json);
            json
        }
        Err(e) => return Err(PokedexError::UpdateDexFailure(e.to_string()).into()),
    };

    tokio::fs::write(path, updated_json)
        .await
        .map_err(|e| e.into())
}

pub fn read_or_create_images_dir(path: Option<&String>) -> Result<String, anyhow::Error> {
    let out_path = if path.is_none() {
        debug!(
            "No saved images path in config, falling back to {}",
            SAVED_IMGS_DIR
        );
        &SAVED_IMGS_DIR.to_owned()
    } else {
        trace!("Found saved images dir: {}", &path.unwrap());
        path.unwrap()
    };

    let path = get_local_path()?.join(out_path);

    // Creates the folder and any missing parents.
    // If it already exists, it does nothing and returns Ok(()).
    fs::create_dir_all(&path)?;

    Ok(path.to_str().expect("Failed to read path").to_owned())
}

pub async fn add_dex_img(dex_path: String, pokemon_name: String) -> Result<(), anyhow::Error> {
    let imgs_path = get_local_path()?.join(STAGING_DIR);

    let dex_buf: PathBuf = dex_path.into();
    trace!("Looking for dex images in {:?}", imgs_path);

    let mut entries = tokio::fs::read_dir(&imgs_path).await?;
    let img_path: PathBuf;
    if let Some(entry) = entries.next_entry().await? {
        img_path = entry.path();
        trace!("Found path to be {:?}", img_path);
    } else {
        return Err(PokedexError::SaveDexImgError(
            "Staging dir is empty! No image to save.".to_string(),
        )
        .into());
    }

    let save_dir = dex_buf.join(pokemon_name);
    tokio::fs::create_dir_all(&save_dir).await?;
    trace!("Saving image to {:?}", save_dir);

    let dest = save_dir.join(&img_path.file_name().unwrap());
    tokio::fs::copy(imgs_path.join(&img_path), &dest).await?;

    debug!("Moved image to {:?}", dest);
    Ok(())
}

pub async fn get_dex_images(
    dex_path: &PathBuf,
    pokemon_name: String,
) -> Result<Vec<Vec<u8>>, anyhow::Error> {
    let imgs_dir = dex_path.join(pokemon_name);
    let mut read_dir = tokio::fs::read_dir(imgs_dir).await?;
    let mut entries = Vec::new();

    while let Some(entry) = read_dir.next_entry().await? {
        let path = entry.path();
        let mut current_entry = Vec::new();

        trace!("Found file {:?}", &path);

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("png") {
            let mut file = tokio::fs::File::open(&path).await?;
            file.read_to_end(&mut current_entry).await?;
        } else {
            debug!("Skipping file {:?}", &path);
        }

        entries.push(current_entry);
    }

    Ok(entries)
}
