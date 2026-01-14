import os
import json
import requests
from urllib.parse import urlparse

# CONFIG
json_files = ["pokemon_plush_images_seq_candidates.json", "rotom_plushdex_cleaned_seq_candidates.json"]
base_folder = r"G:\My Drive\IRL Pokedex\rotomplushdex"           # where all pokemon folders reside
failed_urls_file = "failed_urls.txt"
misc_folder_name = "misc"                  # folder for miscellaneous pokemon

# Ensure base folder exists
os.makedirs(base_folder, exist_ok=True)

# Load failed URLs
if os.path.exists(failed_urls_file):
    with open(failed_urls_file, "r") as f:
        failed_urls = set(line.strip() for line in f)
else:
    failed_urls = set()

# Load JSON files
pokemon_data = {}
for jf in json_files:
    with open(jf, "r", encoding="utf-8") as f:
        data = json.load(f)
        pokemon_data.update(data)

# Helper function to sanitize folder names
def sanitize_name(name):
    name = name.lower()  # ensure lowercase
    if name == "nidoran♀":
        return "nidoran-f"
    elif name == "nidoran♂":
        return "nidoran-m"
    return name

# Function to download image
def download_image(url, save_path):
    try:
        r = requests.get(url, timeout=10)
        r.raise_for_status()
        with open(save_path, "wb") as f:
            f.write(r.content)
        return True
    except Exception:
        return False

# Function to log failed URL immediately
def log_failed_url(url):
    if url not in failed_urls:
        failed_urls.add(url)
        with open(failed_urls_file, "a") as f:
            f.write(url + "\n")
        print(f"Logged failed URL: {url}")

# Process each Pokemon
for pokemon, urls in pokemon_data.items():
    pokemon_safe_name = sanitize_name(pokemon.lower())
    # Determine folder name
    if pokemon.lower() == "misc":
        folder_path = os.path.join(base_folder, misc_folder_name)
    else:
        # Look for a folder that ends with the pokemon name
        matching_folders = [f for f in os.listdir(base_folder)
                            if f.lower().endswith(pokemon_safe_name) and os.path.isdir(os.path.join(base_folder, f))]
        if matching_folders:
            folder_path = os.path.join(base_folder, matching_folders[0])
        else:
            # Create folder if not found
            folder_path = os.path.join(base_folder, f"0000{pokemon_safe_name}")
            os.makedirs(folder_path, exist_ok=True)
            print(f"Created folder for {pokemon}: {folder_path}")

    os.makedirs(folder_path, exist_ok=True)

    # Process each URL
    for url in urls:
        if url in failed_urls:
            continue  # skip known failed URLs

        # Determine filename from URL
        filename = os.path.basename(urlparse(url).path)
        save_path = os.path.join(folder_path, filename)

        if os.path.exists(save_path):
            print(f"Already exists: {save_path}")
            continue  # skip if file already exists

        # Try downloading original URL
        success = download_image(url, save_path)
        if success:
            print(f"Downloaded: {save_path}")


        # If failed, try switching jpg -> png
        if not success and url.lower().endswith(".jpg"):
            png_url = url[:-4] + ".png"
            save_path = os.path.join(folder_path, os.path.basename(png_url))
            success = download_image(png_url, save_path)
            if success:
                print(f"Downloaded: {save_path}")
            if not success:
                log_failed_url(url)
        elif not success:
            log_failed_url(url)

print("Done!")