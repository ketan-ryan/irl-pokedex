import json
import re
import os
import requests
import time
from urllib.parse import urlparse

# Load master JSON
with open("pokemon_master.json") as f:
    master_json = json.load(f)

# Base folder where all Pokémon folders will be created
BASE_DIR = r"G:\My Drive\IRL Pokedex\home models"
os.makedirs(BASE_DIR, exist_ok=True)

# Time delay between requests (in seconds)
REQUEST_DELAY = 0.5

def safe_name(name: str) -> str:
    return re.sub(r'[<>:"/\\|?*]', '', name).strip()

def download_image(url, save_path):
    """Download an image from URL to save_path if it doesn't exist"""
    if url is None:
        return
    if os.path.exists(save_path):
        # Skip if already downloaded
        return
    try:
        response = requests.get(url, timeout=30)
        response.raise_for_status()
        with open(save_path, "wb") as f:
            f.write(response.content)
        time.sleep(REQUEST_DELAY)  # polite delay
    except Exception as e:
        with open('failed.txt', 'a') as f:
            f.write(f"\nFailed to download {url}: {e}")
        print(f"Failed to download {url}: {e}")

def traverse_and_download(d, folder_path):
    """Recursively traverse the JSON and download all image URLs"""
    for key, value in d.items():
        if isinstance(value, dict):
            traverse_and_download(value, folder_path)
        elif isinstance(value, str) and value.startswith("http"):
            # Extract filename from URL
            filename = os.path.basename(urlparse(value).path)
            save_path = os.path.join(folder_path, filename)
            download_image(value, save_path)

# Loop through all Pokémon
for dex, data in master_json.items():
    pokemon_name = data["name"]
    folder_name = safe_name(pokemon_name)
    folder_path = os.path.join(BASE_DIR, folder_name)
    os.makedirs(folder_path, exist_ok=True)

    # Skip the 'name' key and traverse the rest
    traverse_and_download({k: v for k, v in data.items() if k != "name"}, folder_path)

print("All images downloaded!")
