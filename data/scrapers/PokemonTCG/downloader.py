import json
import os
from pathlib import Path
import requests
from io import BytesIO
from PIL import Image

# CONFIG
JSON_PATH = "final_tcg_list.json"
OUTPUT_DIR = Path(r"G:\My Drive\IRL Pokedex\TCG imgs\crops")   # root directory
LOG_FILE = "new_images_to_add.txt"
FAILED_LOG = "failed_downloads.txt"
DRY_RUN = True  # SET TO FALSE WHEN DOWNLOADING 
existing_paths = set()


# LOAD JSON
with open(JSON_PATH, "r", encoding="utf-8") as f:
    all_metadata = json.load(f)

OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

if os.path.exists(LOG_FILE):
    with open(LOG_FILE, "r", encoding="utf-8") as f:
        existing_paths = {line.strip() for line in f if line.strip()}

new_images = []

BAD_NAME_MAP = {
    "nidoran♂": "nidoran-m",
    "nidoran♀": "nidoran-f",
    "flabébé": "flabebe",
    "type: null": "type null",
    "Nidoran♂": "Nidoran-m",
    "Nidoran♀": "Nidoran-f",
    "Nidoran ♂": "Nidoran-m",
    "Nidoran ♀": "Nidoran-f",
    "Flabébé": "Flabebe",
    "Type: Null": "Type Null",
}

def safe_name(name: str) -> str:
    for bad, safe in BAD_NAME_MAP.items():
        name = name.replace(bad, safe)
    return name

def log_failed_url(url: str):
    with open(FAILED_LOG, "a", encoding="utf-8") as f:
        f.write(url + "\n")


def remove_failed_url(url: str):
    if not os.path.exists(FAILED_LOG):
        return

    with open(FAILED_LOG, "r", encoding="utf-8") as f:
        lines = f.readlines()

    with open(FAILED_LOG, "w", encoding="utf-8") as f:
        for line in lines:
            if line.strip() != url:
                f.write(line)

# MAIN LOOP
for pokemon, images in all_metadata.items():
    safe_pokemon = safe_name(pokemon).lower()
    pokemon_dir = OUTPUT_DIR / safe_pokemon
    pokemon_dir.mkdir(exist_ok=True)

    for image_name, url in images.items():
        safe_image_name = safe_name(image_name)
        # safe_image_name = safe_image_name.replace("?", "_")

        filename = f"{safe_image_name}.png"
        image_path = pokemon_dir / filename

        # If image already exists, skip
        if image_path.exists():
            continue

        # Log missing image
        new_images.append(str(image_path))

        # Only download if not in dry-run mode
        if not DRY_RUN:
            try:
                response = requests.get(url, timeout=10)
                response.raise_for_status()

                img = Image.open(BytesIO(response.content))
                cropped = img.crop((26, 35 + 5, 219 - 5, 160))

                cropped.save(image_path)

                remove_failed_url(url)

            except Exception as e:
                print(f"Failed to download {url}: {e}")
                log_failed_url(url)

# WRITE LOG FILE
    with open(LOG_FILE, "a", encoding="utf-8") as f:
        for path in new_images:
            if path not in existing_paths:
                f.write(path + "\n")

print(f"New images found: {len(new_images)}")
print(f"Logged to: {LOG_FILE}")