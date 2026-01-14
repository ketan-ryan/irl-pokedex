import os
import json
import requests
from pathlib import Path
from urllib.parse import urlparse

# Paths to your JSON files
IMAGE_JSON_PATH = "pokemon_plush_images.json"
# IMAGE_JSON_PATH = "rotom_plushdex_cleaned.json"   # UNCOMMENT THIS
ALL_POKEMON_JSON_PATH = "all_pokemon.json"  # Full list of 1025 Pokemon

# Where to save the downloaded images
OUTPUT_DIR = r"G:\My Drive\IRL Pokedex\rotomplushdex"
Path(OUTPUT_DIR).mkdir(parents=True, exist_ok=True)

# Load JSON files
with open(IMAGE_JSON_PATH, "r", encoding="utf-8") as f:
    image_urls = json.load(f)

with open(ALL_POKEMON_JSON_PATH, "r", encoding="utf-8") as f:
    all_pokemon = json.load(f)

# Helper function to sanitize folder names
def sanitize_name(name):
    name = name.lower()  # ensure lowercase
    if name == "nidoran♀":
        return "nidoran-f"
    elif name == "nidoran♂":
        return "nidoran-m"
    return name

# Helper function to download an image
def download_image(url, save_path):
    try:
        response = requests.get(url, timeout=15)
        response.raise_for_status()
        with open(save_path, "wb") as f:
            f.write(response.content)
        print(f"Downloaded: {save_path}")
    except Exception as e:
        print(f"Failed to download {url}: {e}")

# Build a mapping from Pokemon name to Pokédex number
pokedex_mapping = {}
for idx, name in enumerate(all_pokemon, start=1):
    sanitized = sanitize_name(name)
    pokedex_number = str(idx).zfill(4)
    pokedex_mapping[sanitized] = pokedex_number

# Loop through image URLs JSON
for pokemon_name, urls in image_urls.items():
    pokemon_name_lower = pokemon_name.lower()
    
    if pokemon_name_lower == "hqfitpckalos" or pokemon_name_lower == "misc":
        folder_name = os.path.join(OUTPUT_DIR, "misc")
    else:
        sanitized_name = sanitize_name(pokemon_name_lower)
        if sanitized_name not in pokedex_mapping:
            print(f"Skipping unknown Pokemon: {pokemon_name}")
            continue
        folder_name = os.path.join(
            OUTPUT_DIR, f"{pokedex_mapping[sanitized_name]}{sanitized_name}"
        )

    Path(folder_name).mkdir(parents=True, exist_ok=True)

    for url in urls:
        # Extract filename without query parameters
        filename = os.path.basename(urlparse(url).path)
        save_path = os.path.join(folder_name, filename)
        if not os.path.exists(save_path):  # Skip if already downloaded
            download_image(url, save_path)